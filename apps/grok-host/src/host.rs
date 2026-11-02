// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
//! Private, bounded, single-project development control adapter.
use crate::mac::{self, Failure};
use rustgrok_path_index::durable::{DurableError, DurablePathIndex};
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
            fs::{FileTypeExt, MetadataExt, PermissionsExt},
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
fn walk(path: &Path) -> Result<(File, Vec<(u64, u64)>), Failure> {
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
        chain.push(ids(&dir).map_err(io_failure)?);
    }
    Ok((dir, chain))
}
fn control(path: &str) -> Result<(PathBuf, (u64, u64)), Failure> {
    let p = mac::source(path)?;
    let (dir, chain) = walk(p)?;
    let m = dir.metadata().map_err(io_failure)?;
    if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err(failure(4, "unsafe_control"));
    }
    Ok((p.to_path_buf(), *chain.last().unwrap()))
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
fn validate_socket(dir: &Path) -> Result<PathBuf, Failure> {
    let socket = dir.join(SOCKET);
    validate_socket_path(&socket)?;
    let m = fs::symlink_metadata(&socket).map_err(io_failure)?;
    if !m.file_type().is_socket() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0
    {
        return Err(failure(4, "unsafe_control"));
    }
    Ok(socket)
}
struct SocketGuard {
    dir: PathBuf,
    dir_id: (u64, u64),
    socket_id: (u64, u64),
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        if let Ok((d, _)) = walk(&self.dir) {
            if ids(&d).ok() == Some(self.dir_id) {
                let p = self.dir.join(SOCKET);
                if let Ok(m) = fs::symlink_metadata(&p) {
                    if m.file_type().is_socket() && (m.dev(), m.ino()) == self.socket_id {
                        let _ = fs::remove_file(p);
                    }
                }
            }
        }
    }
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
            #[cfg(feature = "test-hooks")]
            gate: Arc::new(TestGate::default()),
            #[cfg(feature = "test-hooks")]
            test_mode: match std::env::var("RUSTGROK_TEST_HOST_MODE").ok().as_deref() {
                Some("pause") => TestMode::Pause,
                Some("panic") => TestMode::Panic,
                Some("indeterminate") => TestMode::Indeterminate,
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
    json!({"operation":"host-status","job":job,"fault":state.fault})
}
fn do_request(
    command: &str,
    value: Option<&str>,
    db: &Arc<DurablePathIndex>,
    shared: &Arc<Mutex<State>>,
) -> Result<Value, Failure> {
    match command {
        "status" if value.is_none() => {
            let state = shared.lock().map_err(|_| failure(5, "host_fault"))?;
            Ok(status(&state))
        }
        "query" => {
            let term = value.ok_or_else(|| failure(2, "usage"))?;
            {
                let s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
                if let Some(f) = s.fault {
                    return Err(host_fault(f));
                }
                if s.stopping {
                    return Err(failure(3, "busy"));
                }
            }
            let snapshot = db.snapshot().map_err(mac::classify)?;
            let paths = snapshot
                .lookup(term)
                .map_err(|e| mac::classify(DurableError::Index(e)))?;
            {
                let s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
                if let Some(f) = s.fault {
                    return Err(host_fault(f));
                }
            }
            Ok(
                json!({"operation":"host-query","generation":snapshot.generation(),"order":"unspecified","paths":paths}),
            )
        }
        "index" => {
            let expected = expected(value.ok_or_else(|| failure(2, "usage"))?)?;
            let mut s = shared.lock().map_err(|_| failure(5, "host_fault"))?;
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
            #[cfg(feature = "test-hooks")]
            let (gate, mode) = {
                s.gate.reset();
                (Arc::clone(&s.gate), s.test_mode)
            };
            let handle = thread::Builder::new()
                .spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #[cfg(feature = "test-hooks")]
                        if expected.is_some() {
                            if mode == TestMode::Pause {
                                let mut observe = || gate.pause();
                                return worker_db.replace_from_source_with_precommit_observer(
                                    expected,
                                    &token,
                                    &mut observe,
                                );
                            }
                            if mode == TestMode::Panic {
                                let mut observe = || panic!("injected worker panic");
                                return worker_db.replace_from_source_with_precommit_observer(
                                    expected,
                                    &token,
                                    &mut observe,
                                );
                            }
                            if mode == TestMode::Indeterminate {
                                return worker_db.replace_from_source_with_fault(
                                    expected,
                                    &token,
                                    rustgrok_path_index::durable::FaultPoint::FailDirectorySync,
                                );
                            }
                        }
                        worker_db.replace_from_source(expected, &token)
                    }));
                    if let Ok(mut s) = worker_state.lock() {
                        match result {
                            Ok(Ok(g)) => {
                                if let Some(j) = s.job.as_mut() {
                                    j.state = "succeeded";
                                    j.generation = Some(g);
                                }
                            }
                            Ok(Err(e)) => {
                                let e = mac::classify(e);
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
            Ok(json!({"operation":"host-index","job":id,"state":"running"}))
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
fn handle(
    mut stream: UnixStream,
    db: Arc<DurablePathIndex>,
    state: Arc<Mutex<State>>,
    permit: Permit,
) {
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
    let request: Value = match serde_json::from_slice(&data) {
        Ok(v) => v,
        Err(_) => {
            response(&mut stream, 4, json!({"error":"protocol_error"}));
            return;
        }
    };
    let Some(obj) = request.as_object() else {
        response(&mut stream, 4, json!({"error":"protocol_error"}));
        return;
    };
    if obj.keys().any(|k| k != "command" && k != "value")
        || !obj.get("command").is_some_and(Value::is_string)
        || obj.get("value").is_some_and(|v| !v.is_string())
    {
        response(&mut stream, 4, json!({"error":"protocol_error"}));
        return;
    }
    let command = obj["command"].as_str().unwrap();
    let value = obj.get("value").and_then(Value::as_str);
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
    let (dir, dir_id) = control(control_dir)?;
    let (_, control_chain) = walk(&dir)?;
    let (_, src_chain) = walk(mac::source(src)?)?;
    let (_, store_chain) = walk(mac::source(store)?)?;
    if !disjoint(&control_chain, &src_chain) || !disjoint(&control_chain, &store_chain) {
        return Err(failure(4, "unsafe_control"));
    }
    let path = dir.join(SOCKET);
    validate_socket_path(&path)?;
    let db = Arc::new(mac::open(store, src, proj)?);
    let published = match db.snapshot() {
        Ok(_) => true,
        Err(DurableError::Index(IndexError::NoPublishedGeneration)) => false,
        Err(e) => return Err(mac::classify(e)),
    };
    let listener = UnixListener::bind(&path).map_err(io_failure)?;
    let m = fs::symlink_metadata(&path).map_err(io_failure)?;
    let _guard = SocketGuard {
        dir,
        dir_id,
        socket_id: (m.dev(), m.ino()),
    };
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(io_failure)?;
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
        json!({"operation":"host","state":"listening","published":published})
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
    Ok(json!({"operation":"host","state":"stopped"}))
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
    let (dir, _) = control(control_dir)?;
    let socket = validate_socket(&dir)?;
    let mut stream = connect_bounded(&socket)?;
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
