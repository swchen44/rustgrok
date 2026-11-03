// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
//! macOS-only durable single-project path generations.
pub mod batch;
mod scanner;
use super::{CancellationToken, Generation, IndexError, IndexLimits, PathIndex, PathSnapshot};
use sha2::{Digest, Sha256};
use std::{
    ffi::CString,
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

const STORE_MAGIC: &[u8] = b"RGPSTORE1";
const GEN_MAGIC: &[u8] = b"RGPGEN01";
const PTR_MAGIC: &[u8] = b"RGPTR001";
const FLAGS_DIR: i32 = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW;

#[derive(Debug)]
pub enum DurableError {
    Io(io::Error),
    Index(IndexError),
    UnsafePath,
    Corrupt,
    Busy,
    LimitExceeded,
    PublicationIndeterminate,
    SourceChanged,
}

/// Test-only fault controls are usable only when `fault-injection` is enabled.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FaultPoint {
    None,
    ExitAfterGenerationSync,
    ExitBeforeRename,
    ExitAfterRename,
    FailDirectorySync,
    CorruptGenerationBeforeReadback,
    CorruptPointerBeforeReadback,
    NoSpaceGenerationWrite,
    NoSpacePointerWrite,
    ObserveBeforeCommit,
    ObserveBeforeWriterLock,
}

fn inject_no_space(fault: FaultPoint, point: FaultPoint) -> Result<(), DurableError> {
    if cfg!(feature = "fault-injection") && fault == point {
        return Err(io::Error::from_raw_os_error(libc::ENOSPC).into());
    }
    Ok(())
}

fn inject_exit(fault: FaultPoint, point: FaultPoint) {
    #[cfg(feature = "fault-injection")]
    if fault == point {
        // SAFETY: the explicitly enabled test feature intentionally simulates process death.
        unsafe { libc::_exit(91) }
    }
    #[cfg(not(feature = "fault-injection"))]
    let _ = (fault, point);
}

fn inject_corruption(
    dir: &File,
    name: &str,
    fault: FaultPoint,
    point: FaultPoint,
) -> Result<(), DurableError> {
    #[cfg(feature = "fault-injection")]
    if fault == point {
        let mut file = open_at(
            dir,
            &component(name)?,
            libc::O_WRONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0,
        )?;
        regular(&file)?;
        file.write_all(b"X")?;
        file.sync_all()?;
    }
    #[cfg(not(feature = "fault-injection"))]
    let _ = (dir, name, fault, point);
    Ok(())
}
impl From<io::Error> for DurableError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<IndexError> for DurableError {
    fn from(value: IndexError) -> Self {
        Self::Index(value)
    }
}

fn component(name: &str) -> Result<CString, DurableError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err(DurableError::UnsafePath);
    }
    CString::new(name).map_err(|_| DurableError::UnsafePath)
}
fn checked_fd(fd: i32) -> io::Result<File> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a successful open/openat returns a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn open_at(dir: &File, name: &CString, flags: i32, mode: libc::mode_t) -> io::Result<File> {
    // SAFETY: dir and name live through the call; openat returns an owned fd.
    checked_fd(unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags, mode as libc::c_uint) })
}
fn identity(file: &File) -> io::Result<(u64, u64)> {
    let meta = file.metadata()?;
    Ok((meta.dev(), meta.ino()))
}
fn walk(path: &Path) -> Result<(File, Vec<(u64, u64)>), DurableError> {
    let raw = path.as_os_str().as_bytes();
    if !raw.starts_with(b"/") {
        return Err(DurableError::UnsafePath);
    }
    let root = CString::new("/").expect("static path");
    // SAFETY: root is a NUL-terminated constant and flags require a directory.
    let mut dir = checked_fd(unsafe { libc::open(root.as_ptr(), FLAGS_DIR) })?;
    let mut chain = vec![identity(&dir)?];
    if raw == b"/" {
        return Ok((dir, chain));
    }
    for part in raw[1..].split(|b| *b == b'/') {
        if part.is_empty() || part == b"." || part == b".." {
            return Err(DurableError::UnsafePath);
        }
        let part = CString::new(part).map_err(|_| DurableError::UnsafePath)?;
        dir = open_at(&dir, &part, FLAGS_DIR, 0)?;
        chain.push(identity(&dir)?);
    }
    Ok((dir, chain))
}
fn separate(source: &[(u64, u64)], storage: &[(u64, u64)]) -> Result<(), DurableError> {
    if storage.contains(source.last().ok_or(DurableError::UnsafePath)?)
        || source.contains(storage.last().ok_or(DurableError::UnsafePath)?)
    {
        return Err(DurableError::UnsafePath);
    }
    Ok(())
}
fn regular(file: &File) -> Result<(), DurableError> {
    let meta = file.metadata()?;
    if !meta.is_file() || meta.nlink() != 1 {
        return Err(DurableError::UnsafePath);
    }
    Ok(())
}
fn create_at(dir: &File, name: &str) -> Result<File, DurableError> {
    let file = open_at(
        dir,
        &component(name)?,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        0o600,
    )?;
    regular(&file)?;
    Ok(file)
}
fn validate_existing_regular(dir: &File, name: &str) -> Result<(), DurableError> {
    let file = open_at(
        dir,
        &component(name)?,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        0,
    )?;
    regular(&file)
}
fn read_at(dir: &File, name: &str, max: usize) -> Result<Option<Vec<u8>>, DurableError> {
    let file = match open_at(
        dir,
        &component(name)?,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        0,
    ) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    regular(&file)?;
    let mut out = Vec::new();
    file.take((max as u64).saturating_add(1))
        .read_to_end(&mut out)?;
    if out.len() > max {
        return Err(DurableError::Corrupt);
    }
    Ok(Some(out))
}
fn rename_at(dir: &File, old: &str, new: &str) -> Result<(), DurableError> {
    let old = component(old)?;
    let new = component(new)?;
    // SAFETY: both names and the directory descriptor remain valid for the call.
    let result =
        unsafe { libc::renameat(dir.as_raw_fd(), old.as_ptr(), dir.as_raw_fd(), new.as_ptr()) };
    if result != 0 {
        return Err(io::Error::last_os_error().into());
    }
    Ok(())
}
fn write_sync(mut file: File, bytes: &[u8]) -> Result<(), DurableError> {
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn lock_file(file: &File) -> Result<(), DurableError> {
    regular(file)?;
    // SAFETY: flock only borrows the valid owned descriptor.
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result != 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::WouldBlock {
            return Err(DurableError::Busy);
        }
        return Err(error.into());
    }
    Ok(())
}
fn max_record(limits: IndexLimits, project: &str) -> Result<usize, DurableError> {
    8usize
        .checked_add(8)
        .and_then(|v| v.checked_add(8))
        .and_then(|v| v.checked_add(project.len()))
        .and_then(|v| v.checked_add(8))
        .and_then(|v| v.checked_add(limits.max_documents.checked_mul(8)?))
        .and_then(|v| v.checked_add(limits.max_total_path_bytes))
        .and_then(|v| v.checked_add(32))
        .ok_or(DurableError::LimitExceeded)
}
fn put_u64(out: &mut Vec<u8>, n: u64) {
    out.extend_from_slice(&n.to_le_bytes());
}
fn marker(project: &str, source_id: (u64, u64)) -> Result<Vec<u8>, DurableError> {
    let mut out = STORE_MAGIC.to_vec();
    put_u64(&mut out, source_id.0);
    put_u64(&mut out, source_id.1);
    put_u64(
        &mut out,
        project
            .len()
            .try_into()
            .map_err(|_| DurableError::LimitExceeded)?,
    );
    out.extend_from_slice(project.as_bytes());
    let digest: [u8; 32] = Sha256::digest(&out).into();
    out.extend_from_slice(&digest);
    Ok(out)
}
fn encode(project: &str, generation: &Generation) -> Result<(Vec<u8>, [u8; 32]), DurableError> {
    let mut out = GEN_MAGIC.to_vec();
    put_u64(&mut out, generation.id);
    put_u64(
        &mut out,
        project
            .len()
            .try_into()
            .map_err(|_| DurableError::LimitExceeded)?,
    );
    out.extend_from_slice(project.as_bytes());
    put_u64(
        &mut out,
        generation
            .manifest
            .len()
            .try_into()
            .map_err(|_| DurableError::LimitExceeded)?,
    );
    let mut paths: Vec<_> = generation.manifest.iter().collect();
    paths.sort_unstable();
    for path in paths {
        put_u64(
            &mut out,
            path.len()
                .try_into()
                .map_err(|_| DurableError::LimitExceeded)?,
        );
        out.extend_from_slice(path.as_bytes());
    }
    let digest: [u8; 32] = Sha256::digest(&out).into();
    out.extend_from_slice(&digest);
    Ok((out, digest))
}
fn take_u64(input: &[u8], cursor: &mut usize) -> Result<u64, DurableError> {
    let end = cursor.checked_add(8).ok_or(DurableError::Corrupt)?;
    let data: [u8; 8] = input
        .get(*cursor..end)
        .ok_or(DurableError::Corrupt)?
        .try_into()
        .map_err(|_| DurableError::Corrupt)?;
    *cursor = end;
    Ok(u64::from_le_bytes(data))
}
fn take_string(input: &[u8], cursor: &mut usize, max: usize) -> Result<String, DurableError> {
    let len: usize = take_u64(input, cursor)?
        .try_into()
        .map_err(|_| DurableError::Corrupt)?;
    if len > max {
        return Err(DurableError::Corrupt);
    }
    let end = cursor.checked_add(len).ok_or(DurableError::Corrupt)?;
    let value = std::str::from_utf8(input.get(*cursor..end).ok_or(DurableError::Corrupt)?)
        .map_err(|_| DurableError::Corrupt)?;
    *cursor = end;
    Ok(value.to_owned())
}
fn decode(
    record: &[u8],
    project: &str,
    id: u64,
    digest: &[u8],
    limits: IndexLimits,
) -> Result<Generation, DurableError> {
    if record.len() < 40 {
        return Err(DurableError::Corrupt);
    }
    let payload = &record[..record.len() - 32];
    let tail = &record[record.len() - 32..];
    if Sha256::digest(payload).as_slice() != tail
        || tail != digest
        || !payload.starts_with(GEN_MAGIC)
    {
        return Err(DurableError::Corrupt);
    }
    let mut cursor = 8;
    if take_u64(payload, &mut cursor)? != id
        || take_string(payload, &mut cursor, limits.max_path_bytes)? != project
    {
        return Err(DurableError::Corrupt);
    }
    let count: usize = take_u64(payload, &mut cursor)?
        .try_into()
        .map_err(|_| DurableError::Corrupt)?;
    if count > limits.max_documents {
        return Err(DurableError::Corrupt);
    }
    let mut paths = Vec::with_capacity(count.min(1024));
    let mut total = 0usize;
    for _ in 0..count {
        let path = take_string(payload, &mut cursor, limits.max_path_bytes)?;
        total = total.checked_add(path.len()).ok_or(DurableError::Corrupt)?;
        if total > limits.max_total_path_bytes {
            return Err(DurableError::Corrupt);
        }
        paths.push(Ok(path));
    }
    if cursor != payload.len() {
        return Err(DurableError::Corrupt);
    }
    build(project, limits, count, paths, id, &CancellationToken::new())
        .map_err(|_| DurableError::Corrupt)
}
fn pointer(id: u64, digest: &[u8; 32]) -> Vec<u8> {
    let mut out = PTR_MAGIC.to_vec();
    put_u64(&mut out, id);
    out.extend_from_slice(digest);
    let checksum: [u8; 32] = Sha256::digest(&out).into();
    out.extend_from_slice(&checksum);
    out
}
fn build<I>(
    project: &str,
    limits: IndexLimits,
    count: usize,
    paths: I,
    id: u64,
    cancellation: &CancellationToken,
) -> Result<Generation, DurableError>
where
    I: IntoIterator<Item = io::Result<String>>,
{
    let scratch = PathIndex::new(project, limits)?;
    let mut input = paths.into_iter();
    let checked = std::iter::from_fn(|| {
        if cancellation.check().is_err() {
            return Some(Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled")));
        }
        input.next()
    });
    let result = scratch.replace(None, count, checked, &CancellationToken::new());
    if let Err(e) = result {
        if cancellation.check().is_err() {
            return Err(IndexError::Cancelled.into());
        }
        return Err(e.into());
    }
    let mut current = scratch
        .current
        .write()
        .map_err(|_| IndexError::LockPoisoned)?;
    let value = current.take().ok_or(DurableError::Corrupt)?;
    let mut generation = Arc::try_unwrap(value).map_err(|_| DurableError::Corrupt)?;
    generation.id = id;
    Ok(generation)
}

pub struct DurablePathIndex {
    inner: PathIndex,
    root: File,
    source: File,
    source_path: PathBuf,
    source_chain: Vec<(u64, u64)>,
    _lock: File,
    pointer: Mutex<Option<Vec<u8>>>,
    max_generation_id: u64,
    indeterminate: AtomicBool,
}
impl DurablePathIndex {
    pub fn create(
        parent: &Path,
        name: &str,
        source: &Path,
        project: &str,
        limits: IndexLimits,
        max_generation_id: u64,
    ) -> Result<Self, DurableError> {
        if max_generation_id == 0 {
            return Err(DurableError::LimitExceeded);
        }
        let index = PathIndex::new(project, limits)?;
        let (source_dir, source_chain) = walk(source)?;
        let (parent, parent_chain) = walk(parent)?;
        separate(&source_chain, &parent_chain)?;
        let parent_meta = parent.metadata()?;
        // SAFETY: geteuid has no arguments or memory-safety preconditions.
        if parent_meta.uid() != unsafe { libc::geteuid() } || parent_meta.mode() & 0o022 != 0 {
            return Err(DurableError::UnsafePath);
        }
        let name = component(name)?;
        // SAFETY: parent and NUL-terminated name stay alive during mkdirat.
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        parent.sync_all()?;
        let root = open_at(&parent, &name, FLAGS_DIR, 0)?;
        let lock = create_at(&root, "LOCK")?;
        lock_file(&lock)?;
        lock.sync_all()?;
        write_sync(
            create_at(&root, "MARKER")?,
            &marker(project, identity(&source_dir)?)?,
        )?;
        let empty_pointer = pointer(0, &[0; 32]);
        write_sync(create_at(&root, "CURRENT")?, &empty_pointer)?;
        root.sync_all()?;
        let store = Self {
            inner: index,
            root,
            source: source_dir,
            source_path: source.to_path_buf(),
            source_chain,
            _lock: lock,
            pointer: Mutex::new(Some(empty_pointer)),
            max_generation_id,
            indeterminate: AtomicBool::new(false),
        };
        store.check_source()?;
        Ok(store)
    }
    pub fn open(
        path: &Path,
        source: &Path,
        project: &str,
        limits: IndexLimits,
        max_generation_id: u64,
    ) -> Result<Self, DurableError> {
        Self::open_impl(path, source, project, limits, max_generation_id, false)
    }
    fn open_impl(
        path: &Path,
        source: &Path,
        project: &str,
        limits: IndexLimits,
        max_generation_id: u64,
        batch: bool,
    ) -> Result<Self, DurableError> {
        if max_generation_id == 0 {
            return Err(DurableError::LimitExceeded);
        }
        let index = PathIndex::new(project, limits)?;
        let (source_dir, source_chain) = walk(source)?;
        let (root, root_chain) = walk(path)?;
        separate(&source_chain, &root_chain)?;
        let root_meta = root.metadata()?;
        // SAFETY: geteuid has no arguments or memory-safety preconditions.
        if root_meta.uid() != unsafe { libc::geteuid() } || root_meta.mode() & 0o077 != 0 {
            return Err(DurableError::UnsafePath);
        }
        let lock = open_at(
            &root,
            &component("LOCK")?,
            libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0,
        )?;
        lock_file(&lock)?;
        let batch_marker = read_at(&root, "BATCH-MARKER", 8)?;
        if (batch && batch_marker.as_deref() != Some(batch::BATCH_MARKER))
            || (!batch && batch_marker.is_some())
        {
            return Err(DurableError::Corrupt);
        }
        let expected_marker = marker(project, identity(&source_dir)?)?;
        if read_at(&root, "MARKER", expected_marker.len())?.as_deref() != Some(&expected_marker) {
            return Err(DurableError::Corrupt);
        }
        let data = read_at(&root, "CURRENT", 80)?.ok_or(DurableError::Corrupt)?;
        if data.len() != 80
            || &data[..8] != PTR_MAGIC
            || Sha256::digest(&data[..48]).as_slice() != &data[48..]
        {
            return Err(DurableError::Corrupt);
        }
        let mut cursor = 8;
        let id = take_u64(&data, &mut cursor)?;
        if id > max_generation_id {
            return Err(DurableError::Corrupt);
        }
        if id == 0 && data[16..48] != [0; 32] {
            return Err(DurableError::Corrupt);
        }
        if id != 0 {
            let raw = read_at(&root, &format!("G-{id}"), max_record(limits, project)?)?
                .ok_or(DurableError::Corrupt)?;
            let generation = Arc::new(decode(&raw, project, id, &data[16..48], limits)?);
            index
                .live
                .lock()
                .map_err(|_| IndexError::LockPoisoned)?
                .push(Arc::downgrade(&generation));
            *index
                .current
                .write()
                .map_err(|_| IndexError::LockPoisoned)? = Some(generation);
        }
        // A previous process may have stopped after rename but before its
        // directory sync. Re-establish it before exposing recovery.
        root.sync_all()?;
        let store = Self {
            inner: index,
            root,
            source: source_dir,
            source_path: source.to_path_buf(),
            source_chain,
            _lock: lock,
            pointer: Mutex::new(Some(data)),
            max_generation_id,
            indeterminate: AtomicBool::new(false),
        };
        store.check_source()?;
        Ok(store)
    }
    fn check_source(&self) -> Result<(), DurableError> {
        let (current, chain) = walk(&self.source_path).map_err(|_| DurableError::SourceChanged)?;
        if chain != self.source_chain
            || identity(&current).map_err(|_| DurableError::SourceChanged)?
                != identity(&self.source).map_err(|_| DurableError::SourceChanged)?
        {
            return Err(DurableError::SourceChanged);
        }
        Ok(())
    }
    pub fn snapshot(&self) -> Result<PathSnapshot, DurableError> {
        if self.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate);
        }
        self.check_source()?;
        let snapshot = self.inner.snapshot()?;
        self.check_source()?;
        if self.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate);
        }
        Ok(snapshot)
    }
    pub fn replace<I>(
        &self,
        expected: Option<u64>,
        count: usize,
        paths: I,
        cancellation: &CancellationToken,
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        self.replace_impl(
            expected,
            count,
            paths,
            cancellation,
            FaultPoint::None,
            None,
            None,
        )
    }

    #[cfg(feature = "fault-injection")]
    pub fn replace_with_fault<I>(
        &self,
        expected: Option<u64>,
        count: usize,
        paths: I,
        cancellation: &CancellationToken,
        fault: FaultPoint,
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        self.replace_impl(expected, count, paths, cancellation, fault, None, None)
    }

    #[cfg(feature = "fault-injection")]
    pub fn replace_with_observer<I>(
        &self,
        expected: Option<u64>,
        count: usize,
        paths: I,
        cancellation: &CancellationToken,
        observer: &mut dyn FnMut(),
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        self.replace_impl(
            expected,
            count,
            paths,
            cancellation,
            FaultPoint::None,
            Some(observer),
            None,
        )
    }

    #[cfg(feature = "fault-injection")]
    pub fn replace_with_precommit_observer<I>(
        &self,
        expected: Option<u64>,
        count: usize,
        paths: I,
        cancellation: &CancellationToken,
        observer: &mut dyn FnMut(),
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        self.replace_impl(
            expected,
            count,
            paths,
            cancellation,
            FaultPoint::ObserveBeforeCommit,
            Some(observer),
            None,
        )
    }

    #[cfg(feature = "fault-injection")]
    pub fn replace_with_prelock_observer<I>(
        &self,
        expected: Option<u64>,
        count: usize,
        paths: I,
        cancellation: &CancellationToken,
        observer: &mut dyn FnMut(),
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        self.replace_impl(
            expected,
            count,
            paths,
            cancellation,
            FaultPoint::ObserveBeforeWriterLock,
            Some(observer),
            None,
        )
    }

    #[allow(clippy::too_many_arguments)] // fault observers and the source validation are independent seams.
    fn replace_impl<I>(
        &self,
        expected: Option<u64>,
        count: usize,
        paths: I,
        cancellation: &CancellationToken,
        fault: FaultPoint,
        mut observer: Option<&mut dyn FnMut()>,
        validation: Option<&dyn Fn() -> Result<(), DurableError>>,
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        if self.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate);
        }
        if fault == FaultPoint::ObserveBeforeWriterLock {
            if let Some(ref mut observer) = observer {
                observer();
            }
        }
        let _writer = self.inner.writer.try_lock().map_err(|e| match e {
            std::sync::TryLockError::WouldBlock => DurableError::Busy,
            std::sync::TryLockError::Poisoned(_) => DurableError::Index(IndexError::LockPoisoned),
        })?;
        if self.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate);
        }
        self.replace_locked(
            expected,
            count,
            paths,
            cancellation,
            fault,
            observer,
            validation,
        )
    }

    // Caller holds the same writer lock used by every publication path.
    #[allow(clippy::too_many_arguments)]
    fn replace_locked<I>(
        &self,
        expected: Option<u64>,
        count: usize,
        paths: I,
        cancellation: &CancellationToken,
        fault: FaultPoint,
        mut observer: Option<&mut dyn FnMut()>,
        validation: Option<&dyn Fn() -> Result<(), DurableError>>,
    ) -> Result<u64, DurableError>
    where
        I: IntoIterator<Item = io::Result<String>>,
    {
        self.check_source()?;
        let actual = self
            .inner
            .current
            .read()
            .map_err(|_| IndexError::LockPoisoned)?
            .as_ref()
            .map(|g| g.id);
        if actual != expected {
            return Err(IndexError::StaleGeneration { expected, actual }.into());
        }
        {
            let mut live = self
                .inner
                .live
                .lock()
                .map_err(|_| IndexError::LockPoisoned)?;
            live.retain(|g| g.strong_count() > 0);
            if live.len() >= self.inner.limits.max_live_generations {
                return Err(DurableError::LimitExceeded);
            }
        }
        cancellation.check()?;
        let mut generation = build(
            &self.inner.project,
            self.inner.limits,
            count,
            paths,
            0,
            cancellation,
        )?;
        self.check_source()?;
        let mut id = actual
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(DurableError::LimitExceeded)?;
        loop {
            if id > self.max_generation_id {
                return Err(DurableError::LimitExceeded);
            }
            cancellation.check()?;
            self.check_source()?;
            let file = match create_at(&self.root, &format!("G-{id}")) {
                Ok(file) => file,
                Err(DurableError::Io(e)) if e.kind() == io::ErrorKind::AlreadyExists => {
                    validate_existing_regular(&self.root, &format!("G-{id}"))?;
                    id = id.checked_add(1).ok_or(DurableError::LimitExceeded)?;
                    continue;
                }
                Err(e) => return Err(e),
            };
            generation.id = id;
            let (record, digest) = encode(&self.inner.project, &generation)?;
            inject_no_space(fault, FaultPoint::NoSpaceGenerationWrite)?;
            write_sync(file, &record)?;
            self.root.sync_all()?;
            inject_corruption(
                &self.root,
                &format!("G-{id}"),
                fault,
                FaultPoint::CorruptGenerationBeforeReadback,
            )?;
            if read_at(
                &self.root,
                &format!("G-{id}"),
                max_record(self.inner.limits, &self.inner.project)?,
            )?
            .as_deref()
                != Some(record.as_slice())
            {
                return Err(DurableError::Corrupt);
            }
            inject_exit(fault, FaultPoint::ExitAfterGenerationSync);
            let pending = match create_at(&self.root, &format!("P-{id}")) {
                Ok(file) => file,
                Err(DurableError::Io(e)) if e.kind() == io::ErrorKind::AlreadyExists => {
                    validate_existing_regular(&self.root, &format!("P-{id}"))?;
                    id = id.checked_add(1).ok_or(DurableError::LimitExceeded)?;
                    continue;
                }
                Err(e) => return Err(e),
            };
            let next_pointer = pointer(id, &digest);
            inject_no_space(fault, FaultPoint::NoSpacePointerWrite)?;
            write_sync(pending, &next_pointer)?;
            self.root.sync_all()?;
            inject_corruption(
                &self.root,
                &format!("P-{id}"),
                fault,
                FaultPoint::CorruptPointerBeforeReadback,
            )?;
            if read_at(&self.root, &format!("P-{id}"), next_pointer.len())?.as_deref()
                != Some(next_pointer.as_slice())
            {
                return Err(DurableError::Corrupt);
            }
            cancellation.check()?;
            let mut expected_pointer = self.pointer.lock().map_err(|_| IndexError::LockPoisoned)?;
            if read_at(&self.root, "CURRENT", 80)? != *expected_pointer {
                return Err(DurableError::Corrupt);
            }
            let mut live = self
                .inner
                .live
                .lock()
                .map_err(|_| IndexError::LockPoisoned)?;
            let next = Arc::new(generation);
            if fault == FaultPoint::ObserveBeforeCommit {
                if let Some(ref mut observer) = observer {
                    observer();
                }
            }
            self.check_source()?;
            if let Some(validate) = validation {
                validate()?;
            }
            cancellation.begin_publication()?;
            inject_exit(fault, FaultPoint::ExitBeforeRename);
            rename_at(&self.root, &format!("P-{id}"), "CURRENT")?;
            inject_exit(fault, FaultPoint::ExitAfterRename);
            if fault != FaultPoint::ObserveBeforeCommit
                && fault != FaultPoint::ObserveBeforeWriterLock
            {
                if let Some(ref mut observer) = observer {
                    if let Err(panic) =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(observer))
                    {
                        self.indeterminate.store(true, Ordering::Release);
                        std::panic::resume_unwind(panic);
                    }
                }
            }
            let sync =
                if cfg!(feature = "fault-injection") && fault == FaultPoint::FailDirectorySync {
                    Err(io::Error::other("injected directory sync failure"))
                } else {
                    self.root.sync_all()
                };
            if sync.is_err() {
                self.indeterminate.store(true, Ordering::Release);
                return Err(DurableError::PublicationIndeterminate);
            }
            let mut current = match self.inner.current.write() {
                Ok(current) => current,
                Err(_) => {
                    self.indeterminate.store(true, Ordering::Release);
                    return Err(DurableError::PublicationIndeterminate);
                }
            };
            *expected_pointer = Some(next_pointer);
            live.push(Arc::downgrade(&next));
            *current = Some(next);
            return Ok(id);
        }
    }
}
