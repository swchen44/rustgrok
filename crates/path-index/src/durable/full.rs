// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
//! Bounded development FULL store. One record and pointer cover the entire fixed registry.
//! No source text is persisted. This module reuses the macOS durable descriptor primitives.
use super::*;
use rustgrok_analysis::{analyze_full, validate_full_index_terms, FullError};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{RwLock, Weak};

const MAGIC: &[u8] = b"RGFULL01";
const GEN: &[u8] = b"RGFGEN01";
const READER: &str = "utf8-default-tabs";

#[derive(Debug)]
pub enum FullStoreError {
    Storage(DurableError),
    Analysis { path: String, error: FullError },
    UnsupportedReader,
    InvalidEncoding { path: String },
    InvalidQuery,
    LimitExceeded,
}
impl From<DurableError> for FullStoreError {
    fn from(e: DurableError) -> Self {
        Self::Storage(e)
    }
}
impl From<io::Error> for FullStoreError {
    fn from(e: io::Error) -> Self {
        Self::Storage(e.into())
    }
}
impl From<IndexError> for FullStoreError {
    fn from(e: IndexError) -> Self {
        Self::Storage(e.into())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FullLimits {
    pub max_projects: usize,
    pub max_documents: usize,
    pub max_file_bytes: usize,
    pub max_source_bytes: usize,
    pub max_postings: usize,
    pub max_generation_bytes: usize,
    pub max_generation_id: u64,
}
impl Default for FullLimits {
    fn default() -> Self {
        Self {
            max_projects: 32,
            max_documents: 10_000,
            max_file_bytes: 8 * 1024 * 1024,
            max_source_bytes: 16 * 1024 * 1024,
            max_postings: 200_000,
            max_generation_bytes: 32 * 1024 * 1024,
            max_generation_id: 32,
        }
    }
}
impl FullLimits {
    fn validate(self) -> Result<Self, FullStoreError> {
        let hard = Self::default();
        if self.max_projects == 0
            || self.max_projects > hard.max_projects
            || self.max_documents == 0
            || self.max_documents > hard.max_documents
            || self.max_file_bytes == 0
            || self.max_file_bytes > hard.max_file_bytes
            || self.max_source_bytes == 0
            || self.max_source_bytes > hard.max_source_bytes
            || self.max_postings == 0
            || self.max_postings > hard.max_postings
            || self.max_generation_bytes < 64
            || self.max_generation_bytes > hard.max_generation_bytes
            || self.max_generation_id == 0
            || self.max_generation_id > hard.max_generation_id
        {
            return Err(FullStoreError::LimitExceeded);
        }
        Ok(self)
    }
    fn paths(self) -> IndexLimits {
        IndexLimits {
            max_documents: self.max_documents,
            max_path_bytes: 4096,
            max_total_path_bytes: 16 * 1024 * 1024,
            max_live_generations: 2,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchPosition {
    pub position: usize,
    pub start_utf16: usize,
    pub end_utf16: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullMatch {
    pub path: String,
    pub source_sha256: String,
    pub positions: Vec<MatchPosition>,
}
struct Document {
    path: String,
    size: usize,
    digest: [u8; 32],
    tokens: usize,
    derived: [u8; 32],
}
type Postings = BTreeMap<String, BTreeMap<usize, Vec<MatchPosition>>>;
struct FullGeneration {
    id: u64,
    projects: BTreeSet<String>,
    docs: Vec<Document>,
    postings: Postings,
}
pub struct FullSnapshot {
    data: Arc<FullGeneration>,
}
impl FullSnapshot {
    pub fn generation(&self) -> u64 {
        self.data.id
    }
    /// Evaluate the admitted ASCII Boolean subset as unranked document paths.
    pub fn lookup_expression(
        &self,
        project: &str,
        expression: &str,
    ) -> Result<BTreeSet<String>, FullStoreError> {
        use crate::{query::parse_full_expression, PathOccur, PathQuery};
        if !self.data.projects.contains(project) {
            return Err(IndexError::InvalidProject.into());
        }
        if !expression.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b' ' | b'(' | b')' | b'+' | b'-')
        }) {
            return Err(FullStoreError::InvalidQuery);
        }
        fn evaluate(
            snapshot: &FullSnapshot,
            project: &str,
            query: PathQuery,
        ) -> Result<BTreeSet<String>, FullStoreError> {
            match query {
                PathQuery::Term(term) => Ok(snapshot
                    .lookup(project, &term)?
                    .into_iter()
                    .map(|m| m.path)
                    .collect()),
                PathQuery::Boolean(clauses) => {
                    let mut required: Option<BTreeSet<String>> = None;
                    let mut optional = BTreeSet::new();
                    let mut excluded = BTreeSet::new();
                    for clause in clauses {
                        let paths = evaluate(snapshot, project, clause.query)?;
                        match clause.occur {
                            PathOccur::Must => match &mut required {
                                Some(current) => current.retain(|path| paths.contains(path)),
                                None => required = Some(paths),
                            },
                            PathOccur::Should => optional.extend(paths),
                            PathOccur::MustNot => excluded.extend(paths),
                        }
                    }
                    let mut result = required.unwrap_or(optional);
                    result.retain(|path| !excluded.contains(path));
                    Ok(result)
                }
            }
        }
        let query = parse_full_expression(expression, 4096).map_err(|error| match error {
            IndexError::LimitExceeded => FullStoreError::LimitExceeded,
            _ => FullStoreError::InvalidQuery,
        })?;
        evaluate(self, project, query)
    }
    pub fn lookup(&self, project: &str, query: &str) -> Result<Vec<FullMatch>, FullStoreError> {
        if !self.data.projects.contains(project) {
            return Err(IndexError::InvalidProject.into());
        }
        // Ordinary CustomQueryParser terms use tokenStream, not multi-term normalize.
        if query.is_empty()
            || matches!(query, "AND" | "OR" | "NOT")
            || query.chars().any(|c| !(c.is_alphanumeric() || c == '_'))
        {
            return Err(FullStoreError::InvalidQuery);
        }
        let tokens = analyze_full(query).map_err(|_| FullStoreError::InvalidQuery)?;
        if tokens.len() != 1 || tokens[0].start_byte != 0 || tokens[0].end_byte != query.len() {
            return Err(FullStoreError::InvalidQuery);
        }
        validate_full_index_terms(&tokens).map_err(|_| FullStoreError::InvalidQuery)?;
        let prefix = format!("/{project}/");
        Ok(self
            .data
            .postings
            .get(&tokens[0].term)
            .into_iter()
            .flat_map(|p| p.iter())
            .filter_map(|(id, positions)| {
                let doc = &self.data.docs[*id];
                doc.path.starts_with(&prefix).then(|| FullMatch {
                    path: doc.path.clone(),
                    source_sha256: hex(&doc.digest),
                    positions: positions.clone(),
                })
            })
            .collect())
    }
}
fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}
fn full_marker(source: (u64, u64)) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    put_u64(&mut out, source.0);
    put_u64(&mut out, source.1);
    out.extend_from_slice(READER.as_bytes());
    let digest = Sha256::digest(&out);
    out.extend_from_slice(&digest);
    out
}
pub struct DurableFullIndex {
    root: File,
    source: File,
    source_path: PathBuf,
    source_chain: Vec<(u64, u64)>,
    _lock: File,
    current: RwLock<Option<Arc<FullGeneration>>>,
    pointer: Mutex<Vec<u8>>,
    writer: Mutex<()>,
    live: Mutex<Vec<Weak<FullGeneration>>>,
    indeterminate: AtomicBool,
    limits: FullLimits,
}
impl DurableFullIndex {
    pub fn create(
        parent: &Path,
        name: &str,
        source: &Path,
        reader: &str,
        limits: FullLimits,
    ) -> Result<Self, FullStoreError> {
        if reader != READER {
            return Err(FullStoreError::UnsupportedReader);
        }
        let limits = limits.validate()?;
        let (src, chain) = walk(source)?;
        let (parent, parent_chain) = walk(parent)?;
        separate(&chain, &parent_chain)?;
        let meta = parent.metadata()?;
        if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o022 != 0 {
            return Err(DurableError::UnsafePath.into());
        }
        let name = component(name)?;
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        parent.sync_all()?;
        let root = open_at(&parent, &name, FLAGS_DIR, 0)?;
        let lock = create_at(&root, "LOCK")?;
        lock_file(&lock)?;
        lock.sync_all()?;
        write_sync(create_at(&root, "MARKER")?, &full_marker(identity(&src)?))?;
        let ptr = pointer(0, &[0; 32]);
        write_sync(create_at(&root, "CURRENT")?, &ptr)?;
        root.sync_all()?;
        let out = Self {
            root,
            source: src,
            source_path: source.to_owned(),
            source_chain: chain,
            _lock: lock,
            current: RwLock::new(None),
            pointer: Mutex::new(ptr),
            writer: Mutex::new(()),
            live: Mutex::new(vec![]),
            indeterminate: AtomicBool::new(false),
            limits,
        };
        out.check_source()?;
        Ok(out)
    }
    pub fn open(path: &Path, source: &Path, limits: FullLimits) -> Result<Self, FullStoreError> {
        let limits = limits.validate()?;
        let (src, chain) = walk(source)?;
        let (root, root_chain) = walk(path)?;
        separate(&chain, &root_chain)?;
        let meta = root.metadata()?;
        if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err(DurableError::UnsafePath.into());
        }
        let lock = open_at(
            &root,
            &component("LOCK")?,
            libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0,
        )?;
        lock_file(&lock)?;
        let expected = full_marker(identity(&src)?);
        if read_at(&root, "MARKER", expected.len())?.as_deref() != Some(&expected) {
            return Err(DurableError::Corrupt.into());
        }
        let ptr = read_at(&root, "CURRENT", 80)?.ok_or(DurableError::Corrupt)?;
        let id = decode_pointer(&ptr, limits)?;
        let current = if id == 0 {
            None
        } else {
            let raw = read_at(&root, &format!("F-{id}"), limits.max_generation_bytes)?
                .ok_or(DurableError::Corrupt)?;
            Some(Arc::new(decode_full(&raw, id, &ptr[16..48], limits)?))
        };
        root.sync_all()?;
        let live = current.iter().map(Arc::downgrade).collect();
        let out = Self {
            root,
            source: src,
            source_path: source.to_owned(),
            source_chain: chain,
            _lock: lock,
            current: RwLock::new(current),
            pointer: Mutex::new(ptr),
            writer: Mutex::new(()),
            live: Mutex::new(live),
            indeterminate: AtomicBool::new(false),
            limits,
        };
        out.check_source()?;
        Ok(out)
    }
    fn check_source(&self) -> Result<(), FullStoreError> {
        if self.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate.into());
        }
        let (src, chain) = walk(&self.source_path).map_err(|_| DurableError::SourceChanged)?;
        if chain != self.source_chain || identity(&src)? != identity(&self.source)? {
            return Err(DurableError::SourceChanged.into());
        }
        Ok(())
    }
    pub fn snapshot(&self) -> Result<FullSnapshot, FullStoreError> {
        self.check_source()?;
        let data = self
            .current
            .read()
            .map_err(|_| IndexError::LockPoisoned)?
            .clone()
            .ok_or(IndexError::NoPublishedGeneration)?;
        self.check_source()?;
        Ok(FullSnapshot { data })
    }
    pub fn replace_from_source(
        &self,
        expected: Option<u64>,
        projects: &[String],
        cancel: &CancellationToken,
    ) -> Result<u64, FullStoreError> {
        self.replace_impl(expected, projects, cancel, FaultPoint::None, None, None)
    }
    #[cfg(feature = "fault-injection")]
    pub fn replace_with_observer(
        &self,
        expected: Option<u64>,
        projects: &[String],
        cancel: &CancellationToken,
        second: &mut dyn FnMut(),
        precommit: &mut dyn FnMut(),
    ) -> Result<u64, FullStoreError> {
        self.replace_impl(
            expected,
            projects,
            cancel,
            FaultPoint::None,
            Some(second),
            Some(precommit),
        )
    }
    #[cfg(feature = "fault-injection")]
    pub fn replace_with_fault(
        &self,
        expected: Option<u64>,
        projects: &[String],
        cancel: &CancellationToken,
        fault: FaultPoint,
    ) -> Result<u64, FullStoreError> {
        self.replace_impl(expected, projects, cancel, fault, None, None)
    }
    /// Test-only observer after rename, before the publication directory sync.
    #[cfg(feature = "fault-injection")]
    pub fn replace_with_publication_observer(
        &self,
        expected: Option<u64>,
        projects: &[String],
        cancel: &CancellationToken,
        observer: &mut dyn FnMut(),
    ) -> Result<u64, FullStoreError> {
        self.replace_impl(
            expected,
            projects,
            cancel,
            FaultPoint::ObserveAfterRename,
            None,
            Some(observer),
        )
    }
    fn replace_impl(
        &self,
        expected: Option<u64>,
        projects: &[String],
        cancel: &CancellationToken,
        fault: FaultPoint,
        mut second: Option<&mut dyn FnMut()>,
        mut precommit: Option<&mut dyn FnMut()>,
    ) -> Result<u64, FullStoreError> {
        let _writer = self.writer.try_lock().map_err(|_| DurableError::Busy)?;
        self.check_source()?;
        cancel.check()?;
        let current = self
            .current
            .read()
            .map_err(|_| IndexError::LockPoisoned)?
            .clone();
        let actual = current.as_ref().map(|g| g.id);
        if actual != expected {
            return Err(IndexError::StaleGeneration { expected, actual }.into());
        }
        let registry = validate_projects(projects, self.limits)?;
        if current.as_ref().is_some_and(|g| g.projects != registry) {
            return Err(IndexError::InvalidProject.into());
        }
        {
            let mut live = self.live.lock().map_err(|_| IndexError::LockPoisoned)?;
            live.retain(|g| g.strong_count() > 0);
            if live.len() >= 2 {
                return Err(FullStoreError::LimitExceeded);
            }
        }
        let mut data = FullGeneration {
            id: 0,
            projects: registry.clone(),
            docs: vec![],
            postings: BTreeMap::new(),
        };
        let mut observed = Vec::new();
        let mut source_bytes = 0;
        let mut path_bytes = 0;
        let mut entries = 0;
        let mut count = 0;
        for (i, project) in registry.iter().enumerate() {
            if i == 1 {
                if let Some(observe) = second.as_mut() {
                    observe();
                }
            }
            cancel.check()?;
            let dir = open_at(&self.source, &component(project)?, FLAGS_DIR, 0)?;
            let observation = scanner::observe_project(&dir, project, self.limits.paths(), cancel)?;
            entries += observation.entries;
            path_bytes += observation.path_bytes;
            if entries > self.limits.max_documents * 2 + 128 * registry.len()
                || path_bytes > 16 * 1024 * 1024
            {
                return Err(FullStoreError::LimitExceeded);
            }
            for path in &observation.paths {
                cancel.check()?;
                if data.docs.len() >= self.limits.max_documents {
                    return Err(FullStoreError::LimitExceeded);
                }
                let bytes = self.read_source(path, cancel)?;
                source_bytes += bytes.len();
                if source_bytes > self.limits.max_source_bytes {
                    return Err(FullStoreError::LimitExceeded);
                }
                let bom = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
                    3
                } else {
                    0
                };
                // UTF-16 and malformed UTF-8 never reach the analyzer or silently lose data.
                let text = std::str::from_utf8(&bytes[bom..])
                    .map_err(|_| FullStoreError::InvalidEncoding { path: path.clone() })?;
                let tokens = analyze_full(text).map_err(|error| FullStoreError::Analysis {
                    path: path.clone(),
                    error,
                })?;
                validate_full_index_terms(&tokens).map_err(|error| FullStoreError::Analysis {
                    path: path.clone(),
                    error,
                })?;
                count += tokens.len();
                if count > self.limits.max_postings {
                    return Err(FullStoreError::LimitExceeded);
                }
                let doc = data.docs.len();
                let derived = derived_digest(tokens.iter().map(|t| {
                    (
                        t.position,
                        t.term.as_str(),
                        t.start_utf16,
                        t.end_utf16,
                        t.start_byte + bom,
                        t.end_byte + bom,
                    )
                }));
                data.docs.push(Document {
                    path: path.clone(),
                    size: bytes.len(),
                    digest: Sha256::digest(&bytes).into(),
                    tokens: tokens.len(),
                    derived,
                });
                for token in tokens {
                    data.postings
                        .entry(token.term)
                        .or_default()
                        .entry(doc)
                        .or_default()
                        .push(MatchPosition {
                            position: token.position,
                            start_utf16: token.start_utf16,
                            end_utf16: token.end_utf16,
                            start_byte: token.start_byte + bom,
                            end_byte: token.end_byte + bom,
                        });
                }
            }
            observed.push((project.clone(), observation));
        }
        let mut id = actual.unwrap_or(0) + 1;
        let (record, digest) = loop {
            if id > self.limits.max_generation_id {
                return Err(FullStoreError::LimitExceeded);
            }
            cancel.check()?;
            self.check_source()?;
            let file = match create_at(&self.root, &format!("F-{id}")) {
                Ok(f) => f,
                Err(DurableError::Io(e)) if e.kind() == io::ErrorKind::AlreadyExists => {
                    validate_existing_regular(&self.root, &format!("F-{id}"))?;
                    id += 1;
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            data.id = id;
            let encoded = encode_full(&data, self.limits)?;
            inject_no_space(fault, FaultPoint::NoSpaceGenerationWrite)?;
            write_sync(file, &encoded.0)?;
            self.root.sync_all()?;
            inject_corruption(
                &self.root,
                &format!("F-{id}"),
                fault,
                FaultPoint::CorruptGenerationBeforeReadback,
            )?;
            if read_at(
                &self.root,
                &format!("F-{id}"),
                self.limits.max_generation_bytes,
            )?
            .as_deref()
                != Some(encoded.0.as_slice())
            {
                return Err(DurableError::Corrupt.into());
            }
            break encoded;
        };
        let _ = record;
        inject_exit(fault, FaultPoint::ExitAfterGenerationSync);
        let pending = create_at(&self.root, &format!("P-{id}"))?;
        let next_ptr = pointer(id, &digest);
        inject_no_space(fault, FaultPoint::NoSpacePointerWrite)?;
        write_sync(pending, &next_ptr)?;
        self.root.sync_all()?;
        inject_corruption(
            &self.root,
            &format!("P-{id}"),
            fault,
            FaultPoint::CorruptPointerBeforeReadback,
        )?;
        if read_at(&self.root, &format!("P-{id}"), 80)?.as_deref() != Some(next_ptr.as_slice()) {
            return Err(DurableError::Corrupt.into());
        }
        if fault != FaultPoint::ObserveAfterRename {
            if let Some(observe) = precommit.as_mut() {
                observe();
            }
        }
        self.check_source()?;
        for (project, original) in &observed {
            cancel.check()?;
            let dir = open_at(&self.source, &component(project)?, FLAGS_DIR, 0)?;
            if scanner::observe_project(&dir, project, self.limits.paths(), cancel)? != *original {
                return Err(DurableError::SourceChanged.into());
            }
        }
        for doc in &data.docs {
            let bytes = self.read_source(&doc.path, cancel)?;
            if bytes.len() != doc.size || Sha256::digest(&bytes).as_slice() != doc.digest {
                return Err(DurableError::SourceChanged.into());
            }
        }
        let mut ptr = self.pointer.lock().map_err(|_| IndexError::LockPoisoned)?;
        if read_at(&self.root, "CURRENT", 80)?.as_deref() != Some(ptr.as_slice()) {
            return Err(DurableError::Corrupt.into());
        }
        let next = Arc::new(data);
        let mut live = self.live.lock().map_err(|_| IndexError::LockPoisoned)?;
        cancel.begin_publication()?;
        inject_exit(fault, FaultPoint::ExitBeforeRename);
        rename_at(&self.root, &format!("P-{id}"), "CURRENT")?;
        inject_exit(fault, FaultPoint::ExitAfterRename);
        if fault == FaultPoint::ObserveAfterRename {
            if let Some(observe) = precommit.as_mut() {
                if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(observe))
                {
                    self.indeterminate.store(true, Ordering::Release);
                    std::panic::resume_unwind(panic);
                }
            }
        }
        if (cfg!(feature = "fault-injection") && fault == FaultPoint::FailDirectorySync)
            || self.root.sync_all().is_err()
        {
            self.indeterminate.store(true, Ordering::Release);
            return Err(DurableError::PublicationIndeterminate.into());
        }
        // Queries retain the previous immutable Arc throughout all durable I/O.
        // The new in-memory batch becomes visible only after directory sync.
        let mut current = match self.current.write() {
            Ok(current) => current,
            Err(_) => {
                self.indeterminate.store(true, Ordering::Release);
                return Err(DurableError::PublicationIndeterminate.into());
            }
        };
        *ptr = next_ptr;
        live.push(Arc::downgrade(&next));
        *current = Some(next);
        Ok(id)
    }
    fn read_source(
        &self,
        path: &str,
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>, FullStoreError> {
        cancel.check()?;
        let mut parts = path
            .strip_prefix('/')
            .ok_or(DurableError::UnsafePath)?
            .split('/')
            .peekable();
        let mut dir = self.source.try_clone()?;
        while let Some(part) = parts.next() {
            if parts.peek().is_some() {
                dir = open_at(&dir, &component(part)?, FLAGS_DIR, 0)?;
            } else {
                let file = open_at(
                    &dir,
                    &component(part)?,
                    libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
                    0,
                )?;
                let meta = file.metadata()?;
                if !meta.is_file() || meta.mode() & 0o444 == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "source is not readable regular file",
                    )
                    .into());
                }
                if meta.len() > self.limits.max_file_bytes as u64 {
                    return Err(FullStoreError::LimitExceeded);
                }
                let mut bytes = Vec::new();
                file.take(self.limits.max_file_bytes as u64 + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > self.limits.max_file_bytes {
                    return Err(FullStoreError::LimitExceeded);
                }
                cancel.check()?;
                return Ok(bytes);
            }
        }
        Err(DurableError::UnsafePath.into())
    }
}
fn validate_projects(
    projects: &[String],
    limits: FullLimits,
) -> Result<BTreeSet<String>, FullStoreError> {
    if projects.is_empty() || projects.len() > limits.max_projects {
        return Err(FullStoreError::LimitExceeded);
    }
    let mut out = BTreeSet::new();
    for p in projects {
        PathIndex::new(p, limits.paths())?;
        if !p
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
            || !out.insert(p.clone())
        {
            return Err(IndexError::InvalidProject.into());
        }
    }
    Ok(out)
}
fn decode_pointer(ptr: &[u8], limits: FullLimits) -> Result<u64, FullStoreError> {
    if ptr.len() != 80
        || &ptr[..8] != PTR_MAGIC
        || Sha256::digest(&ptr[..48]).as_slice() != &ptr[48..]
    {
        return Err(DurableError::Corrupt.into());
    }
    let id = take_u64(ptr, &mut 8)?;
    if id > limits.max_generation_id || (id == 0 && ptr[16..48] != [0; 32]) {
        return Err(DurableError::Corrupt.into());
    }
    Ok(id)
}
fn put_string(out: &mut Vec<u8>, s: &str) {
    put_u64(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}
fn encode_full(
    data: &FullGeneration,
    limits: FullLimits,
) -> Result<(Vec<u8>, [u8; 32]), FullStoreError> {
    let mut out = GEN.to_vec();
    put_u64(&mut out, data.id);
    put_u64(&mut out, data.projects.len() as u64);
    for p in &data.projects {
        put_string(&mut out, p);
    }
    put_u64(&mut out, data.docs.len() as u64);
    for d in &data.docs {
        put_string(&mut out, &d.path);
        put_u64(&mut out, d.size as u64);
        out.extend_from_slice(&d.digest);
        put_u64(&mut out, d.tokens as u64);
        out.extend_from_slice(&d.derived);
    }
    put_u64(&mut out, data.postings.len() as u64);
    for (term, docs) in &data.postings {
        put_string(&mut out, term);
        put_u64(&mut out, docs.len() as u64);
        for (doc, positions) in docs {
            put_u64(&mut out, *doc as u64);
            put_u64(&mut out, positions.len() as u64);
            for p in positions {
                for n in [
                    p.position,
                    p.start_utf16,
                    p.end_utf16,
                    p.start_byte,
                    p.end_byte,
                ] {
                    put_u64(&mut out, n as u64);
                }
            }
        }
        if out.len() > limits.max_generation_bytes.saturating_sub(32) {
            return Err(FullStoreError::LimitExceeded);
        }
    }
    if out.len() > limits.max_generation_bytes.saturating_sub(32) {
        return Err(FullStoreError::LimitExceeded);
    }
    let digest: [u8; 32] = Sha256::digest(&out).into();
    out.extend_from_slice(&digest);
    Ok((out, digest))
}
fn bounded(input: &[u8], cursor: &mut usize, max: usize) -> Result<usize, DurableError> {
    let n: usize = take_u64(input, cursor)?
        .try_into()
        .map_err(|_| DurableError::Corrupt)?;
    if n > max {
        return Err(DurableError::Corrupt);
    }
    Ok(n)
}
fn decode_full(
    record: &[u8],
    id: u64,
    digest: &[u8],
    limits: FullLimits,
) -> Result<FullGeneration, FullStoreError> {
    if record.len() < 48 || record.len() > limits.max_generation_bytes {
        return Err(DurableError::Corrupt.into());
    }
    let input = &record[..record.len() - 32];
    let tail = &record[record.len() - 32..];
    if !input.starts_with(GEN) || tail != digest || Sha256::digest(input).as_slice() != tail {
        return Err(DurableError::Corrupt.into());
    }
    let mut at = 8;
    if take_u64(input, &mut at)? != id {
        return Err(DurableError::Corrupt.into());
    }
    let n = bounded(input, &mut at, limits.max_projects)?;
    let mut names = vec![];
    for _ in 0..n {
        names.push(take_string(input, &mut at, 4096)?);
    }
    let projects = validate_projects(&names, limits).map_err(|_| DurableError::Corrupt)?;
    let n = bounded(input, &mut at, limits.max_documents)?;
    let mut docs = vec![];
    let mut paths = BTreeSet::new();
    let mut bytes = 0usize;
    let mut path_bytes = 0;
    for _ in 0..n {
        let path = take_string(input, &mut at, 4096)?;
        if !valid_document_path(&path, &projects) || !paths.insert(path.clone()) {
            return Err(DurableError::Corrupt.into());
        }
        let size = bounded(input, &mut at, limits.max_file_bytes)?;
        bytes += size;
        path_bytes += path.len();
        if bytes > limits.max_source_bytes || path_bytes > 16 * 1024 * 1024 {
            return Err(DurableError::Corrupt.into());
        }
        let digest = input
            .get(at..at + 32)
            .ok_or(DurableError::Corrupt)?
            .try_into()
            .map_err(|_| DurableError::Corrupt)?;
        at += 32;
        let tokens = bounded(input, &mut at, limits.max_postings)?;
        let derived = input
            .get(at..at + 32)
            .ok_or(DurableError::Corrupt)?
            .try_into()
            .map_err(|_| DurableError::Corrupt)?;
        at += 32;
        docs.push(Document {
            path,
            size,
            digest,
            tokens,
            derived,
        });
    }
    let n = bounded(input, &mut at, limits.max_postings)?;
    let mut postings = BTreeMap::new();
    let mut total = 0;
    let mut unique = BTreeSet::new();
    for _ in 0..n {
        let term = take_string(input, &mut at, 32766)?;
        if term.is_empty() {
            return Err(DurableError::Corrupt.into());
        }
        let n = bounded(input, &mut at, docs.len())?;
        if n == 0 {
            return Err(DurableError::Corrupt.into());
        }
        let mut terms = BTreeMap::new();
        for _ in 0..n {
            let doc = bounded(input, &mut at, docs.len().saturating_sub(1))?;
            let source = docs.get(doc).ok_or(DurableError::Corrupt)?;
            let n = bounded(input, &mut at, limits.max_postings)?;
            total += n;
            if n == 0 || total > limits.max_postings {
                return Err(DurableError::Corrupt.into());
            }
            let mut positions = vec![];
            for _ in 0..n {
                let position = bounded(input, &mut at, 1_000_000)?;
                let start_utf16 = bounded(input, &mut at, limits.max_file_bytes * 2)?;
                let end_utf16 = bounded(input, &mut at, limits.max_file_bytes * 2)?;
                let start_byte = bounded(input, &mut at, source.size)?;
                let end_byte = bounded(input, &mut at, source.size)?;
                if start_utf16 >= end_utf16
                    || start_byte >= end_byte
                    || !unique.insert((doc, position))
                    || positions
                        .last()
                        .is_some_and(|p: &MatchPosition| p.position >= position)
                {
                    return Err(DurableError::Corrupt.into());
                }
                positions.push(MatchPosition {
                    position,
                    start_utf16,
                    end_utf16,
                    start_byte,
                    end_byte,
                });
            }
            if terms.insert(doc, positions).is_some() {
                return Err(DurableError::Corrupt.into());
            }
        }
        if postings.insert(term, terms).is_some() {
            return Err(DurableError::Corrupt.into());
        }
    }
    if at != input.len() {
        return Err(DurableError::Corrupt.into());
    }
    let mut by_doc: Vec<BTreeMap<usize, (&str, &MatchPosition)>> =
        (0..docs.len()).map(|_| BTreeMap::new()).collect();
    for (term, terms) in &postings {
        for (doc, positions) in terms {
            for p in positions {
                by_doc[*doc].insert(p.position, (term.as_str(), p));
            }
        }
    }
    for (doc, terms) in docs.iter().zip(&by_doc) {
        if terms.len() != doc.tokens || terms.keys().copied().ne(0..doc.tokens) {
            return Err(DurableError::Corrupt.into());
        }
        let digest = derived_digest(terms.iter().map(|(position, (term, p))| {
            (
                *position,
                *term,
                p.start_utf16,
                p.end_utf16,
                p.start_byte,
                p.end_byte,
            )
        }));
        if digest != doc.derived {
            return Err(DurableError::Corrupt.into());
        }
    }
    Ok(FullGeneration {
        id,
        projects,
        docs,
        postings,
    })
}

fn derived_digest<'a>(
    items: impl IntoIterator<Item = (usize, &'a str, usize, usize, usize, usize)>,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    for (position, term, start_utf16, end_utf16, start_byte, end_byte) in items {
        for n in [
            position,
            term.len(),
            start_utf16,
            end_utf16,
            start_byte,
            end_byte,
        ] {
            digest.update((n as u64).to_le_bytes());
        }
        digest.update(term.as_bytes());
    }
    digest.finalize().into()
}

fn valid_document_path(path: &str, projects: &BTreeSet<String>) -> bool {
    let Some(path) = path.strip_prefix('/') else {
        return false;
    };
    let Some((project, tail)) = path.split_once('/') else {
        return false;
    };
    projects.contains(project)
        && tail.split('/').all(|name| {
            !name.is_empty()
                && !name.starts_with('.')
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b))
        })
        && [".c", ".py", ".js"].iter().any(|ext| tail.ends_with(ext))
}
