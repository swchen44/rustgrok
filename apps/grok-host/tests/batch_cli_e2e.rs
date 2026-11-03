// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
#![cfg(target_os = "macos")]
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    source: PathBuf,
    parent: PathBuf,
    store: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "rustgrok-batch-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let parent = root.join("stores");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&parent).unwrap();
        let store = parent.join("batch");
        Self {
            root,
            source,
            parent,
            store,
        }
    }
    fn src(&self) -> &str {
        self.source.to_str().unwrap()
    }
    fn parent(&self) -> &str {
        self.parent.to_str().unwrap()
    }
    fn store(&self) -> &str {
        self.store.to_str().unwrap()
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rustgrok"))
            .args(args)
            .output()
            .unwrap()
    }
    fn git(&self, rel: &str) {
        let dir = self.source.join(rel);
        fs::create_dir_all(&dir).unwrap();
        assert!(Command::new("git")
            .args(["init", "--quiet"])
            .arg(dir)
            .status()
            .unwrap()
            .success());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn ok(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
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
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[test]
fn subprocess_two_project_source_batch_publishes_one_generation() {
    let f = Fixture::new();
    f.git("alpha/component");
    f.git("alpha/component/nested");
    f.git("beta/repo");
    fs::write(f.source.join("alpha/component/main.c"), b"int main;\n").unwrap();
    fs::write(
        f.source.join("alpha/component/nested/app.js"),
        b"let value;\n",
    )
    .unwrap();
    fs::write(f.source.join("beta/repo/helper.py"), b"value = 1\n").unwrap();
    let before = files(&f.source);
    assert_eq!(
        ok(f.run(&["batch-create", f.parent(), "batch", f.src()])),
        json!({"operation":"batch-create","status":"created"})
    );
    assert_eq!(
        ok(f.run(&["batch-index", f.store(), f.src(), "none", "alpha", "beta"])),
        json!({"operation":"batch-index","generation":1})
    );
    assert_eq!(
        ok(f.run(&["batch-query", f.store(), f.src(), "alpha", "main"])),
        json!({"operation":"batch-query","generation":1,"order":"unspecified","paths":["/alpha/component/main.c"]})
    );
    assert_eq!(
        ok(f.run(&["batch-query", f.store(), f.src(), "beta", "helper"])),
        json!({"operation":"batch-query","generation":1,"order":"unspecified","paths":["/beta/repo/helper.py"]})
    );
    assert_eq!(files(&f.source), before);
}

fn failed(output: Output, code: i32, label: &str) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap(),
        json!({"error":label})
    );
}

#[test]
fn failed_second_project_and_changed_registry_preserve_old_batch() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let f = Fixture::new();
    f.git("alpha/repo");
    f.git("beta/repo");
    fs::write(f.source.join("alpha/repo/old.c"), b"old alpha\n").unwrap();
    fs::write(f.source.join("beta/repo/old.py"), b"old beta\n").unwrap();
    ok(f.run(&["batch-create", f.parent(), "batch", f.src()]));
    ok(f.run(&["batch-index", f.store(), f.src(), "none", "alpha", "beta"]));
    let old_pointer = fs::read(f.store.join("CURRENT")).unwrap();
    fs::remove_file(f.source.join("alpha/repo/old.c")).unwrap();
    fs::write(f.source.join("alpha/repo/new.c"), b"new alpha\n").unwrap();
    fs::write(f.source.join("beta/repo/new.py"), b"new beta\n").unwrap();
    fs::write(f.source.join("beta/repo/bad.txt"), b"unsupported\n").unwrap();
    let saved = files(&f.source);
    failed(
        f.run(&["batch-index", f.store(), f.src(), "1", "alpha", "beta"]),
        4,
        "unsupported_or_limit",
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), old_pointer);
    assert_eq!(files(&f.source), saved);
    for name in ["alpha", "beta"] {
        let got = ok(f.run(&["batch-query", f.store(), f.src(), name, "old"]));
        assert_eq!(got["generation"], 1);
        assert_eq!(got["paths"].as_array().unwrap().len(), 1);
    }
    failed(
        f.run(&["batch-index", f.store(), f.src(), "1", "alpha"]),
        4,
        "unsupported_or_limit",
    );
    failed(
        f.run(&["batch-index", f.store(), f.src(), "1", "alpha", "alpha"]),
        4,
        "unsupported_or_limit",
    );
    failed(
        f.run(&["batch-index", f.store(), f.src(), "1", "alpha", "unknown"]),
        4,
        "unsupported_or_limit",
    );
    failed(
        f.run(&["batch-index", f.store(), f.src(), "1", "alpha", "../beta"]),
        2,
        "usage",
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), old_pointer);
    fs::remove_file(f.source.join("beta/repo/bad.txt")).unwrap();
    let target = f.source.join("beta/repo/new.py");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o000)).unwrap();
    failed(
        f.run(&["batch-index", f.store(), f.src(), "1", "alpha", "beta"]),
        6,
        "io_error",
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), old_pointer);
    fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
    let link = f.source.join("beta/repo/link.c");
    symlink("new.py", &link).unwrap();
    failed(
        f.run(&["batch-index", f.store(), f.src(), "1", "alpha", "beta"]),
        4,
        "unsupported_or_limit",
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), old_pointer);
    fs::remove_file(link).unwrap();
    let saved = files(&f.source);
    assert_eq!(
        ok(f.run(&["batch-index", f.store(), f.src(), "1", "beta", "alpha"]))["generation"],
        2
    );
    assert_ne!(fs::read(f.store.join("CURRENT")).unwrap(), old_pointer);
    for name in ["alpha", "beta"] {
        let got = ok(f.run(&[
            "batch-query-expression",
            f.store(),
            f.src(),
            name,
            &format!("{name} AND new"),
        ]));
        assert_eq!(got["generation"], 2);
        assert_eq!(got["paths"].as_array().unwrap().len(), 1);
    }
    assert_eq!(files(&f.source), saved);
}

#[test]
fn single_and_batch_stores_refuse_cross_open_and_unknown_project() {
    let f = Fixture::new();
    f.git("alpha/repo");
    fs::write(f.source.join("alpha/repo/file.c"), b"x\n").unwrap();
    ok(f.run(&["batch-create", f.parent(), "batch", f.src()]));
    ok(f.run(&["batch-index", f.store(), f.src(), "none", "alpha"]));
    failed(
        f.run(&["batch-query", f.store(), f.src(), "beta", "file"]),
        4,
        "unsupported_or_limit",
    );
    failed(
        f.run(&["query", f.store(), f.src(), "rg_batch", "file"]),
        5,
        "corrupt_store",
    );
    ok(f.run(&["create", f.parent(), "single", f.src(), "rg_batch"]));
    let single = f.parent.join("single");
    failed(
        f.run(&[
            "batch-query",
            single.to_str().unwrap(),
            f.src(),
            "alpha",
            "file",
        ]),
        5,
        "corrupt_store",
    );
}

#[cfg(feature = "test-hooks")]
struct PausedBatch {
    child: std::process::Child,
    lines: std::sync::mpsc::Receiver<String>,
}
#[cfg(feature = "test-hooks")]
impl PausedBatch {
    fn start(f: &Fixture, pause_second: bool) -> Self {
        use std::{
            io::{BufRead, BufReader},
            process::Stdio,
            sync::mpsc,
            thread,
            time::Duration,
        };
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustgrok"));
        if pause_second {
            cmd.env("RUSTGROK_TEST_BATCH_PAUSE_SECOND", "1");
        }
        let mut child = cmd
            .args(["batch-index", f.store(), f.src(), "1", "alpha", "beta"])
            .env("RUSTGROK_TEST_PAUSE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                if tx.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        let running = Self { child, lines: rx };
        let first = running.lines.recv_timeout(Duration::from_secs(5));
        assert_eq!(first.as_deref(), Ok("RUSTGROK_TEST_PAUSED"));
        running
    }
    fn release(&mut self) {
        use std::io::Write;
        self.child.stdin.as_mut().unwrap().write_all(b"x").unwrap();
    }
    fn next(&self) -> String {
        self.lines
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("batch stderr timeout")
    }
    fn stdout(&mut self) -> Vec<u8> {
        use std::io::Read;
        let mut out = Vec::new();
        self.child
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut out)
            .unwrap();
        out
    }
}
#[cfg(feature = "test-hooks")]
impl Drop for PausedBatch {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            if self.child.kill().is_ok() {
                let _ = self.child.wait();
            }
        }
    }
}

#[cfg(feature = "test-hooks")]
fn indexed_fixture() -> Fixture {
    let f = Fixture::new();
    f.git("alpha/repo");
    f.git("beta/repo");
    fs::write(f.source.join("alpha/repo/old.c"), b"old\n").unwrap();
    fs::write(f.source.join("beta/repo/old.py"), b"old\n").unwrap();
    ok(f.run(&["batch-create", f.parent(), "batch", f.src()]));
    ok(f.run(&["batch-index", f.store(), f.src(), "none", "alpha", "beta"]));
    fs::write(f.source.join("alpha/repo/new.c"), b"new\n").unwrap();
    fs::write(f.source.join("beta/repo/new.py"), b"new\n").unwrap();
    f
}

#[cfg(feature = "test-hooks")]
#[test]
fn real_sigint_cancels_batch_and_other_process_reports_busy() {
    let f = indexed_fixture();
    let saved = files(&f.source);
    let pointer = fs::read(f.store.join("CURRENT")).unwrap();
    let mut running = PausedBatch::start(&f, true);
    failed(
        f.run(&["batch-query", f.store(), f.src(), "alpha", "old"]),
        3,
        "busy",
    );
    assert_eq!(
        unsafe { libc::kill(running.child.id() as i32, libc::SIGINT) },
        0
    );
    assert_eq!(running.next(), "RUSTGROK_TEST_SIGNAL_OBSERVED");
    running.release();
    assert_eq!(running.child.wait().unwrap().code(), Some(130));
    assert!(running.stdout().is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&running.next()).unwrap(),
        json!({"error":"cancelled"})
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
    for name in ["alpha", "beta"] {
        let view = ok(f.run(&["batch-query", f.store(), f.src(), name, "old"]));
        assert_eq!(view["generation"], 1);
        assert_eq!(view["paths"].as_array().unwrap().len(), 1);
        assert!(
            ok(f.run(&["batch-query", f.store(), f.src(), name, "new"]))["paths"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(files(&f.source), saved);
}

#[cfg(feature = "test-hooks")]
#[test]
fn second_project_changes_after_scan_abort_whole_batch() {
    let f = indexed_fixture();
    let pointer = fs::read(f.store.join("CURRENT")).unwrap();
    let mut running = PausedBatch::start(&f, false);
    fs::write(
        f.source.join("beta/repo/new.py"),
        b"changed after first observation\n",
    )
    .unwrap();
    let saved = files(&f.source);
    running.release();
    assert_eq!(running.child.wait().unwrap().code(), Some(5));
    assert!(running.stdout().is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&running.next()).unwrap(),
        json!({"error":"source_changed"})
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
    for name in ["alpha", "beta"] {
        assert_eq!(
            ok(f.run(&["batch-query", f.store(), f.src(), name, "old"]))["generation"],
            1
        );
    }
    assert_eq!(files(&f.source), saved);
}

#[cfg(not(feature = "test-hooks"))]
#[test]
fn default_batch_binary_ignores_test_pause_environment() {
    let f = Fixture::new();
    f.git("alpha/repo");
    f.git("beta/repo");
    fs::write(f.source.join("alpha/repo/main.c"), b"x\n").unwrap();
    fs::write(f.source.join("beta/repo/main.py"), b"x\n").unwrap();
    ok(f.run(&["batch-create", f.parent(), "batch", f.src()]));
    let out = Command::new(env!("CARGO_BIN_EXE_rustgrok"))
        .args(["batch-index", f.store(), f.src(), "none", "alpha", "beta"])
        .env("RUSTGROK_TEST_PAUSE", "1")
        .env("RUSTGROK_TEST_BATCH_PAUSE_SECOND", "1")
        .output()
        .unwrap();
    assert_eq!(ok(out), json!({"operation":"batch-index","generation":1}));
}
