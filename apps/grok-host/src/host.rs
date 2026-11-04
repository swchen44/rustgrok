// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
//! Private, bounded development control adapter for single or batch stores.
use crate::mac::{self, Failure};
use rustgrok_path_index::durable::{
    batch::DurableBatchIndex,
    full::{DurableFullIndex, FullLimits},
    DurableError, DurablePathIndex,
};
use rustgrok_path_index::{CancellationToken, IndexError};
use serde_json::{json, Value};
use std::{
    ffi::CString,
    fs::{self, File},
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
const SOCKET: &str = "control.sock";
const REQUEST_BYTES: usize = 4096;
const RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_CLIENTS: usize = 8;
const TIMEOUT: Duration = Duration::from_secs(3);
static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);
extern "C" fn stop_signal(_: libc::c_int) {
    STOP_SIGNAL.store(true, Ordering::Release);
}
fn failure(code: i32, label: &'static str) -> Failure {
    Failure::new(code, label)
}
fn io_failure(_: io::Error) -> Failure {
    failure(6, "io_error")
}
fn host_fault(label: &'static str) -> Failure {
    failure(
        if label == "publication_indeterminate" {
            7
        } else {
            5
        },
        label,
    )
}
fn ids(file: &File) -> io::Result<(u64, u64)> {
    let m = file.metadata()?;
    Ok((m.dev(), m.ino()))
}
// These macOS SDK functions return a retained ACL object or NULL with errno.
// We reject all extended ACLs, including deny-only entries, and all errors except
// the locally observed ENOENT meaning no extended ACL on the still-open fd.
unsafe extern "C" {
    fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> *mut libc::c_void;
    fn acl_free(value: *mut libc::c_void) -> libc::c_int;
}
const ACL_TYPE_EXTENDED: libc::c_int = 0x00000100;
fn trusted_directory(dir: &File) -> Result<(), Failure> {
    let meta = dir.metadata().map_err(io_failure)?;
    let mut volume: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: the open directory descriptor and writable statfs buffer are valid.
    if unsafe { libc::fstatfs(dir.as_raw_fd(), &mut volume) } != 0 {
        return Err(failure(4, "unsafe_control"));
    }
    let fs_name: Vec<u8> = volume
        .f_fstypename
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    // Restrict this development profile to the native, locally observed APFS
    // security model; unknown or opaque authorization capabilities fail closed.
    if fs_name != b"apfs"
        || volume.f_flags & libc::MNT_LOCAL as u32 == 0
        || unsafe { libc::fpathconf(dir.as_raw_fd(), libc::_PC_EXTENDED_SECURITY_NP) } != 1
        || unsafe { libc::fpathconf(dir.as_raw_fd(), libc::_PC_AUTH_OPAQUE_NP) } != 0
    {
        return Err(failure(4, "unsafe_control"));
    }
    let uid = unsafe { libc::geteuid() };
    if meta.uid() != 0 && meta.uid() != uid {
        return Err(failure(4, "unsafe_control"));
    }
    let mode = meta.mode();
    if mode & 0o022 != 0 && mode & 0o1000 == 0 {
        return Err(failure(4, "unsafe_control"));
    }
    // SAFETY: the descriptor is valid; any returned ACL is freed exactly once.
    let acl = unsafe { acl_get_fd_np(dir.as_raw_fd(), ACL_TYPE_EXTENDED) };
    if !acl.is_null() {
        let _ = unsafe { acl_free(acl) };
        return Err(failure(4, "unsafe_control"));
    }
    if io::Error::last_os_error().raw_os_error() != Some(libc::ENOENT) {
        return Err(failure(4, "unsafe_control"));
    }
    Ok(())
}
fn walk(path: &Path) -> Result<(File, Vec<(u64, u64)>), Failure> {
    walk_with_policy(path, false)
}
fn walk_with_policy(path: &Path, trusted: bool) -> Result<(File, Vec<(u64, u64)>), Failure> {
    let raw = path.as_os_str().as_bytes();
    if !raw.starts_with(b"/") {
        return Err(failure(2, "usage"));
    }
    let root = CString::new("/").unwrap();
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
    let fd = unsafe { libc::open(root.as_ptr(), flags) };
    if fd < 0 {
        return Err(io_failure(io::Error::last_os_error()));
    }
    let mut dir = unsafe { File::from_raw_fd(fd) };
    if trusted {
        trusted_directory(&dir)?;
    }
    let mut chain = vec![ids(&dir).map_err(io_failure)?];
    if raw == b"/" {
        return Ok((dir, chain));
    }
    for part in raw[1..].split(|b| *b == b'/') {
        if part.is_empty() || part == b"." || part == b".." {
            return Err(failure(4, "unsupported_or_limit"));
        }
        let name = CString::new(part).map_err(|_| failure(4, "unsupported_or_limit"))?;
        let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(io_failure(io::Error::last_os_error()));
        }
        dir = unsafe { File::from_raw_fd(fd) };
        if trusted {
            trusted_directory(&dir)?;
        }
        chain.push(ids(&dir).map_err(io_failure)?);
    }
    Ok((dir, chain))
}
struct ControlDir {
    path: PathBuf,
    fd: File,
    chain: Vec<(u64, u64)>,
}
fn control(path: &str) -> Result<ControlDir, Failure> {
    let p = mac::source(path)?;
    let (dir, chain) = walk_with_policy(p, true)?;
    let m = dir.metadata().map_err(io_failure)?;
    if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err(failure(4, "unsafe_control"));
    }
    Ok(ControlDir {
        path: p.to_path_buf(),
        fd: dir,
        chain,
    })
}
fn disjoint(a: &[(u64, u64)], b: &[(u64, u64)]) -> bool {
    !a.contains(b.last().unwrap()) && !b.contains(a.last().unwrap())
}
fn validate_socket_path(path: &Path) -> Result<(), Failure> {
    let address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    let bytes = path.as_os_str().as_bytes();
    if bytes.contains(&0) || bytes.len() >= address.sun_path.len() {
        return Err(failure(4, "unsafe_control"));
    }
    Ok(())
}
fn socket_stat(dir: &File) -> Result<libc::stat, Failure> {
    let name = CString::new(SOCKET).unwrap();
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: dir is open, name is NUL-terminated, and stat points to writable storage.
    if unsafe {
        libc::fstatat(
            dir.as_raw_fd(),
            name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io_failure(io::Error::last_os_error()));
    }
    Ok(stat)
}
fn validate_socket(dir: &File) -> Result<(), Failure> {
    let m = socket_stat(dir)?;
    if m.st_mode & libc::S_IFMT != libc::S_IFSOCK
        || m.st_uid != unsafe { libc::geteuid() }
        || m.st_mode & 0o077 != 0
    {
        return Err(failure(4, "unsafe_control"));
    }
    Ok(())
}
struct SocketGuard {
    dir: File,
    socket_id: (u64, u64),
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        if let Ok(m) = socket_stat(&self.dir) {
            if m.st_mode & libc::S_IFMT == libc::S_IFSOCK
                && (m.st_dev as u64, m.st_ino) == self.socket_id
            {
                let name = CString::new(SOCKET).unwrap();
                // SAFETY: dir is a retained descriptor and name is a fixed NUL-terminated component.
                let _ = unsafe { libc::unlinkat(self.dir.as_raw_fd(), name.as_ptr(), 0) };
            }
        }
    }
}
// Called only on the main thread during host startup or a one-shot CLI client,
// before the host spawns worker/client threads. The process cwd is restored.
struct RestoreCwd(File);
impl Drop for RestoreCwd {
    fn drop(&mut self) {
        let _ = unsafe { libc::fchdir(self.0.as_raw_fd()) };
    }
}
fn in_control_dir<T>(
    dir: &File,
    action: impl FnOnce() -> Result<T, Failure>,
) -> Result<T, Failure> {
    let old = File::open(".").map_err(io_failure)?;
    // SAFETY: dir is an open directory descriptor; no concurrent process threads use cwd here.
    if unsafe { libc::fchdir(dir.as_raw_fd()) } != 0 {
        return Err(io_failure(io::Error::last_os_error()));
    }
    let guard = RestoreCwd(old);
    let result = action();
    // Fail closed if restoration fails, even when the operation succeeded.
    if unsafe { libc::fchdir(guard.0.as_raw_fd()) } != 0 {
        return Err(io_failure(io::Error::last_os_error()));
    }
    drop(guard);
    result
}
fn remaining(deadline: Instant) -> Result<Duration, Failure> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err(failure(6, "io_error"));
    }
    Ok(left)
}
fn read_frame(stream: &mut UnixStream, max: usize) -> Result<Vec<u8>, Failure> {
    let deadline = Instant::now() + TIMEOUT;
    let mut data = Vec::new();
    loop {
        stream
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(io_failure)?;
        let mut chunk = [0u8; 1024];
        let n = stream.read(&mut chunk).map_err(io_failure)?;
        if n == 0 {
            return Err(failure(4, "protocol_error"));
        }
        if data.len().saturating_add(n) > max {
            return Err(failure(4, "protocol_error"));
        }
        data.extend_from_slice(&chunk[..n]);
        if let Some(position) = data.iter().position(|b| *b == b'\n') {
            if position + 1 != data.len() {
                return Err(failure(4, "protocol_error"));
            }
            return Ok(data);
        }
    }
}
fn write_frame(stream: &mut UnixStream, data: &[u8]) -> Result<(), Failure> {
    let deadline = Instant::now() + TIMEOUT;
    let mut written = 0;
    while written < data.len() {
        stream
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(io_failure)?;
        let n = stream.write(&data[written..]).map_err(io_failure)?;
        if n == 0 {
            return Err(failure(6, "io_error"));
        }
        written += n;
    }
    Ok(())
}
fn response(stream: &mut UnixStream, code: i32, body: Value) {
    let mut bytes = serde_json::to_vec(&json!({"code":code,"body":body})).unwrap();
    if bytes.len() + 1 > RESPONSE_BYTES {
        bytes = serde_json::to_vec(&json!({"code":4,"body":{"error":"result_limit"}})).unwrap();
    }
    bytes.push(b'\n');
    let _ = write_frame(stream, &bytes);
}
fn peer(stream: &UnixStream) -> bool {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    unsafe {
        libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) == 0 && uid == libc::geteuid()
    }
}
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum TestMode {
    None,
    Pause,
    Panic,
    Indeterminate,
    PublicationPause,
}
#[cfg(feature = "test-hooks")]
#[derive(Default)]
struct GateData {
    paused: bool,
    release: bool,
    cancelled: bool,
}
#[cfg(feature = "test-hooks")]
#[derive(Default)]
struct TestGate {
    data: Mutex<GateData>,
    changed: std::sync::Condvar,
}
#[cfg(feature = "test-hooks")]
impl TestGate {
    fn reset(&self) {
        *self.data.lock().unwrap() = GateData::default();
    }
    fn pause(&self) {
        let mut g = self.data.lock().unwrap();
        g.paused = true;
        self.changed.notify_all();
        while !g.release && !g.cancelled {
            g = self.changed.wait(g).unwrap();
        }
        g.paused = false;
    }
    fn paused(&self) -> bool {
        self.data.lock().map(|g| g.paused).unwrap_or(false)
    }
    fn release(&self) -> bool {
        let mut g = self.data.lock().unwrap();
        if !g.paused {
            return false;
        }
        g.release = true;
        self.changed.notify_all();
        true
    }
    fn cancel(&self) {
        if let Ok(mut g) = self.data.lock() {
            g.cancelled = true;
            self.changed.notify_all();
        }
    }
}
struct PanicLatch(Arc<AtomicBool>);
impl Drop for PanicLatch {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.store(true, Ordering::Release);
        }
    }
}
struct Job {
    id: u64,
    state: &'static str,
    generation: Option<u64>,
    error: Option<&'static str>,
    token: Arc<CancellationToken>,
}
struct State {
    job: Option<Job>,
    next: u64,
    stopping: bool,
    fault: Option<&'static str>,
    workers: Vec<JoinHandle<()>>,
    panic_latch: Arc<AtomicBool>,
    #[cfg(feature = "test-hooks")]
    gate: Arc<TestGate>,
    #[cfg(feature = "test-hooks")]
    test_mode: TestMode,
}
impl State {
    fn new() -> Self {
        Self {
            job: None,
            next: 1,
            stopping: false,
            fault: None,
            workers: Vec::new(),
            panic_latch: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "test-hooks")]
            gate: Arc::new(TestGate::default()),
            #[cfg(feature = "test-hooks")]
            test_mode: match std::env::var("RUSTGROK_TEST_HOST_MODE").ok().as_deref() {
                Some("pause") => TestMode::Pause,
                Some("panic") => TestMode::Panic,
                Some("indeterminate") => TestMode::Indeterminate,
                Some("publication-pause") => TestMode::PublicationPause,
                _ => TestMode::None,
            },
        }
    }
    fn running(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.state == "running")
    }
}
fn expected(s: &str) -> Result<Option<u64>, Failure> {
    if s == "none" {
        return Ok(None);
    }
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(failure(2, "usage"));
    }
    let n = s.parse::<u64>().map_err(|_| failure(2, "usage"))?;
    if n == 0 {
        return Err(failure(2, "usage"));
    }
    Ok(Some(n))
}
fn status(state: &State) -> Value {
    let job = match &state.job {
        None => Value::Null,
        Some(j) => {
            let mut v = json!({"id":j.id,"state":j.state});
            #[cfg(feature = "test-hooks")]
            if j.state == "running" {
                v["paused"] = json!(state.gate.paused());
            }
            if let Some(n) = j.generation {
                v["generation"] = json!(n);
            }
            if let Some(e) = j.error {
                v["error"] = json!(e);
            }
            v
        }
    };
    let fault = state.fault.or_else(|| {
        state
            .panic_latch
            .load(Ordering::Acquire)
            .then_some("worker_panic")
    });
    json!({"operation":"host-status","job":job,"fault":fault})
}
enum Backend {
    Single(DurablePathIndex),
    Batch(DurableBatchIndex),
    Full(DurableFullIndex),
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchQuery {
    project: String,
    query: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchIndex {
    expected: String,
    projects: Vec<String>,
}

fn do_request(
    command: &str,
    value: Option<&str>,
    db: &Arc<Backend>,
    shared: &Arc<Mutex<State>>,
) -> Result<Value, Failure> {
    match command {
        "status" if value.is_none() => {
            let state = shared.lock().map_err(|_| failure(5, "host_fault"))?;
            Ok(status(&state))
        }
        "query" | "query-expression" | "batch-query" | "batch-query-expression" | "full-query" => {
            let raw = value.ok_or_else(|| failure(2, "usage"))?;
            {
                let s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
                if s.panic_latch.load(Ordering::Acquire) {
                    return Err(host_fault("worker_panic"));
                }
                if let Some(f) = s.fault {
                    return Err(host_fault(f));
                }
                if s.stopping {
                    return Err(failure(3, "busy"));
                }
            }
            let operation = match command {
                "query" => "host-query",
                "query-expression" => "host-query-expression",
                "batch-query" => "batch-host-query",
                "batch-query-expression" => "batch-host-query-expression",
                _ => "full-host-query",
            };
            let result = match (&**db, command) {
                (Backend::Single(db), "query" | "query-expression") => {
                    let snapshot = db.snapshot().map_err(mac::classify)?;
                    let paths = if command == "query-expression" {
                        snapshot.lookup_expression(raw)
                    } else {
                        snapshot.lookup(raw)
                    }
                    .map_err(|e| mac::classify(DurableError::Index(e)))?;
                    json!({"operation":operation,"generation":snapshot.generation(),"order":"unspecified","paths":paths})
                }
                (Backend::Batch(db), "batch-query" | "batch-query-expression") => {
                    let q: BatchQuery =
                        serde_json::from_str(raw).map_err(|_| failure(4, "protocol_error"))?;
                    mac::project(&q.project)?;
                    let snapshot = db.snapshot().map_err(mac::classify)?;
                    let paths = if command == "batch-query-expression" {
                        snapshot.lookup_expression(&q.project, &q.query)
                    } else {
                        snapshot.lookup(&q.project, &q.query)
                    }
                    .map_err(mac::classify)?;
                    json!({"operation":operation,"generation":snapshot.generation(),"order":"unspecified","paths":paths})
                }
                (Backend::Full(db), "full-query") => {
                    let q: BatchQuery =
                        serde_json::from_str(raw).map_err(|_| failure(4, "protocol_error"))?;
                    mac::project(&q.project)?;
                    let snapshot = db.snapshot().map_err(mac::full_error)?;
                    let matches=snapshot.lookup(&q.project,&q.query).map_err(mac::full_error)?.into_iter().map(|m| {
                        let positions=m.positions.into_iter().map(|p|json!({"position":p.position,"start_utf16":p.start_utf16,"end_utf16":p.end_utf16,"start_byte":p.start_byte,"end_byte":p.end_byte})).collect::<Vec<_>>();
                        json!({"path":m.path,"source_sha256":m.source_sha256,"positions":positions})
                    }).collect::<Vec<_>>();
                    // Recheck durable availability after lookup, including the failure-publication gap.
                    db.snapshot().map_err(mac::full_error)?;
                    json!({"operation":operation,"generation":snapshot.generation(),"order":"unspecified","matches":matches})
                }
                _ => return Err(failure(2, "usage")),
            };
            {
                let s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
                if s.panic_latch.load(Ordering::Acquire) {
                    return Err(host_fault("worker_panic"));
                }
                if let Some(f) = s.fault {
                    return Err(host_fault(f));
                }
            }
            Ok(result)
        }
        "index" | "batch-index" | "full-index" => {
            let raw = value.ok_or_else(|| failure(2, "usage"))?;
            let (expected, projects) = match (&**db, command) {
                (Backend::Single(_), "index") => (expected(raw)?, None),
                (Backend::Batch(_), "batch-index") | (Backend::Full(_), "full-index") => {
                    let request: BatchIndex =
                        serde_json::from_str(raw).map_err(|_| failure(4, "protocol_error"))?;
                    if request.projects.is_empty()
                        || request.projects.len() > mac::BATCH_LIMITS.max_projects
                    {
                        return Err(failure(4, "unsupported_or_limit"));
                    }
                    for name in &request.projects {
                        mac::project(name)?;
                    }
                    (expected(&request.expected)?, Some(request.projects))
                }
                _ => return Err(failure(2, "usage")),
            };
            let mut s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
            if s.panic_latch.load(Ordering::Acquire) {
                return Err(host_fault("worker_panic"));
            }
            if let Some(f) = s.fault {
                return Err(host_fault(f));
            }
            if s.stopping || s.running() {
                return Err(failure(3, "busy"));
            }
            let mut i = 0;
            while i < s.workers.len() {
                if s.workers[i].is_finished() {
                    let h = s.workers.swap_remove(i);
                    let _ = h.join();
                } else {
                    i += 1;
                }
            }
            if s.workers.len() >= 2 {
                return Err(failure(3, "busy"));
            }
            let id = s.next;
            s.next = s
                .next
                .checked_add(1)
                .ok_or_else(|| failure(4, "unsupported_or_limit"))?;
            let token = Arc::new(CancellationToken::new());
            s.job = Some(Job {
                id,
                state: "running",
                generation: None,
                error: None,
                token: Arc::clone(&token),
            });
            let worker_db = Arc::clone(db);
            let worker_state = Arc::clone(shared);
            let worker_panic_latch = Arc::clone(&s.panic_latch);
            #[cfg(feature = "test-hooks")]
            let (gate, mode) = {
                s.gate.reset();
                (Arc::clone(&s.gate), s.test_mode)
            };
            let handle = thread::Builder::new()
                .spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let _panic_latch = PanicLatch(worker_panic_latch);
                        match (&*worker_db, projects.as_deref()) {
                            (Backend::Single(db), None) => {
                                #[cfg(feature = "test-hooks")]
                                if expected.is_some() {
                                    if mode == TestMode::Pause {
                                        let mut observe = || gate.pause();
                                        return db.replace_from_source_with_precommit_observer(expected, &token, &mut observe).map_err(mac::classify);
                                    }
                                    if mode == TestMode::Panic {
                                        let mut observe = || panic!("injected worker panic");
                                        return db.replace_from_source_with_precommit_observer(expected, &token, &mut observe).map_err(mac::classify);
                                    }
                                    if mode == TestMode::Indeterminate {
                                        return db.replace_from_source_with_fault(expected, &token, rustgrok_path_index::durable::FaultPoint::FailDirectorySync).map_err(mac::classify);
                                    }
                                }
                                db.replace_from_source(expected, &token).map_err(mac::classify)
                            }
                            (Backend::Batch(db), Some(projects)) => {
                                #[cfg(feature = "test-hooks")]
                                if expected.is_some() {
                                    if mode == TestMode::Pause {
                                        let mut observe = || gate.pause();
                                        return db.replace_from_source_with_second_project_observer(expected, projects, &token, &mut observe).map_err(mac::classify);
                                    }
                                    if mode == TestMode::Panic {
                                        let mut observe = || panic!("injected batch worker panic");
                                        return db.replace_from_source_with_precommit_observer(expected, projects, &token, &mut observe).map_err(mac::classify);
                                    }
                                    if mode == TestMode::Indeterminate {
                                        return db.replace_from_source_with_fault(expected, projects, &token, rustgrok_path_index::durable::FaultPoint::FailDirectorySync).map_err(mac::classify);
                                    }
                                }
                                db.replace_from_source(expected, projects, &token).map_err(mac::classify)
                            }
                            (Backend::Full(db),Some(projects)) => {
                                #[cfg(feature="test-hooks")]
                                if expected.is_some() {
                                    if mode==TestMode::PublicationPause {
                                        return db.replace_with_publication_observer(expected,projects,&token,&mut ||gate.pause()).map_err(mac::full_error);
                                    }
                                    if mode==TestMode::Pause {
                                        return db.replace_with_observer(expected,projects,&token,&mut ||gate.pause(),&mut ||{}).map_err(mac::full_error);
                                    }
                                    if mode==TestMode::Panic {
                                        return db.replace_with_observer(expected,projects,&token,&mut ||{},&mut ||panic!("injected FULL worker panic")).map_err(mac::full_error);
                                    }
                                    if mode==TestMode::Indeterminate {
                                        return db.replace_with_fault(expected,projects,&token,rustgrok_path_index::durable::FaultPoint::FailDirectorySync).map_err(mac::full_error);
                                    }
                                }
                                db.replace_from_source(expected,projects,&token).map_err(mac::full_error)
                            }
                            _ => Err(mac::classify(DurableError::Index(IndexError::InvalidProject))),
                        }
                    }));
                    #[cfg(feature = "test-hooks")]
                    if result.is_err()
                        && std::env::var_os("RUSTGROK_TEST_PANIC_GAP").as_deref()
                            == Some(std::ffi::OsStr::new("1"))
                    {
                        gate.pause();
                    }
                    if let Ok(mut s) = worker_state.lock() {
                        match result {
                            Ok(Ok(g)) => {
                                if let Some(j) = s.job.as_mut() {
                                    j.state = "succeeded";
                                    j.generation = Some(g);
                                }
                            }
                            Ok(Err(e)) => {
                                if e.label == "publication_indeterminate" {
                                    s.fault = Some("publication_indeterminate");
                                }
                                if let Some(j) = s.job.as_mut() {
                                    j.state = if e.label == "cancelled" {
                                        "cancelled"
                                    } else {
                                        "failed"
                                    };
                                    j.error = Some(e.label);
                                }
                            }
                            Err(_) => {
                                s.fault = Some("worker_panic");
                                if let Some(j) = s.job.as_mut() {
                                    j.state = "failed";
                                    j.error = Some("worker_panic");
                                }
                            }
                        }
                    }
                })
                .map_err(|_| failure(6, "io_error"));
            match handle {
                Ok(h) => s.workers.push(h),
                Err(e) => {
                    if let Some(j) = s.job.as_mut() {
                        j.state = "failed";
                        j.error = Some("io_error");
                    }
                    return Err(e);
                }
            }
            Ok(
                json!({"operation":if command == "batch-index" {"batch-host-index"} else if command == "full-index" {"full-host-index"} else {"host-index"},"job":id,"state":"running"}),
            )
        }
        "cancel" if value.is_none() => {
            let s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
            let j = s
                .job
                .as_ref()
                .filter(|j| j.state == "running")
                .ok_or_else(|| failure(4, "no_running_job"))?;
            let accepted = j.token.cancel();
            #[cfg(feature = "test-hooks")]
            if accepted {
                s.gate.cancel();
            }
            Ok(json!({"operation":"host-cancel","accepted":accepted}))
        }
        #[cfg(feature = "test-hooks")]
        "test-release" if value.is_none() => {
            let s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
            if !s.gate.release() {
                return Err(failure(4, "no_paused_job"));
            }
            Ok(json!({"operation":"host-test-release","state":"released"}))
        }
        "stop" if value.is_none() => {
            let mut s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
            if s.running() {
                if let Some(j) = &s.job {
                    j.token.cancel();
                    #[cfg(feature = "test-hooks")]
                    s.gate.cancel();
                }
                return Err(failure(3, "busy"));
            }
            s.stopping = true;
            Ok(json!({"operation":"host-stop","state":"stopping"}))
        }
        _ => Err(failure(2, "usage")),
    }
}
struct Permit(Arc<AtomicUsize>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
fn handle(mut stream: UnixStream, db: Arc<Backend>, state: Arc<Mutex<State>>, permit: Permit) {
    let _permit = permit;
    if stream.set_nonblocking(false).is_err() {
        return;
    }
    let _ = stream.set_read_timeout(Some(TIMEOUT));
    let _ = stream.set_write_timeout(Some(TIMEOUT));
    if !peer(&stream) {
        response(&mut stream, 4, json!({"error":"unsafe_control"}));
        return;
    }
    let data = match read_frame(&mut stream, REQUEST_BYTES) {
        Ok(d) => d,
        Err(e) => {
            response(&mut stream, e.code, json!({"error":e.label}));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        command: String,
        #[serde(default)]
        value: Option<String>,
    }
    let request: Request = match serde_json::from_slice(&data) {
        Ok(v) => v,
        Err(_) => {
            response(&mut stream, 4, json!({"error":"protocol_error"}));
            return;
        }
    };
    let command = request.command.as_str();
    let value = request.value.as_deref();
    match do_request(command, value, &db, &state) {
        Ok(v) => response(&mut stream, 0, v),
        Err(e) => response(&mut stream, e.code, json!({"error":e.label})),
    }
}
pub(crate) fn serve(
    store: &str,
    src: &str,
    proj: &str,
    control_dir: &str,
) -> Result<Value, Failure> {
    serve_impl(store, src, StoreMode::Single(proj), control_dir)
}
pub(crate) fn serve_batch(store: &str, src: &str, control_dir: &str) -> Result<Value, Failure> {
    serve_impl(store, src, StoreMode::Batch, control_dir)
}
pub(crate) fn serve_full(store: &str, src: &str, control_dir: &str) -> Result<Value, Failure> {
    serve_impl(store, src, StoreMode::Full, control_dir)
}
enum StoreMode<'a> {
    Single(&'a str),
    Batch,
    Full,
}
fn serve_impl(
    store: &str,
    src: &str,
    mode: StoreMode<'_>,
    control_dir: &str,
) -> Result<Value, Failure> {
    let ControlDir {
        path: dir,
        fd: dir_fd,
        chain: control_chain,
    } = control(control_dir)?;
    let (_, src_chain) = walk(mac::source(src)?)?;
    let (_, store_chain) = walk(mac::source(store)?)?;
    if !disjoint(&control_chain, &src_chain) || !disjoint(&control_chain, &store_chain) {
        return Err(failure(4, "unsafe_control"));
    }
    let path = dir.join(SOCKET);
    validate_socket_path(&path)?;
    let (operation, backend) = match mode {
        StoreMode::Single(name) => ("host", Backend::Single(mac::open(store, src, name)?)),
        StoreMode::Batch => ("batch-host", Backend::Batch(mac::open_batch(store, src)?)),
        StoreMode::Full => (
            "full-host",
            Backend::Full(
                DurableFullIndex::open(
                    mac::source(store)?,
                    mac::source(src)?,
                    FullLimits::default(),
                )
                .map_err(mac::full_error)?,
            ),
        ),
    };
    let db = Arc::new(backend);
    let published_result = match &*db {
        Backend::Single(db) => db.snapshot().map(|_| ()).map_err(mac::classify),
        Backend::Batch(db) => db.snapshot().map(|_| ()).map_err(mac::classify),
        Backend::Full(db) => db.snapshot().map(|_| ()).map_err(mac::full_error),
    };
    let published = match published_result {
        Ok(()) => true,
        Err(e) if e.label == "no_generation" => false,
        Err(e) => return Err(e),
    };
    #[cfg(feature = "test-hooks")]
    if std::env::var_os("RUSTGROK_TEST_BEFORE_BIND").as_deref() == Some(std::ffi::OsStr::new("1")) {
        eprintln!("RUSTGROK_TEST_BEFORE_BIND");
        let mut byte = [0u8; 1];
        io::stdin().read_exact(&mut byte).map_err(io_failure)?;
    }
    let (listener, _guard) = in_control_dir(&dir_fd, || {
        let listener = UnixListener::bind(SOCKET).map_err(io_failure)?;
        let stat = socket_stat(&dir_fd)?;
        let guard = SocketGuard {
            dir: dir_fd.try_clone().map_err(io_failure)?,
            socket_id: (stat.st_dev as u64, stat.st_ino),
        };
        fs::set_permissions(SOCKET, fs::Permissions::from_mode(0o600)).map_err(io_failure)?;
        let after = socket_stat(&dir_fd)?;
        if after.st_mode & libc::S_IFMT != libc::S_IFSOCK
            || (after.st_dev as u64, after.st_ino) != guard.socket_id
        {
            return Err(failure(4, "unsafe_control"));
        }
        Ok((listener, guard))
    })?;
    listener.set_nonblocking(true).map_err(io_failure)?;
    STOP_SIGNAL.store(false, Ordering::Release);
    for signal in [libc::SIGINT, libc::SIGTERM] {
        if unsafe { libc::signal(signal, stop_signal as *const () as libc::sighandler_t) }
            == libc::SIG_ERR
        {
            return Err(failure(6, "io_error"));
        }
    }
    println!(
        "{}",
        json!({"operation":operation,"state":"listening","published":published})
    );
    io::stdout().flush().map_err(io_failure)?;
    let state = Arc::new(Mutex::new(State::new()));
    let active = Arc::new(AtomicUsize::new(0));
    let mut clients: Vec<JoinHandle<()>> = Vec::new();
    let mut fatal_error: Option<Failure> = None;
    loop {
        let (stopping, running) = {
            let mut s = state.lock().map_err(|_| failure(5, "host_fault"))?;
            if STOP_SIGNAL.load(Ordering::Acquire) {
                s.stopping = true;
                if let Some(j) = &s.job {
                    if j.state == "running" {
                        j.token.cancel();
                        #[cfg(feature = "test-hooks")]
                        s.gate.cancel();
                    }
                }
            }
            (s.stopping, s.running())
        };
        if stopping && !running {
            break;
        }
        if fatal_error.is_some() {
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                if stopping
                    || active.load(Ordering::Acquire) >= MAX_CLIENTS
                    || clients.len() >= MAX_CLIENTS
                {
                    let _ = stream.set_write_timeout(Some(TIMEOUT));
                    response(&mut stream, 3, json!({"error":"busy"}));
                } else {
                    active.fetch_add(1, Ordering::AcqRel);
                    let permit = Permit(Arc::clone(&active));
                    let d = Arc::clone(&db);
                    let s = Arc::clone(&state);
                    if let Ok(h) =
                        thread::Builder::new().spawn(move || handle(stream, d, s, permit))
                    {
                        clients.push(h);
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10))
            }
            Err(e) => {
                fatal_error = Some(io_failure(e));
                STOP_SIGNAL.store(true, Ordering::Release);
            }
        }
        let mut i = 0;
        while i < clients.len() {
            if clients[i].is_finished() {
                let h = clients.swap_remove(i);
                let _ = h.join();
            } else {
                i += 1;
            }
        }
    }
    drop(listener);
    for h in clients {
        let _ = h.join();
    }
    let workers = {
        let mut s = state.lock().map_err(|_| failure(5, "host_fault"))?;
        std::mem::take(&mut s.workers)
    };
    for h in workers {
        let _ = h.join();
    }
    if let Some(error) = fatal_error {
        return Err(error);
    }
    Ok(json!({"operation":operation,"state":"stopped"}))
}
fn connect_bounded(path: &Path) -> Result<UnixStream, Failure> {
    let bytes = path.as_os_str().as_bytes();
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.contains(&0) || bytes.len() >= address.sun_path.len() {
        return Err(failure(4, "unsafe_control"));
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    address.sun_len =
        u8::try_from(2 + bytes.len() + 1).map_err(|_| failure(4, "unsafe_control"))?;
    for (dst, src) in address.sun_path.iter_mut().zip(bytes) {
        *dst = *src as libc::c_char;
    }
    // SAFETY: successful socket yields an owned descriptor, retained by File until converted.
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(io_failure(io::Error::last_os_error()));
    }
    let owned = unsafe { File::from_raw_fd(fd) };
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0
        || unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } < 0
    {
        return Err(io_failure(io::Error::last_os_error()));
    }
    let rc = unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_un).cast(),
            address.sun_len as libc::socklen_t,
        )
    };
    if rc != 0 {
        let e = io::Error::last_os_error();
        if !matches!(
            e.raw_os_error(),
            Some(libc::EINPROGRESS) | Some(libc::EAGAIN) | Some(libc::EALREADY)
        ) {
            return Err(io_failure(e));
        }
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let left = remaining(deadline)?;
            let ms = (left.as_millis().saturating_add(1)).min(i32::MAX as u128) as i32;
            let mut pollfd = libc::pollfd {
                fd,
                events: libc::POLLOUT,
                revents: 0,
            };
            let rc = unsafe { libc::poll(&mut pollfd, 1, ms) };
            if rc == 0 {
                return Err(failure(6, "io_error"));
            }
            if rc < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(io_failure(e));
            }
            let mut error: libc::c_int = 0;
            let mut len = std::mem::size_of_val(&error) as libc::socklen_t;
            if unsafe {
                libc::getsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_ERROR,
                    (&mut error as *mut libc::c_int).cast(),
                    &mut len,
                )
            } < 0
            {
                return Err(io_failure(io::Error::last_os_error()));
            }
            if error != 0 {
                return Err(io_failure(io::Error::from_raw_os_error(error)));
            }
            break;
        }
    }
    // SAFETY: transfer ownership exactly once to the UnixStream.
    let stream = unsafe { UnixStream::from_raw_fd(owned.into_raw_fd()) };
    stream.set_nonblocking(false).map_err(io_failure)?;
    Ok(stream)
}
pub(crate) fn client(
    control_dir: &str,
    command: &str,
    value: Option<&str>,
) -> Result<Value, Failure> {
    let ControlDir {
        path: dir,
        fd: dir_fd,
        ..
    } = control(control_dir)?;
    validate_socket_path(&dir.join(SOCKET))?;
    validate_socket(&dir_fd)?;
    let mut stream = in_control_dir(&dir_fd, || connect_bounded(Path::new(SOCKET)))?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(io_failure)?;
    stream
        .set_write_timeout(Some(TIMEOUT))
        .map_err(io_failure)?;
    if !peer(&stream) {
        return Err(failure(4, "unsafe_control"));
    }
    let mut data = serde_json::to_vec(&json!({"command":command,"value":value}))
        .map_err(|_| failure(4, "protocol_error"))?;
    if value.is_none() {
        data = serde_json::to_vec(&json!({"command":command})).unwrap();
    }
    if data.len() + 1 > REQUEST_BYTES {
        return Err(failure(4, "protocol_error"));
    }
    data.push(b'\n');
    write_frame(&mut stream, &data)?;
    let raw = read_frame(&mut stream, RESPONSE_BYTES)?;
    let reply: Value = serde_json::from_slice(&raw).map_err(|_| failure(4, "protocol_error"))?;
    let code = reply
        .get("code")
        .and_then(Value::as_i64)
        .ok_or_else(|| failure(4, "protocol_error"))?;
    let body = reply
        .get("body")
        .ok_or_else(|| failure(4, "protocol_error"))?;
    if code == 0 {
        return Ok(body.clone());
    }
    let label = body
        .get("error")
        .and_then(Value::as_str)
        .ok_or_else(|| failure(4, "protocol_error"))?;
    let label = match label {
        "busy" => "busy",
        "unsafe_control" => "unsafe_control",
        "no_running_job" => "no_running_job",
        "no_paused_job" => "no_paused_job",
        "host_fault" => "host_fault",
        "worker_panic" => "worker_panic",
        "publication_indeterminate" => "publication_indeterminate",
        "no_generation" => "no_generation",
        "unsupported_or_limit" => "unsupported_or_limit",
        "unsupported_full_input" => "unsupported_full_input",
        "unsupported_encoding" => "unsupported_encoding",
        "unsupported_reader" => "unsupported_reader",
        "unsupported_query" => "unsupported_query",
        "stale_generation" => "stale_generation",
        "source_changed" => "source_changed",
        "corrupt_store" => "corrupt_store",
        "io_error" => "io_error",
        "usage" => "usage",
        "protocol_error" => "protocol_error",
        "result_limit" => "result_limit",
        _ => "protocol_error",
    };
    Err(failure(i32::try_from(code).unwrap_or(4), label))
}
