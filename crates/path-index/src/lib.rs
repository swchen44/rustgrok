// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
//! Single-process, single-project path postings over a supplied manifest.
//!
//! This module does not select files from source, store source content, rank
//! results, implement the OpenGrok query parser or persist generations.

mod query;
pub use query::{parse_path_expression, PathClause, PathOccur, PathQuery};

use rustgrok_analysis::analyze_path;
use std::{
    collections::{HashMap, HashSet},
    io,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex, RwLock, TryLockError, Weak,
    },
};

#[derive(Debug)]
pub enum IndexError {
    InvalidProject,
    InvalidLimits,
    LimitExceeded,
    InvalidPath(String),
    UnsupportedPath {
        path: String,
        byte_offset: usize,
    },
    DuplicatePath(String),
    Source(io::Error),
    CountMismatch {
        expected: usize,
        actual: usize,
    },
    Cancelled,
    CancellationTokenUsed,
    WriterBusy,
    StaleGeneration {
        expected: Option<u64>,
        actual: Option<u64>,
    },
    NoPublishedGeneration,
    UnsupportedQuery,
    GenerationOverflow,
    LockPoisoned,
}

struct Generation {
    id: u64,
    max_path_bytes: usize,
    manifest: HashSet<String>,
    postings: HashMap<String, HashSet<String>>,
}

/// Per-generation limits chosen by the caller; this crate has no default SLO.
#[derive(Clone, Copy, Debug)]
pub struct IndexLimits {
    pub max_documents: usize,
    pub max_path_bytes: usize,
    pub max_total_path_bytes: usize,
    pub max_live_generations: usize,
}

/// A cooperative cancellation token for one publication attempt.
/// `cancel` returns false once publication has atomically begun.
#[derive(Default)]
pub struct CancellationToken {
    state: AtomicU8,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) -> bool {
        match self
            .state
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) | Err(1) => true,
            Err(_) => false,
        }
    }

    fn check(&self) -> Result<(), IndexError> {
        match self.state.load(Ordering::Acquire) {
            0 => Ok(()),
            1 => Err(IndexError::Cancelled),
            _ => Err(IndexError::CancellationTokenUsed),
        }
    }

    fn begin_publication(&self) -> Result<(), IndexError> {
        match self
            .state
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Ok(()),
            Err(1) => Err(IndexError::Cancelled),
            Err(_) => Err(IndexError::CancellationTokenUsed),
        }
    }
}

/// One logical project; a complete manifest is supplied by a separate caller.
pub struct PathIndex {
    project: String,
    limits: IndexLimits,
    writer: Mutex<()>,
    current: RwLock<Option<Arc<Generation>>>,
    live: Mutex<Vec<Weak<Generation>>>,
}

/// A stable view of a single in-memory published generation.
pub struct PathSnapshot {
    inner: Arc<Generation>,
}

fn valid_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && !component.contains(['\\', '\0'])
}

impl PathIndex {
    pub fn new(project: &str, limits: IndexLimits) -> Result<Self, IndexError> {
        if limits.max_documents == 0
            || limits.max_path_bytes == 0
            || limits.max_total_path_bytes == 0
            || limits.max_live_generations < 2
        {
            return Err(IndexError::InvalidLimits);
        }
        if project.len() > limits.max_path_bytes {
            return Err(IndexError::LimitExceeded);
        }
        if !valid_component(project) || project.contains('/') || analyze_path(project).is_err() {
            return Err(IndexError::InvalidProject);
        }
        Ok(Self {
            project: project.into(),
            limits,
            writer: Mutex::new(()),
            current: RwLock::new(None),
            live: Mutex::new(Vec::new()),
        })
    }

    /// Replace the entire in-memory generation from an explicit logical path
    /// manifest. The caller is responsible for source selection/completeness.
    /// On error the previous generation stays published. Cancellation is
    /// cooperative and is checked between iterator calls. Cancellation and
    /// beginning publication have a single atomic winner.
    pub fn replace<I>(
        &self,
        expected_generation: Option<u64>,
        expected_count: usize,
        paths: I,
        cancellation: &CancellationToken,
    ) -> Result<u64, IndexError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        let _writer = self.writer.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => IndexError::WriterBusy,
            TryLockError::Poisoned(_) => IndexError::LockPoisoned,
        })?;
        let actual = self
            .current
            .read()
            .map_err(|_| IndexError::LockPoisoned)?
            .as_ref()
            .map(|generation| generation.id);
        if actual != expected_generation {
            return Err(IndexError::StaleGeneration {
                expected: expected_generation,
                actual,
            });
        }
        if expected_count > self.limits.max_documents {
            return Err(IndexError::LimitExceeded);
        }
        {
            let mut live = self.live.lock().map_err(|_| IndexError::LockPoisoned)?;
            live.retain(|generation| generation.strong_count() > 0);
            if live.len() >= self.limits.max_live_generations {
                return Err(IndexError::LimitExceeded);
            }
        }
        let mut postings: HashMap<String, HashSet<String>> = HashMap::new();
        let mut seen = HashSet::new();
        let mut total_path_bytes = 0usize;
        let mut iterator = paths.into_iter();
        loop {
            cancellation.check()?;
            let Some(item) = iterator.next() else { break };
            let path = item.map_err(IndexError::Source)?;
            if path.len() > self.limits.max_path_bytes {
                return Err(IndexError::LimitExceeded);
            }
            total_path_bytes = total_path_bytes
                .checked_add(path.len())
                .ok_or(IndexError::LimitExceeded)?;
            if total_path_bytes > self.limits.max_total_path_bytes {
                return Err(IndexError::LimitExceeded);
            }
            let mut components = path.split('/');
            if components.next() != Some("") || components.next() != Some(self.project.as_str()) {
                return Err(IndexError::InvalidPath(path));
            }
            let rest: Vec<_> = components.collect();
            if rest.is_empty() || rest.iter().any(|component| !valid_component(component)) {
                return Err(IndexError::InvalidPath(path));
            }
            if !seen.insert(path.clone()) {
                return Err(IndexError::DuplicatePath(path));
            }
            if seen.len() > expected_count {
                return Err(IndexError::CountMismatch {
                    expected: expected_count,
                    actual: seen.len(),
                });
            }
            let tokens = analyze_path(&path).map_err(|error| IndexError::UnsupportedPath {
                path: path.clone(),
                byte_offset: error.byte_offset,
            })?;
            for token in tokens {
                postings.entry(token.term).or_default().insert(path.clone());
            }
        }
        if seen.len() != expected_count {
            return Err(IndexError::CountMismatch {
                expected: expected_count,
                actual: seen.len(),
            });
        }
        cancellation.check()?;
        let mut current = self.current.write().map_err(|_| IndexError::LockPoisoned)?;
        let id = current
            .as_ref()
            .map_or(Some(1), |generation| generation.id.checked_add(1))
            .ok_or(IndexError::GenerationOverflow)?;
        let next = Arc::new(Generation {
            id,
            max_path_bytes: self.limits.max_path_bytes,
            manifest: seen,
            postings,
        });
        let mut live = self.live.lock().map_err(|_| IndexError::LockPoisoned)?;
        cancellation.begin_publication()?;
        live.push(Arc::downgrade(&next));
        *current = Some(next);
        Ok(id)
    }

    pub fn snapshot(&self) -> Result<PathSnapshot, IndexError> {
        let current = self.current.read().map_err(|_| IndexError::LockPoisoned)?;
        let inner = current.as_ref().ok_or(IndexError::NoPublishedGeneration)?;
        Ok(PathSnapshot {
            inner: Arc::clone(inner),
        })
    }
}

impl PathSnapshot {
    pub fn generation(&self) -> u64 {
        self.inner.id
    }

    /// The exact paths supplied by the caller for this generation.
    pub fn manifest(&self) -> HashSet<String> {
        self.inner.manifest.clone()
    }

    /// Return complete, unranked candidates for a bounded path expression.
    pub fn lookup_expression(&self, query: &str) -> Result<HashSet<String>, IndexError> {
        fn evaluate(
            snapshot: &PathSnapshot,
            expr: PathQuery,
        ) -> Result<HashSet<String>, IndexError> {
            match expr {
                PathQuery::Term(term) => snapshot.lookup(&term),
                PathQuery::Boolean(clauses) => {
                    let mut must: Option<HashSet<String>> = None;
                    let mut should = HashSet::new();
                    let mut excluded = HashSet::new();
                    let mut has_should = false;
                    for clause in clauses {
                        let paths = evaluate(snapshot, clause.query)?;
                        match clause.occur {
                            PathOccur::Must => match &mut must {
                                Some(current) => current.retain(|p| paths.contains(p)),
                                None => must = Some(paths),
                            },
                            PathOccur::Should => {
                                has_should = true;
                                should.extend(paths);
                            }
                            PathOccur::MustNot => excluded.extend(paths),
                        }
                    }
                    let mut result =
                        must.unwrap_or_else(|| if has_should { should } else { HashSet::new() });
                    result.retain(|p| !excluded.contains(p));
                    Ok(result)
                }
            }
        }
        evaluate(
            self,
            parse_path_expression(query, self.inner.max_path_bytes)?,
        )
    }

    /// Return unranked candidates for one analyzed path term.
    pub fn lookup(&self, query: &str) -> Result<HashSet<String>, IndexError> {
        if query.len() > self.inner.max_path_bytes {
            return Err(IndexError::LimitExceeded);
        }
        if query.is_empty()
            || matches!(query, "AND" | "OR" | "NOT")
            || !query
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '🦀' | '😀'))
        {
            return Err(IndexError::UnsupportedQuery);
        }
        let tokens = analyze_path(query).map_err(|_| IndexError::UnsupportedQuery)?;
        if tokens.len() != 1 {
            return Err(IndexError::UnsupportedQuery);
        }
        Ok(self
            .inner
            .postings
            .get(&tokens[0].term)
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(target_os = "macos")]
pub mod durable;
