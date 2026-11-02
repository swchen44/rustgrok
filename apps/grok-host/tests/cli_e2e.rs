/*
 * CDDL HEADER START
 * The contents of this file are subject to the terms of the
 * Common Development and Distribution License (the "License").
 * You may not use this file except in compliance with the License.
 * See LICENSE.txt included in this distribution for the specific
 * language governing permissions and limitations under the License.
 * When distributing Covered Code, include this CDDL HEADER in each
 * file and include the License file at LICENSE.txt.
 * CDDL HEADER END
 */
/*
 * Copyright (c) 2008, 2023, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2017, 2019, Chris Fraire <cfraire@me.com>.
 * Portions Copyright (c) 2020, Ric Harris <harrisric@users.noreply.github.com>.
 * Portions Copyright (c) 2026, swchen44.
 * Based on the add/remove scenario in OpenGrok IndexerTest.java,
 * commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3.
 * Process, lock and cancellation cases are Rustgrok-specific additions.
 */
// SPDX-License-Identifier: CDDL-1.0
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
                "rustgrok-cli-e2e-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let parent = root.join("indexes");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&parent).unwrap();
        let store = parent.join("store");
        Self {
            root,
            source,
            parent,
            store,
        }
    }
    fn git(&self, rel: &str) {
        let p = self.source.join(rel);
        fs::create_dir_all(&p).unwrap();
        let o = Command::new("git")
            .args(["init", "--quiet"])
            .arg(p)
            .output()
            .unwrap();
        assert!(o.status.success(), "git init failed");
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rustgrok"))
            .args(args)
            .output()
            .unwrap()
    }
    fn source(&self) -> &str {
        self.source.to_str().unwrap()
    }
    fn parent(&self) -> &str {
        self.parent.to_str().unwrap()
    }
    fn store(&self) -> &str {
        self.store.to_str().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn contents(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for item in fs::read_dir(path).unwrap() {
            let p = item.unwrap().path();
            let m = fs::symlink_metadata(&p).unwrap();
            if m.is_dir() {
                walk(base, &p, out);
            } else if m.is_file() {
                out.insert(
                    p.strip_prefix(base).unwrap().to_owned(),
                    fs::read(&p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(path, path, &mut out);
    out
}
fn ok(output: Output) -> Value {
    assert!(
        output.status.success(),
        "exit {:?}, stderr {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn subprocess_index_query_add_remove_and_restart() {
    let f = Fixture::new();
    f.git("component-c");
    f.git("component-c/nested-js");
    f.git("component-js");
    fs::write(
        f.source.join("component-c/main.c"),
        include_bytes!("../../../tests/fixtures/public-source/c/main.c"),
    )
    .unwrap();
    fs::write(
        f.source.join("component-c/nested-js/function.js"),
        include_bytes!("../../../tests/fixtures/public-source/javascript/function.js"),
    )
    .unwrap();
    fs::write(
        f.source.join("component-js/function.js"),
        include_bytes!("../../../tests/fixtures/public-source/javascript/function.js"),
    )
    .unwrap();
    let before = contents(&f.source);
    assert_eq!(
        ok(f.run(&["create", f.parent(), "store", f.source(), "alpha"])),
        json!({"operation":"create","status":"created"})
    );
    assert_eq!(
        ok(f.run(&["index", f.store(), f.source(), "alpha", "none"])),
        json!({"operation":"index","generation":1})
    );
    assert_eq!(
        ok(f.run(&["query", f.store(), f.source(), "alpha", "main"])),
        json!({"operation":"query","generation":1,"order":"unspecified","paths":["/alpha/component-c/main.c"]})
    );
    let matches = ok(f.run(&["query", f.store(), f.source(), "alpha", "function"]));
    assert_eq!(matches["order"], json!("unspecified"));
    let actual = matches["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        actual,
        [
            "/alpha/component-c/nested-js/function.js",
            "/alpha/component-js/function.js"
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(before, contents(&f.source));
    fs::remove_file(f.source.join("component-c/main.c")).unwrap();
    fs::write(f.source.join("component-c/new.py"), b"print('new')\n").unwrap();
    let before = contents(&f.source);
    assert_eq!(
        ok(f.run(&["index", f.store(), f.source(), "alpha", "1"])),
        json!({"operation":"index","generation":2})
    );
    assert_eq!(
        ok(f.run(&["query", f.store(), f.source(), "alpha", "main"])),
        json!({"operation":"query","generation":2,"order":"unspecified","paths":[]})
    );
    assert_eq!(
        ok(f.run(&["query", f.store(), f.source(), "alpha", "new"])),
        json!({"operation":"query","generation":2,"order":"unspecified","paths":["/alpha/component-c/new.py"]})
    );
    assert_eq!(before, contents(&f.source));
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
#[test]
fn errors_are_explicit_and_failed_updates_preserve_previous_generation() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    fs::write(f.source.join("main.c"), b"main\n").unwrap();
    ok(f.run(&["create", f.parent(), "store", f.source(), "alpha"]));
    ok(f.run(&["index", f.store(), f.source(), "alpha", "none"]));
    let saved = contents(&f.source);
    error(
        f.run(&["query", f.store(), f.source(), "alpha", "*"]),
        4,
        "unsupported_or_limit",
    );
    error(
        f.run(&["index", f.store(), f.source(), "alpha", "99"]),
        4,
        "stale_generation",
    );
    error(
        f.run(&["index", f.store(), f.source(), "alpha", "invalid"]),
        2,
        "usage",
    );
    error(
        f.run(&["index", f.store(), f.source(), "alpha", "+1"]),
        2,
        "usage",
    );
    fs::write(f.source.join("unsupported.txt"), b"unsupported\n").unwrap();
    error(
        f.run(&["index", f.store(), f.source(), "alpha", "1"]),
        4,
        "unsupported_or_limit",
    );
    fs::remove_file(f.source.join("unsupported.txt")).unwrap();
    symlink(f.source.join("main.c"), f.source.join("link.c")).unwrap();
    error(
        f.run(&["index", f.store(), f.source(), "alpha", "1"]),
        4,
        "unsupported_or_limit",
    );
    fs::remove_file(f.source.join("link.c")).unwrap();
    assert_eq!(
        ok(f.run(&["query", f.store(), f.source(), "alpha", "main"])),
        json!({"operation":"query","generation":1,"order":"unspecified","paths":["/alpha/main.c"]})
    );
    assert_eq!(contents(&f.source), saved);
    let current = f.store.join("CURRENT");
    let bytes = fs::read(&current).unwrap();
    fs::write(&current, b"corrupt").unwrap();
    error(
        f.run(&["query", f.store(), f.source(), "alpha", "main"]),
        5,
        "corrupt_store",
    );
    fs::write(&current, bytes).unwrap();
    assert_eq!(
        ok(f.run(&["query", f.store(), f.source(), "alpha", "main"]))["generation"],
        json!(1)
    );
    assert_eq!(contents(&f.source), saved);
}

#[cfg(feature = "test-hooks")]
struct PausedIndex {
    child: std::process::Child,
    lines: std::sync::mpsc::Receiver<String>,
}
#[cfg(feature = "test-hooks")]
impl PausedIndex {
    fn start(f: &Fixture) -> Self {
        use std::{
            io::{BufRead, BufReader},
            process::Stdio,
            sync::mpsc,
            thread,
            time::Duration,
        };
        let mut child = Command::new(env!("CARGO_BIN_EXE_rustgrok"))
            .args(["index", f.store(), f.source(), "alpha", "1"])
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
        let first = rx.recv_timeout(Duration::from_secs(5));
        if first.as_deref() != Ok("RUSTGROK_TEST_PAUSED") {
            let _ = child.kill();
            let _ = child.wait();
            panic!("index did not pause: {first:?}");
        }
        Self { child, lines: rx }
    }
    fn release(&mut self) {
        use std::io::Write;
        self.child.stdin.as_mut().unwrap().write_all(b"x").unwrap();
    }
    fn stdout(&mut self) -> Vec<u8> {
        use std::io::Read;
        let mut data = Vec::new();
        self.child
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        data
    }
    fn next(&self) -> String {
        self.lines
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("child stderr timeout")
    }
}
#[cfg(feature = "test-hooks")]
impl Drop for PausedIndex {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(feature = "test-hooks")]
#[test]
fn competing_query_reports_busy_while_real_index_process_holds_store_lock() {
    let f = Fixture::new();
    fs::write(f.source.join("main.c"), b"main\n").unwrap();
    ok(f.run(&["create", f.parent(), "store", f.source(), "alpha"]));
    ok(f.run(&["index", f.store(), f.source(), "alpha", "none"]));
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    let saved = contents(&f.source);
    let mut running = PausedIndex::start(&f);
    error(
        f.run(&["query", f.store(), f.source(), "alpha", "main"]),
        3,
        "busy",
    );
    running.release();
    let status = running.child.wait().unwrap();
    assert!(status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&running.stdout()).unwrap(),
        json!({"operation":"index","generation":2})
    );
    assert_eq!(
        ok(f.run(&["query", f.store(), f.source(), "alpha", "extra"]))["generation"],
        json!(2)
    );
    assert_eq!(contents(&f.source), saved);
}

#[cfg(feature = "test-hooks")]
#[test]
fn sigint_cancels_real_index_and_keeps_old_generation() {
    let f = Fixture::new();
    fs::write(f.source.join("main.c"), b"main\n").unwrap();
    ok(f.run(&["create", f.parent(), "store", f.source(), "alpha"]));
    ok(f.run(&["index", f.store(), f.source(), "alpha", "none"]));
    fs::write(f.source.join("extra.js"), b"extra\n").unwrap();
    let saved = contents(&f.source);
    let mut running = PausedIndex::start(&f);
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
    assert_eq!(
        ok(f.run(&["query", f.store(), f.source(), "alpha", "extra"])),
        json!({"operation":"query","generation":1,"order":"unspecified","paths":[]})
    );
    assert_eq!(contents(&f.source), saved);
}

#[cfg(not(feature = "test-hooks"))]
#[test]
fn default_binary_ignores_test_pause_environment() {
    let f = Fixture::new();
    fs::write(f.source.join("main.c"), b"main\n").unwrap();
    ok(f.run(&["create", f.parent(), "store", f.source(), "alpha"]));
    let output = Command::new(env!("CARGO_BIN_EXE_rustgrok"))
        .args(["index", f.store(), f.source(), "alpha", "none"])
        .env("RUSTGROK_TEST_PAUSE", "1")
        .output()
        .unwrap();
    assert_eq!(ok(output), json!({"operation":"index","generation":1}));
}

#[cfg(feature = "test-hooks")]
#[test]
fn observed_source_change_aborts_real_index_before_publication() {
    let f = Fixture::new();
    fs::write(f.source.join("main.c"), b"main\n").unwrap();
    ok(f.run(&["create", f.parent(), "store", f.source(), "alpha"]));
    ok(f.run(&["index", f.store(), f.source(), "alpha", "none"]));
    fs::write(f.source.join("extra.js"), b"before\n").unwrap();
    let mut running = PausedIndex::start(&f);
    fs::write(f.source.join("extra.js"), b"different source content\n").unwrap();
    let expected = contents(&f.source);
    running.release();
    assert_eq!(running.child.wait().unwrap().code(), Some(5));
    assert!(running.stdout().is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&running.next()).unwrap(),
        json!({"error":"source_changed"})
    );
    assert_eq!(
        ok(f.run(&["query", f.store(), f.source(), "alpha", "extra"])),
        json!({"operation":"query","generation":1,"order":"unspecified","paths":[]})
    );
    assert_eq!(contents(&f.source), expected);
}
