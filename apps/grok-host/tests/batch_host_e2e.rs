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
                "rg-bhost-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let parent = root.join("stores");
        let control = root.join("control");
        for p in [&source, &parent, &control] {
            fs::create_dir(p).unwrap();
        }
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&control, fs::Permissions::from_mode(0o700)).unwrap();
        let store = parent.join("batch");
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
    fn git(&self, rel: &str) {
        let p = self.source.join(rel);
        fs::create_dir_all(&p).unwrap();
        assert!(Command::new("git")
            .args(["init", "--quiet"])
            .arg(p)
            .status()
            .unwrap()
            .success());
    }
    fn initialize(&self) {
        self.git("alpha/repo");
        self.git("alpha/repo/nested");
        self.git("beta/repo");
        fs::write(self.source.join("alpha/repo/old.c"), b"old\n").unwrap();
        fs::write(self.source.join("alpha/repo/nested/app.js"), b"app\n").unwrap();
        fs::write(self.source.join("beta/repo/old.py"), b"old\n").unwrap();
        ok(self.run(&["batch-create", self.p(), "batch", self.s()]));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn ok(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    serde_json::from_slice(&out.stdout).unwrap()
}
struct Host(Child);
impl Host {
    fn start(f: &Fixture, mode: &str, published: bool) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustgrok"));
        cmd.args(["batch-host", f.t(), f.s(), f.c()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if mode != "none" {
            cmd.env("RUSTGROK_TEST_HOST_MODE", mode);
        }
        let mut host = Self(cmd.spawn().unwrap());
        let output = host.0.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(output);
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
        assert_eq!(
            first
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok()),
            Some(json!({"operation":"batch-host","state":"listening","published":published}))
        );
        host
    }
    fn wait(&mut self) {
        assert!(self.0.wait().unwrap().success());
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Ok(None) = self.0.try_wait() {
            if self.0.kill().is_ok() {
                let _ = self.0.wait();
            }
        }
    }
}
fn wait_job(f: &Fixture, id: u64, state: &str) -> Value {
    for _ in 0..250 {
        let value = ok(f.run(&["batch-host-status", f.c()]));
        if value["job"]["id"] == id && value["job"]["state"] == state {
            return value;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("job {id} did not become {state}")
}
#[test]
fn batch_host_starts_indexes_queries_and_stops() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start(&f, "none", false);
    assert_eq!(
        ok(f.run(&["batch-host-index", f.c(), "none", "alpha", "beta"]))["job"],
        1
    );
    assert_eq!(wait_job(&f, 1, "succeeded")["job"]["generation"], 1);
    assert_eq!(
        ok(f.run(&["batch-host-query", f.c(), "alpha", "old"])),
        json!({"operation":"batch-host-query","generation":1,"order":"unspecified","paths":["/alpha/repo/old.c"]})
    );
    ok(f.run(&["batch-host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
fn failed(out: Output, code: i32, label: &str) {
    assert_eq!(
        out.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stderr).unwrap(),
        json!({"error":label})
    );
}
#[cfg(feature = "test-hooks")]
fn source_files(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn walk(
        base: &std::path::Path,
        path: &std::path::Path,
        out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for item in fs::read_dir(path).unwrap() {
            let p = item.unwrap().path();
            let m = fs::symlink_metadata(&p).unwrap();
            if m.is_dir() {
                walk(base, &p, out)
            } else if m.is_file() {
                out.insert(
                    p.strip_prefix(base).unwrap().to_owned(),
                    fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[cfg(feature = "test-hooks")]
fn wait_paused(f: &Fixture, id: u64) {
    for _ in 0..250 {
        let v = ok(f.run(&["batch-host-status", f.c()]));
        if v["job"]["id"] == id && v["job"]["paused"] == true {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("job did not pause");
}
#[cfg(feature = "test-hooks")]
fn started(f: &Fixture, mode: &str) -> Host {
    f.initialize();
    let host = Host::start(f, mode, false);
    ok(f.run(&["batch-host-index", f.c(), "none", "alpha", "beta"]));
    assert_eq!(wait_job(f, 1, "succeeded")["job"]["generation"], 1);
    host
}

#[cfg(feature = "test-hooks")]
#[test]
fn old_batch_stays_queryable_during_second_scan_then_switches_together() {
    let f = Fixture::new();
    let mut host = started(&f, "pause");
    fs::write(f.source.join("alpha/repo/new.c"), b"new\n").unwrap();
    fs::write(f.source.join("beta/repo/new.py"), b"new\n").unwrap();
    let before = source_files(&f.source);
    assert_eq!(
        ok(f.run(&["batch-host-index", f.c(), "1", "alpha", "beta"]))["job"],
        2
    );
    wait_paused(&f, 2);
    let mut children = Vec::new();
    for name in ["alpha", "beta", "alpha", "beta"] {
        children.push((
            name,
            Command::new(env!("CARGO_BIN_EXE_rustgrok"))
                .args(["batch-host-query", f.c(), name, "old"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        ));
    }
    for (name, child) in children {
        let v = ok(child.wait_with_output().unwrap());
        assert_eq!(v["generation"], 1);
        assert_eq!(
            v["paths"],
            json!([format!(
                "/{name}/repo/old.{}",
                if name == "alpha" { "c" } else { "py" }
            )])
        );
    }
    assert_eq!(
        ok(f.run(&[
            "batch-host-query-expression",
            f.c(),
            "alpha",
            "alpha AND new"
        ]))["paths"],
        json!([])
    );
    ok(f.run(&["batch-host-test-release", f.c()]));
    assert_eq!(wait_job(&f, 2, "succeeded")["job"]["generation"], 2);
    for name in ["alpha", "beta"] {
        let v = ok(f.run(&["batch-host-query", f.c(), name, "new"]));
        assert_eq!(v["generation"], 2);
        assert_eq!(v["paths"].as_array().unwrap().len(), 1);
    }
    assert_eq!(source_files(&f.source), before);
    ok(f.run(&["batch-host-stop", f.c()]));
    host.wait();
    let mut host = Host::start(&f, "none", true);
    for name in ["alpha", "beta"] {
        assert_eq!(
            ok(f.run(&["batch-host-query", f.c(), name, "new"]))["generation"],
            2
        );
    }
    ok(f.run(&["batch-host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn cancelled_failed_and_stale_batch_jobs_keep_both_old_projects() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let mut host = started(&f, "pause");
    let pointer = fs::read(f.store.join("CURRENT")).unwrap();
    fs::write(f.source.join("alpha/repo/new.c"), b"new\n").unwrap();
    fs::write(f.source.join("beta/repo/new.py"), b"new\n").unwrap();
    ok(f.run(&["batch-host-index", f.c(), "1", "alpha", "beta"]));
    wait_paused(&f, 2);
    assert_eq!(
        ok(f.run(&["batch-host-cancel", f.c()])),
        json!({"operation":"host-cancel","accepted":true})
    );
    assert_eq!(wait_job(&f, 2, "cancelled")["job"]["error"], "cancelled");
    fs::write(f.source.join("beta/repo/bad.txt"), b"bad\n").unwrap();
    ok(f.run(&["batch-host-index", f.c(), "1", "alpha", "beta"]));
    wait_paused(&f, 3);
    ok(f.run(&["batch-host-test-release", f.c()]));
    assert_eq!(
        wait_job(&f, 3, "failed")["job"]["error"],
        "unsupported_or_limit"
    );
    fs::remove_file(f.source.join("beta/repo/bad.txt")).unwrap();
    let file = f.source.join("beta/repo/new.py");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    ok(f.run(&["batch-host-index", f.c(), "1", "alpha", "beta"]));
    wait_paused(&f, 4);
    ok(f.run(&["batch-host-test-release", f.c()]));
    assert_eq!(wait_job(&f, 4, "failed")["job"]["error"], "io_error");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    ok(f.run(&["batch-host-index", f.c(), "99", "alpha", "beta"]));
    assert_eq!(
        wait_job(&f, 5, "failed")["job"]["error"],
        "stale_generation"
    );
    ok(f.run(&["batch-host-index", f.c(), "1", "alpha"]));
    assert_eq!(
        wait_job(&f, 6, "failed")["job"]["error"],
        "unsupported_or_limit"
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
    for name in ["alpha", "beta"] {
        let view = ok(f.run(&["batch-host-query", f.c(), name, "old"]));
        assert_eq!(view["generation"], 1);
        assert_eq!(view["paths"].as_array().unwrap().len(), 1);
    }
    ok(f.run(&["batch-host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn worker_panic_and_indeterminate_block_reads_until_restart() {
    for (mode, label, exit, expected_gen, term) in [
        ("panic", "worker_panic", 5, 1, "old"),
        ("indeterminate", "publication_indeterminate", 7, 2, "new"),
    ] {
        let f = Fixture::new();
        let mut host = started(&f, mode);
        fs::write(f.source.join("alpha/repo/new.c"), b"new\n").unwrap();
        fs::write(f.source.join("beta/repo/new.py"), b"new\n").unwrap();
        ok(f.run(&["batch-host-index", f.c(), "1", "alpha", "beta"]));
        assert_eq!(wait_job(&f, 2, "failed")["job"]["error"], label);
        failed(
            f.run(&["batch-host-query", f.c(), "alpha", "old"]),
            exit,
            label,
        );
        failed(
            f.run(&["batch-host-index", f.c(), "1", "alpha", "beta"]),
            exit,
            label,
        );
        ok(f.run(&["batch-host-stop", f.c()]));
        host.wait();
        let mut host = Host::start(&f, "none", true);
        for name in ["alpha", "beta"] {
            let view = ok(f.run(&["batch-host-query", f.c(), name, term]));
            assert_eq!(view["generation"], expected_gen);
            assert_eq!(view["paths"].as_array().unwrap().len(), 1);
        }
        ok(f.run(&["batch-host-stop", f.c()]));
        host.wait();
    }
}

#[test]
fn batch_socket_rejects_duplicate_and_unknown_request_fields() {
    use std::{
        io::{Read, Write},
        os::unix::net::UnixStream,
    };
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start(&f, "none", false);
    for frame in ["{\"command\":\"status\",\"command\":\"stop\"}\n", "{\"command\":\"status\",\"unknown\":1}\n", "{\"command\":\"batch-index\",\"value\":\"{\\\"expected\\\":\\\"none\\\",\\\"expected\\\":\\\"none\\\",\\\"projects\\\":[\\\"alpha\\\"]}\"}\n"] {
        let mut stream=UnixStream::connect(f.control.join("control.sock")).unwrap();stream.write_all(frame.as_bytes()).unwrap();let mut raw=Vec::new();stream.read_to_end(&mut raw).unwrap();let response:Value=serde_json::from_slice(&raw).unwrap();assert_eq!(response["code"],4);assert_eq!(response["body"]["error"],"protocol_error");
    }
    assert_eq!(ok(f.run(&["batch-host-status", f.c()]))["job"], Value::Null);
    ok(f.run(&["batch-host-stop", f.c()]));
    host.wait();
}

#[cfg(not(feature = "test-hooks"))]
#[test]
fn default_batch_host_ignores_pause_mode() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start(&f, "pause", false);
    ok(f.run(&["batch-host-index", f.c(), "none", "alpha", "beta"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("alpha/repo/new.c"), b"new\n").unwrap();
    ok(f.run(&["batch-host-index", f.c(), "1", "alpha", "beta"]));
    wait_job(&f, 2, "succeeded");
    ok(f.run(&["batch-host-stop", f.c()]));
    host.wait();
}
