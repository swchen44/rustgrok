// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
#![cfg(target_os = "macos")]
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    source: PathBuf,
    parent: PathBuf,
    store: PathBuf,
    control: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "rg-host-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let parent = root.join("indexes");
        let control = root.join("control");
        for p in [&source, &parent, &control] {
            fs::create_dir(p).unwrap();
        }
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&control, fs::Permissions::from_mode(0o700)).unwrap();
        let store = parent.join("store");
        Self {
            root,
            source,
            parent,
            store,
            control,
        }
    }
    fn s(&self) -> &str {
        self.source.to_str().unwrap()
    }
    fn p(&self) -> &str {
        self.parent.to_str().unwrap()
    }
    fn t(&self) -> &str {
        self.store.to_str().unwrap()
    }
    fn c(&self) -> &str {
        self.control.to_str().unwrap()
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rustgrok"))
            .args(args)
            .output()
            .unwrap()
    }
    fn initialize(&self) {
        fs::write(self.source.join("main.c"), b"main\n").unwrap();
        success(self.run(&["create", self.p(), "store", self.s(), "alpha"]));
    }
    fn start(&self, paused: bool) -> Host {
        Host::start(self, paused, false)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "exit {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
// Own the child from spawn until Host takes over; Child::drop alone does not kill it.
struct StartupChild(Option<Child>);
impl StartupChild {
    fn child(&mut self) -> &mut Child {
        self.0.as_mut().expect("startup child")
    }
    fn terminate(&mut self) -> (bool, String) {
        let child = self.child();
        match child.try_wait() {
            Ok(Some(status)) => (true, format!("already exited: {status}")),
            Ok(None) => match child.kill() {
                Ok(()) => match child.wait() {
                    Ok(status) => (true, format!("killed and reaped: {status}")),
                    Err(error) => (false, format!("wait failed after kill: {error}")),
                },
                Err(error) => (false, format!("kill failed: {error}")),
            },
            Err(error) => (false, format!("try_wait failed: {error}")),
        }
    }
    fn into_child(mut self) -> Child {
        self.0.take().expect("startup child")
    }
}
impl Drop for StartupChild {
    fn drop(&mut self) {
        if self.0.is_some() {
            let _ = self.terminate();
        }
    }
}
struct Host {
    child: Child,
}
impl Host {
    fn start(f: &Fixture, paused: bool, published: bool) -> Self {
        Self::start_mode(f, if paused { "pause" } else { "none" }, published)
    }
    fn start_mode(f: &Fixture, mode: &str, published: bool) -> Self {
        Self::start_mode_with_spawn_hook(f, mode, published, |_| {})
    }
    fn start_mode_with_spawn_hook(
        f: &Fixture,
        mode: &str,
        published: bool,
        on_spawn: impl FnOnce(&mut Child),
    ) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustgrok"));
        cmd.args(["host", f.t(), f.s(), "alpha", f.c()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if mode == "panic-gap" {
            cmd.env("RUSTGROK_TEST_HOST_MODE", "panic")
                .env("RUSTGROK_TEST_PANIC_GAP", "1");
        } else if mode != "none" {
            cmd.env("RUSTGROK_TEST_HOST_MODE", mode);
        }
        #[cfg(not(feature = "test-hooks"))]
        if mode == "panic" {
            cmd.env("RUSTGROK_TEST_BEFORE_BIND", "1")
                .env("RUSTGROK_TEST_PANIC_GAP", "1");
        }
        let mut startup = StartupChild(Some(cmd.spawn().unwrap()));
        on_spawn(startup.child());
        let stdout = startup.child().stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let _ = tx.send(line);
            for line in reader.lines() {
                if line.is_err() {
                    break;
                }
            }
        });
        let first = rx.recv_timeout(Duration::from_secs(5));
        if first
            .as_ref()
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            != Some(json!({"operation":"host","state":"listening","published":published}))
        {
            let (reaped, cleanup) = startup.terminate();
            let mut stderr = String::new();
            if reaped {
                if let Some(mut pipe) = startup.child().stderr.take() {
                    use std::io::Read;
                    let _ = pipe.read_to_string(&mut stderr);
                }
            } else {
                stderr.push_str("not read because child was not reaped");
            }
            panic!(
                "host startup: {first:?}, stderr: {stderr}, cleanup: {cleanup}, control: {:?}",
                f.control
            );
        }
        Self {
            child: startup.into_child(),
        }
    }
    fn wait(&mut self) {
        assert!(self.child.wait().unwrap().success());
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            if self.child.kill().is_ok() {
                let _ = self.child.wait();
            }
        }
    }
}
fn wait_job(f: &Fixture, id: u64, state: &str) -> Value {
    for _ in 0..250 {
        let v = success(f.run(&["host-status", f.c()]));
        if v["job"]["id"] == id && v["job"]["state"] == state {
            return v;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("job {id} did not become {state}")
}
#[test]
fn host_build_query_stop_and_restart() {
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(false);
    let job = success(f.run(&["host-index", f.c(), "none"]));
    assert_eq!(
        job,
        json!({"operation":"host-index","job":1,"state":"running"})
    );
    assert_eq!(wait_job(&f, 1, "succeeded")["job"]["generation"], 1);
    assert_eq!(
        success(f.run(&["host-query", f.c(), "main"])),
        json!({"operation":"host-query","generation":1,"order":"unspecified","paths":["/alpha/main.c"]})
    );
    assert_eq!(
        success(f.run(&["host-stop", f.c()])),
        json!({"operation":"host-stop","state":"stopping"})
    );
    host.wait();
    let direct = success(f.run(&["query", f.t(), f.s(), "alpha", "main"]));
    assert_eq!(direct["generation"], 1);
}

#[test]
fn trickle_request_has_a_whole_frame_deadline() {
    use std::{
        io::{Read, Write},
        net::Shutdown,
        os::unix::net::UnixStream,
    };
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(false);
    let stream = UnixStream::connect(f.control.join("control.sock")).unwrap();
    let mut writer = stream.try_clone().unwrap();
    let send = thread::spawn(move || {
        for _ in 0..50 {
            if writer.write_all(b"x").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }
    });
    let mut reader = stream.try_clone().unwrap();
    let (tx, rx) = mpsc::channel();
    let receive = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        let _ = tx.send(bytes);
    });
    let result = rx.recv_timeout(Duration::from_millis(4500));
    let _ = stream.shutdown(Shutdown::Both);
    let _ = send.join();
    let _ = receive.join();
    let bytes = result.expect("server allowed a slow trickle beyond the whole-frame deadline");
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_ne!(value["code"], 0);
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[test]
fn fragmented_valid_request_is_accepted_before_deadline() {
    use std::{
        io::{Read, Write},
        os::unix::net::UnixStream,
    };
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(false);
    let mut stream = UnixStream::connect(f.control.join("control.sock")).unwrap();
    stream.write_all(b"{\"command\":").unwrap();
    thread::sleep(Duration::from_millis(200));
    stream.write_all(b"\"status\"}\n").unwrap();
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["code"], 0);
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn queries_keep_old_generation_during_build_then_switch_and_restart() {
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(true);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    assert_eq!(success(f.run(&["host-index", f.c(), "1"]))["job"], 2);
    let mut paused = false;
    for _ in 0..250 {
        let s = success(f.run(&["host-status", f.c()]));
        if s["job"]["id"] == 2 && s["job"]["paused"] == true {
            paused = true;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(paused, "build never paused");
    assert_eq!(
        success(f.run(&["host-query", f.c(), "extra"])),
        json!({"operation":"host-query","generation":1,"order":"unspecified","paths":[]})
    );
    assert_eq!(
        success(f.run(&["host-query-expression", f.c(), "alpha AND extra"])),
        json!({"operation":"host-query-expression","generation":1,"order":"unspecified","paths":[]})
    );
    for op in ["AND", "OR", "NOT"] {
        let input = format!("alpha {op}\u{c}extra");
        error(
            f.run(&["host-query-expression", f.c(), &input]),
            4,
            "unsupported_or_limit",
        );
    }
    assert_eq!(
        success(f.run(&["host-query-expression", f.c(), "alpha AND main"])),
        json!({"operation":"host-query-expression","generation":1,"order":"unspecified","paths":["/alpha/main.c"]})
    );
    let busy = f.run(&["query", f.t(), f.s(), "alpha", "main"]);
    assert_eq!(busy.status.code(), Some(3));
    let second = f.run(&["host-index", f.c(), "1"]);
    assert_eq!(second.status.code(), Some(3));
    success(f.run(&["host-test-release", f.c()]));
    assert_eq!(wait_job(&f, 2, "succeeded")["job"]["generation"], 2);
    assert_eq!(
        success(f.run(&["host-query", f.c(), "extra"]))["paths"],
        json!(["/alpha/extra.js"])
    );
    assert_eq!(
        success(f.run(&["host-query-expression", f.c(), "alpha AND extra"])),
        json!({"operation":"host-query-expression","generation":2,"order":"unspecified","paths":["/alpha/extra.js"]})
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
    let mut host = Host::start(&f, false, true);
    assert_eq!(
        success(f.run(&["host-query", f.c(), "extra"]))["generation"],
        2
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
    assert_eq!(fs::read(f.source.join("main.c")).unwrap(), b"main\n");
    assert_eq!(fs::read(f.source.join("extra.js")).unwrap(), b"extra\n");
}

#[cfg(feature = "test-hooks")]
fn wait_paused(f: &Fixture, id: u64) {
    for _ in 0..250 {
        let s = success(f.run(&["host-status", f.c()]));
        if s["job"]["id"] == id && s["job"]["paused"] == true {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("job did not pause");
}
fn error(output: Output, code: i32, label: &str) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap(),
        json!({"error":label})
    );
}

#[cfg(feature = "test-hooks")]
#[test]
fn cancelled_failed_and_stale_jobs_keep_previous_generation() {
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(true);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    wait_paused(&f, 2);
    assert_eq!(
        success(f.run(&["host-cancel", f.c()])),
        json!({"operation":"host-cancel","accepted":true})
    );
    assert_eq!(wait_job(&f, 2, "cancelled")["job"]["error"], "cancelled");
    assert_eq!(
        success(f.run(&["host-query", f.c(), "extra"]))["generation"],
        1
    );
    assert_eq!(
        success(f.run(&["host-query-expression", f.c(), "alpha AND extra"])),
        json!({"operation":"host-query-expression","generation":1,"order":"unspecified","paths":[]})
    );
    fs::write(f.source.join("bad.txt"), b"unsupported\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    assert_eq!(
        wait_job(&f, 3, "failed")["job"]["error"],
        "unsupported_or_limit"
    );
    fs::remove_file(f.source.join("bad.txt")).unwrap();
    success(f.run(&["host-index", f.c(), "99"]));
    assert_eq!(
        wait_job(&f, 4, "failed")["job"]["error"],
        "stale_generation"
    );
    assert_eq!(
        success(f.run(&["host-query", f.c(), "extra"]))["paths"],
        json!([])
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
    let mut host = Host::start(&f, false, true);
    assert_eq!(
        success(f.run(&["host-query", f.c(), "main"]))["generation"],
        1
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn worker_panic_blocks_requests_until_restart() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start_mode(&f, "panic", false);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    assert_eq!(wait_job(&f, 2, "failed")["job"]["error"], "worker_panic");
    error(f.run(&["host-query", f.c(), "main"]), 5, "worker_panic");
    error(f.run(&["host-index", f.c(), "1"]), 5, "worker_panic");
    success(f.run(&["host-stop", f.c()]));
    host.wait();
    let mut host = Host::start(&f, false, true);
    assert_eq!(
        success(f.run(&["host-query", f.c(), "main"]))["generation"],
        1
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn indeterminate_publication_fails_closed_until_reopen() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start_mode(&f, "indeterminate", false);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    assert_eq!(
        wait_job(&f, 2, "failed")["job"]["error"],
        "publication_indeterminate"
    );
    error(
        f.run(&["host-query", f.c(), "main"]),
        7,
        "publication_indeterminate",
    );
    error(
        f.run(&["host-index", f.c(), "1"]),
        7,
        "publication_indeterminate",
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
    let mut host = Host::start(&f, false, true);
    assert_eq!(
        success(f.run(&["host-query", f.c(), "extra"]))["generation"],
        2
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn sigterm_cancels_worker_then_releases_store_and_socket() {
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(true);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    wait_paused(&f, 2);
    assert_eq!(
        unsafe { libc::kill(host.child.id() as i32, libc::SIGTERM) },
        0
    );
    for _ in 0..250 {
        if let Some(status) = host.child.try_wait().unwrap() {
            assert!(status.success());
            assert!(!f.control.join("control.sock").exists());
            assert_eq!(
                success(f.run(&["query", f.t(), f.s(), "alpha", "extra"]))["generation"],
                1
            );
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("host failed to stop");
}

#[cfg(feature = "test-hooks")]
#[test]
fn stop_during_build_requests_cancellation_and_requires_retry() {
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(true);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    wait_paused(&f, 2);
    error(f.run(&["host-stop", f.c()]), 3, "busy");
    assert_eq!(wait_job(&f, 2, "cancelled")["job"]["error"], "cancelled");
    assert_eq!(
        success(f.run(&["host-query", f.c(), "main"]))["generation"],
        1
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[test]
fn unsafe_control_paths_and_existing_socket_are_rejected_without_overwrite() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let f = Fixture::new();
    f.initialize();
    let path = f.control.join("control.sock");
    fs::write(&path, b"do not overwrite").unwrap();
    error(
        f.run(&["host", f.t(), f.s(), "alpha", f.c()]),
        6,
        "io_error",
    );
    assert_eq!(fs::read(&path).unwrap(), b"do not overwrite");
    fs::remove_file(path).unwrap();
    fs::set_permissions(&f.source, fs::Permissions::from_mode(0o700)).unwrap();
    error(
        f.run(&["host", f.t(), f.s(), "alpha", f.s()]),
        4,
        "unsafe_control",
    );
    assert!(!f.source.join("control.sock").exists());
    error(
        f.run(&["host", f.t(), f.s(), "alpha", f.t()]),
        4,
        "unsafe_control",
    );
    assert!(!f.store.join("control.sock").exists());
    fs::set_permissions(&f.control, fs::Permissions::from_mode(0o755)).unwrap();
    error(
        f.run(&["host", f.t(), f.s(), "alpha", f.c()]),
        4,
        "unsafe_control",
    );
    fs::set_permissions(&f.control, fs::Permissions::from_mode(0o700)).unwrap();
    let link = f.root.join("alias");
    symlink(&f.control, &link).unwrap();
    let output = f.run(&["host", f.t(), f.s(), "alpha", link.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(!f.control.join("control.sock").exists());
}

#[test]
fn oversized_frame_and_client_limit_fail_closed_then_recover() {
    use std::{
        io::{Read, Write},
        os::unix::{fs::PermissionsExt, net::UnixStream},
    };
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(false);
    let socket = f.control.join("control.sock");
    assert_eq!(
        fs::symlink_metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let mut over = UnixStream::connect(&socket).unwrap();
    over.write_all(&vec![b'x'; 4097]).unwrap();
    let mut reply = Vec::new();
    over.read_to_end(&mut reply).unwrap();
    let value: Value = serde_json::from_slice(&reply).unwrap();
    assert_eq!(value["body"]["error"], "protocol_error");
    let mut held = Vec::new();
    for _ in 0..8 {
        let mut s = UnixStream::connect(&socket).unwrap();
        s.write_all(b"{").unwrap();
        held.push(s);
    }
    let mut busy = false;
    for _ in 0..80 {
        let o = f.run(&["host-status", f.c()]);
        if o.status.code() == Some(3) {
            error(o, 3, "busy");
            busy = true;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(busy, "host did not enforce client cap");
    drop(held);
    let mut recovered = false;
    for _ in 0..80 {
        let o = f.run(&["host-status", f.c()]);
        if o.status.success() {
            recovered = true;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(recovered);
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[test]
fn oversized_query_response_returns_error_not_partial_results() {
    use rustgrok_path_index::durable::DurablePathIndex;
    use rustgrok_path_index::{CancellationToken, IndexLimits};
    let f = Fixture::new();
    let limits = IndexLimits {
        max_documents: 50_000,
        max_path_bytes: 4096,
        max_total_path_bytes: 16 * 1024 * 1024,
        max_live_generations: 2,
    };
    let db = DurablePathIndex::create(&f.parent, "store", &f.source, "alpha", limits, 1_000_000)
        .unwrap();
    let paths = (0..900)
        .map(|i| {
            Ok(format!(
                "/alpha/common/{}file{i}.c",
                "abcdefghij/".repeat(110)
            ))
        })
        .collect::<Vec<_>>();
    db.replace(None, paths.len(), paths, &CancellationToken::new())
        .unwrap();
    drop(db);
    let mut host = Host::start(&f, false, true);
    error(f.run(&["host-query", f.c(), "common"]), 4, "result_limit");
    error(
        f.run(&["host-query-expression", f.c(), "alpha AND common"]),
        4,
        "result_limit",
    );
    assert_eq!(
        success(f.run(&["host-query", f.c(), "absent"]))["paths"],
        json!([])
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[test]
fn overlong_unix_socket_path_is_rejected_before_bind() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.initialize();
    let long = f.root.join("x".repeat(90));
    fs::create_dir(&long).unwrap();
    fs::set_permissions(&long, fs::Permissions::from_mode(0o700)).unwrap();
    error(
        f.run(&["host", f.t(), f.s(), "alpha", long.to_str().unwrap()]),
        4,
        "unsafe_control",
    );
    assert!(!long.join("control.sock").exists());
}

#[cfg(not(feature = "test-hooks"))]
#[test]
fn default_host_ignores_test_modes_and_has_no_test_release_command() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start_mode(&f, "panic", false);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    assert_eq!(wait_job(&f, 2, "succeeded")["job"]["generation"], 2);
    error(f.run(&["host-test-release", f.c()]), 2, "usage");
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn killed_host_keeps_old_generation_and_does_not_remove_stale_socket() {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(true);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    wait_paused(&f, 2);
    let path = f.control.join("control.sock");
    let before = fs::symlink_metadata(&path).unwrap();
    assert!(before.file_type().is_socket());
    host.child.kill().unwrap();
    let _ = host.child.wait().unwrap();
    assert_eq!(
        success(f.run(&["query", f.t(), f.s(), "alpha", "main"]))["generation"],
        1
    );
    error(
        f.run(&["host", f.t(), f.s(), "alpha", f.c()]),
        6,
        "io_error",
    );
    let after = fs::symlink_metadata(path).unwrap();
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
}

#[test]
fn client_times_out_if_same_user_socket_never_replies() {
    use std::os::unix::{fs::PermissionsExt, net::UnixListener};
    let f = Fixture::new();
    let socket = f.control.join("control.sock");
    let _listener = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    error(f.run(&["host-status", f.c()]), 6, "io_error");
}

#[test]
fn ambiguous_and_invalid_json_frames_have_no_side_effects() {
    use std::{
        io::{Read, Write},
        os::unix::net::UnixStream,
    };
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(false);
    let socket = f.control.join("control.sock");
    for frame in [
        b"{\"command\":\"status\",\"command\":\"stop\"}\n".as_slice(),
        b"{\"command\":\"index\",\"value\":\"none\",\"value\":\"1\"}\n".as_slice(),
        b"{\"command\":\"index\",\"value\":null,\"value\":\"none\"}\n".as_slice(),
        b"{\"command\":\"status\",\"comm\\u0061nd\":\"stop\"}\n".as_slice(),
        b"{\"command\":\"stop\",\"unexpected\":true}\n".as_slice(),
        b"{\"command\":\"\xff\"}\n".as_slice(),
    ] {
        let mut stream = UnixStream::connect(&socket).unwrap();
        stream.write_all(frame).unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&raw).unwrap(),
            json!({"code":4,"body":{"error":"protocol_error"}})
        );
        assert_eq!(success(f.run(&["host-status", f.c()]))["job"], Value::Null);
    }
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

fn rejected_host_without_side_effect(f: &Fixture, control: &std::path::Path) {
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_rustgrok"))
        .args(["host", f.t(), f.s(), "alpha", control.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if child.try_wait().unwrap().is_some() {
            let output = child.wait_with_output().unwrap();
            error(output, 4, "unsafe_control");
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    let _ = child.wait();
    panic!("host accepted an unsafe control ancestor or ACL");
}

#[test]
fn untrusted_writable_ancestor_and_extended_acl_fail_before_bind() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.initialize();
    let outside = f.root.join("untrusted");
    let control = outside.join("ctl");
    fs::create_dir(&outside).unwrap();
    fs::create_dir(&control).unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o777)).unwrap();
    fs::set_permissions(&control, fs::Permissions::from_mode(0o700)).unwrap();
    rejected_host_without_side_effect(&f, &control);
    assert!(!f.source.join("control.sock").exists());
    assert!(!control.join("control.sock").exists());
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o700)).unwrap();
    let status = Command::new("chmod")
        .args(["+a", "everyone allow add_file,delete_child"])
        .arg(&outside)
        .status()
        .unwrap();
    assert!(status.success(), "could not establish real macOS ACL");
    assert_eq!(
        fs::symlink_metadata(&outside).unwrap().permissions().mode() & 0o077,
        0
    );
    let acl = Command::new("ls")
        .arg("-lde")
        .arg(&outside)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&acl.stdout).contains("everyone allow"));
    rejected_host_without_side_effect(&f, &control);
    assert!(!f.source.join("control.sock").exists());
    assert!(!control.join("control.sock").exists());
}

#[cfg(feature = "test-hooks")]
#[test]
fn renamed_control_and_symlink_swap_cannot_write_or_unlink_inside_source() {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::fs::symlink,
    };
    let f = Fixture::new();
    f.initialize();
    let mut child = Command::new(env!("CARGO_BIN_EXE_rustgrok"))
        .args(["host", f.t(), f.s(), "alpha", f.c()])
        .env("RUSTGROK_TEST_BEFORE_BIND", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut line = String::new();
        let _ = BufReader::new(stderr).read_line(&mut line);
        let _ = tx.send(line);
    });
    let marker = rx.recv_timeout(Duration::from_secs(5));
    if marker.as_deref() != Ok("RUSTGROK_TEST_BEFORE_BIND\n") {
        let _ = child.kill();
        let _ = child.wait();
        panic!("missing pre-bind marker {marker:?}");
    }
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut r = BufReader::new(stdout);
        let mut line = String::new();
        let _ = r.read_line(&mut line);
        let _ = tx.send(line);
        for _ in r.lines() {}
    });
    let mut host = Host { child };
    let held = f.root.join("held-control");
    fs::rename(&f.control, &held).unwrap();
    symlink(&f.source, &f.control).unwrap();
    host.child.stdin.as_mut().unwrap().write_all(b"x").unwrap();
    let ready = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("host did not complete bind");
    assert_eq!(
        serde_json::from_str::<Value>(&ready).unwrap()["state"],
        "listening"
    );
    assert!(
        !f.source.join("control.sock").exists(),
        "socket was bound inside source"
    );
    assert!(held.join("control.sock").exists());
    fs::write(f.source.join("control.sock"), b"source sentinel").unwrap();
    assert_eq!(
        unsafe { libc::kill(host.child.id() as i32, libc::SIGTERM) },
        0
    );
    host.wait();
    assert_eq!(
        fs::read(f.source.join("control.sock")).unwrap(),
        b"source sentinel"
    );
    assert!(!held.join("control.sock").exists());
}

#[cfg(feature = "test-hooks")]
#[test]
fn caught_worker_panic_cannot_serve_queries_before_fault_publication() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start_mode(&f, "panic-gap", false);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    success(f.run(&["host-index", f.c(), "1"]));
    wait_paused(&f, 2);
    error(f.run(&["host-query", f.c(), "main"]), 5, "worker_panic");
    success(f.run(&["host-test-release", f.c()]));
    wait_job(&f, 2, "failed");
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[test]
fn trusted_sticky_ancestor_can_host_private_control_directory() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = Fixture::new();
    f.initialize();
    let shared = f.root.join("sticky");
    let control = shared.join("ctl");
    fs::create_dir(&shared).unwrap();
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o1777)).unwrap();
    fs::create_dir(&control).unwrap();
    fs::set_permissions(&control, fs::Permissions::from_mode(0o700)).unwrap();
    f.control = control;
    let mut host = f.start(false);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    assert_eq!(
        success(f.run(&["host-query", f.c(), "main"]))["generation"],
        1
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}

#[test]
fn startup_helper_reaps_child_if_setup_panics_after_spawn() {
    let f = Fixture::new();
    f.initialize();
    let mut pid = 0;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _host = Host::start_mode_with_spawn_hook(&f, "none", false, |child| {
            pid = child.id() as i32;
            assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0);
            panic!("injected setup panic after spawn");
        });
    }));
    assert!(result.is_err());
    assert!(pid > 0);
    let mut status = 0;
    let observed = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
    if observed == 0 {
        // This PID is our unreaped child, so it cannot have been reused.
        assert_eq!(unsafe { libc::kill(pid, libc::SIGKILL) }, 0);
        assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
        panic!("startup helper leaked its child after setup panic");
    }
    assert_eq!(observed, -1, "startup helper did not reap its child");
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn readiness_timeout_reaps_stopped_child_and_reports_cleanup() {
    let f = Fixture::new();
    f.initialize();
    let mut pid = 0;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _host = Host::start_mode_with_spawn_hook(&f, "none", false, |child| {
            pid = child.id() as i32;
            assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0);
        });
    }));
    let panic = outcome.expect_err("stopped child should hit readiness timeout");
    let text = panic
        .downcast_ref::<String>()
        .expect("startup panic message");
    assert!(text.contains("Err(Timeout)"), "{text}");
    assert!(text.contains("killed and reaped"), "{text}");
    let mut status = 0;
    let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
    if result == 0 {
        assert_eq!(unsafe { libc::kill(pid, libc::SIGKILL) }, 0);
        let _ = unsafe { libc::waitpid(pid, &mut status, 0) };
        panic!("timeout left an unreaped child");
    }
    assert_eq!(result, -1, "timeout child was not reaped");
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn subprocess_host_path_expression_uses_published_generation() {
    let f = Fixture::new();
    f.initialize();
    let mut host = f.start(false);
    success(f.run(&["host-index", f.c(), "none"]));
    wait_job(&f, 1, "succeeded");
    assert_eq!(
        success(f.run(&["host-query-expression", f.c(), "alpha AND main"])),
        json!({"operation":"host-query-expression","generation":1,"order":"unspecified","paths":["/alpha/main.c"]})
    );
    for op in ["AND", "OR", "NOT"] {
        let input = format!("alpha {op}\u{c}main");
        error(
            f.run(&["host-query-expression", f.c(), &input]),
            4,
            "unsupported_or_limit",
        );
    }
    let bad = f.run(&["host-query-expression", f.c(), "main OR alpha AND absent"]);
    assert_eq!(bad.status.code(), Some(4));
    assert_eq!(
        success(f.run(&["host-query", f.c(), "main"]))["paths"],
        json!(["/alpha/main.c"])
    );
    success(f.run(&["host-stop", f.c()]));
    host.wait();
}
