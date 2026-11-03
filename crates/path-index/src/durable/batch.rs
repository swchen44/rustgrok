// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
//! Fixed-registry, explicit-manifest atomic path batch on macOS.
//!
//! The caller owns source selection and supplies the complete manifest for every
//! project on every replacement. This is not source scanning or complete Q8.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::TryLockError,
};

pub(super) const BATCH_MARKER: &[u8] = b"RGBATCH1";
const VIRTUAL_PROJECT: &str = "rg_batch";
const PREFIX: &str = "/rg_batch";
const HARD_PROJECTS: usize = 64;
const HARD_DOCUMENTS: usize = 50_000;
const HARD_BYTES: usize = 16 * 1024 * 1024;
const HARD_PATH: usize = 4096;

#[derive(Clone, Copy, Debug)]
pub struct BatchLimits {
    pub per_project: IndexLimits,
    pub max_projects: usize,
    pub max_batch_documents: usize,
    pub max_batch_path_bytes: usize,
}

impl BatchLimits {
    fn storage_limits(self) -> Result<IndexLimits, DurableError> {
        let p = self.per_project;
        if self.max_projects == 0
            || self.max_projects > HARD_PROJECTS
            || self.max_batch_documents == 0
            || self.max_batch_documents > HARD_DOCUMENTS
            || self.max_batch_path_bytes == 0
            || self.max_batch_path_bytes > HARD_BYTES
            || p.max_documents == 0
            || p.max_documents > HARD_DOCUMENTS
            || p.max_total_path_bytes == 0
            || p.max_total_path_bytes > HARD_BYTES
            || p.max_path_bytes == 0
            || p.max_path_bytes > HARD_PATH
            || !(2..=16).contains(&p.max_live_generations)
        {
            return Err(DurableError::LimitExceeded);
        }
        let count = self
            .max_batch_documents
            .checked_add(self.max_projects)
            .ok_or(DurableError::LimitExceeded)?;
        let path = p
            .max_path_bytes
            .checked_add(PREFIX.len())
            .and_then(|value| value.checked_add(1))
            .ok_or(DurableError::LimitExceeded)?;
        let overhead = PREFIX
            .len()
            .checked_mul(self.max_batch_documents)
            .and_then(|x| {
                self.max_projects
                    .checked_mul(path)
                    .and_then(|y| x.checked_add(y))
            })
            .ok_or(DurableError::LimitExceeded)?;
        let bytes = self
            .max_batch_path_bytes
            .checked_add(overhead)
            .ok_or(DurableError::LimitExceeded)?;
        Ok(IndexLimits {
            max_documents: count,
            max_path_bytes: path,
            max_total_path_bytes: bytes,
            max_live_generations: p.max_live_generations,
        })
    }
}

/// Complete manifest of one project in a fixed-registry batch.
pub struct BatchProject<I> {
    pub project: String,
    pub expected_count: usize,
    pub paths: I,
}

/// One marked batch store owns one durable publication pointer and cross-process lock.
pub struct DurableBatchIndex {
    inner: DurablePathIndex,
    limits: BatchLimits,
    writer: Mutex<()>,
    scratch: Arc<Mutex<()>>,
}

/// A retained view of the same published batch for every selected project.
pub struct BatchSnapshot {
    inner: PathSnapshot,
    projects: BTreeSet<String>,
    limits: BatchLimits,
    scratch: Arc<Mutex<()>>,
}

fn acquire(lock: &Mutex<()>) -> Result<std::sync::MutexGuard<'_, ()>, DurableError> {
    lock.try_lock().map_err(|e| match e {
        TryLockError::WouldBlock => DurableError::Busy,
        TryLockError::Poisoned(_) => DurableError::Index(IndexError::LockPoisoned),
    })
}

fn registry(
    snapshot: &PathSnapshot,
    limits: BatchLimits,
) -> Result<BTreeSet<String>, DurableError> {
    let prefix = format!("{PREFIX}/");
    let mut names = BTreeSet::new();
    for path in &snapshot.inner.manifest {
        let rest = path.strip_prefix(&prefix).ok_or(DurableError::Corrupt)?;
        if !rest.contains('/') {
            PathIndex::new(rest, limits.per_project).map_err(|_| DurableError::Corrupt)?;
            if !names.insert(rest.to_owned()) {
                return Err(DurableError::Corrupt);
            }
        }
    }
    if names.is_empty() || names.len() > limits.max_projects {
        return Err(DurableError::Corrupt);
    }
    let mut manifests: BTreeMap<String, Vec<io::Result<String>>> = names
        .iter()
        .map(|name| (name.clone(), Vec::new()))
        .collect();
    let mut total_documents = 0usize;
    let mut total_bytes = 0usize;
    for path in &snapshot.inner.manifest {
        let rest = path.strip_prefix(&prefix).ok_or(DurableError::Corrupt)?;
        if let Some((project, tail)) = rest.split_once('/') {
            let manifest = manifests.get_mut(project).ok_or(DurableError::Corrupt)?;
            let original = format!("/{project}/{tail}");
            total_documents = total_documents
                .checked_add(1)
                .ok_or(DurableError::Corrupt)?;
            total_bytes = total_bytes
                .checked_add(original.len())
                .ok_or(DurableError::Corrupt)?;
            if total_documents > limits.max_batch_documents
                || total_bytes > limits.max_batch_path_bytes
                || manifest.len() >= limits.per_project.max_documents
            {
                return Err(DurableError::Corrupt);
            }
            manifest.push(Ok(original));
        }
    }
    for (project, manifest) in manifests {
        build(
            &project,
            limits.per_project,
            manifest.len(),
            manifest,
            snapshot.generation(),
            &CancellationToken::new(),
        )
        .map_err(|_| DurableError::Corrupt)?;
    }
    Ok(names)
}

impl DurableBatchIndex {
    pub fn create(
        parent: &Path,
        name: &str,
        source: &Path,
        limits: BatchLimits,
        max_generation_id: u64,
    ) -> Result<Self, DurableError> {
        let storage = limits.storage_limits()?;
        let inner = DurablePathIndex::create(
            parent,
            name,
            source,
            VIRTUAL_PROJECT,
            storage,
            max_generation_id,
        )?;
        write_sync(create_at(&inner.root, "BATCH-MARKER")?, BATCH_MARKER)?;
        inner.root.sync_all()?;
        Ok(Self {
            inner,
            limits,
            writer: Mutex::new(()),
            scratch: Arc::new(Mutex::new(())),
        })
    }
    pub fn open(
        path: &Path,
        source: &Path,
        limits: BatchLimits,
        max_generation_id: u64,
    ) -> Result<Self, DurableError> {
        let storage = limits.storage_limits()?;
        let inner = DurablePathIndex::open_impl(
            path,
            source,
            VIRTUAL_PROJECT,
            storage,
            max_generation_id,
            true,
        )?;
        match inner.snapshot() {
            Ok(snapshot) => {
                registry(&snapshot, limits)?;
            }
            Err(DurableError::Index(IndexError::NoPublishedGeneration)) => {}
            Err(error) => return Err(error),
        }
        Ok(Self {
            inner,
            limits,
            writer: Mutex::new(()),
            scratch: Arc::new(Mutex::new(())),
        })
    }
    pub fn snapshot(&self) -> Result<BatchSnapshot, DurableError> {
        self.snapshot_impl(None)
    }
    #[cfg(feature = "fault-injection")]
    pub fn snapshot_with_observer(
        &self,
        observer: &mut dyn FnMut(),
    ) -> Result<BatchSnapshot, DurableError> {
        self.snapshot_impl(Some(observer))
    }
    fn snapshot_impl(
        &self,
        observer: Option<&mut dyn FnMut()>,
    ) -> Result<BatchSnapshot, DurableError> {
        let inner = self.inner.snapshot()?;
        let projects = {
            let _scratch = acquire(&self.scratch)?;
            registry(&inner, self.limits)?
        };
        if let Some(observer) = observer {
            observer();
        }
        if self.inner.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate);
        }
        self.inner.check_source()?;
        if self.inner.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate);
        }
        Ok(BatchSnapshot {
            inner,
            projects,
            limits: self.limits,
            scratch: Arc::clone(&self.scratch),
        })
    }
    pub fn replace<I>(
        &self,
        expected: Option<u64>,
        projects: Vec<BatchProject<I>>,
        cancellation: &CancellationToken,
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        self.replace_impl(expected, projects, cancellation, FaultPoint::None)
    }
    #[cfg(feature = "fault-injection")]
    pub fn replace_with_fault<I>(
        &self,
        expected: Option<u64>,
        projects: Vec<BatchProject<I>>,
        cancellation: &CancellationToken,
        fault: FaultPoint,
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        self.replace_impl(expected, projects, cancellation, fault)
    }
    fn replace_impl<I>(
        &self,
        expected: Option<u64>,
        projects: Vec<BatchProject<I>>,
        cancellation: &CancellationToken,
        fault: FaultPoint,
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        let _writer = acquire(&self.writer)?;
        cancellation.check()?;
        if projects.is_empty() || projects.len() > self.limits.max_projects {
            return Err(DurableError::LimitExceeded);
        }
        let previous = match self.inner.snapshot() {
            Ok(snapshot) => Some(snapshot),
            Err(DurableError::Index(IndexError::NoPublishedGeneration)) => None,
            Err(error) => return Err(error),
        };
        let actual = previous.as_ref().map(PathSnapshot::generation);
        if expected != actual {
            return Err(IndexError::StaleGeneration { expected, actual }.into());
        }
        let mut names = BTreeSet::new();
        for item in &projects {
            PathIndex::new(&item.project, self.limits.per_project)?;
            if !names.insert(item.project.clone()) {
                return Err(IndexError::InvalidProject.into());
            }
        }
        if let Some(ref prior) = previous {
            let _scratch = acquire(&self.scratch)?;
            if registry(prior, self.limits)? != names {
                return Err(IndexError::InvalidProject.into());
            }
        }
        let mut all = Vec::new();
        let mut count = 0usize;
        let mut bytes = 0usize;
        for item in projects {
            cancellation.check()?;
            let generation = build(
                &item.project,
                self.limits.per_project,
                item.expected_count,
                item.paths,
                0,
                cancellation,
            )?;
            count = count
                .checked_add(generation.manifest.len())
                .ok_or(DurableError::LimitExceeded)?;
            if count > self.limits.max_batch_documents {
                return Err(DurableError::LimitExceeded);
            }
            all.push(Ok(format!("{PREFIX}/{}", item.project)));
            for path in generation.manifest {
                bytes = bytes
                    .checked_add(path.len())
                    .ok_or(DurableError::LimitExceeded)?;
                if bytes > self.limits.max_batch_path_bytes {
                    return Err(DurableError::LimitExceeded);
                }
                all.push(Ok(format!("{PREFIX}{path}")));
            }
        }
        cancellation.check()?;
        self.inner
            .replace_impl(expected, all.len(), all, cancellation, fault, None, None)
    }
}

impl BatchSnapshot {
    pub fn generation(&self) -> u64 {
        self.inner.generation()
    }
    pub fn projects(&self) -> Vec<String> {
        self.projects.iter().cloned().collect()
    }
    fn query(
        &self,
        project: &str,
        query: &str,
        expression: bool,
    ) -> Result<HashSet<String>, DurableError> {
        if !self.projects.contains(project) {
            return Err(IndexError::InvalidProject.into());
        }
        let _scratch = acquire(&self.scratch)?;
        let prefix = format!("{PREFIX}/{project}/");
        let paths: Vec<_> = self
            .inner
            .inner
            .manifest
            .iter()
            .filter_map(|path| {
                path.strip_prefix(&prefix)
                    .map(|tail| Ok(format!("/{project}/{tail}")))
            })
            .collect();
        let generation = build(
            project,
            self.limits.per_project,
            paths.len(),
            paths,
            self.generation(),
            &CancellationToken::new(),
        )?;
        let snapshot = PathSnapshot {
            inner: Arc::new(generation),
        };
        if expression {
            snapshot.lookup_expression(query)
        } else {
            snapshot.lookup(query)
        }
        .map_err(Into::into)
    }
    pub fn lookup(&self, project: &str, term: &str) -> Result<HashSet<String>, DurableError> {
        self.query(project, term, false)
    }
    pub fn lookup_expression(
        &self,
        project: &str,
        query: &str,
    ) -> Result<HashSet<String>, DurableError> {
        self.query(project, query, true)
    }
}
