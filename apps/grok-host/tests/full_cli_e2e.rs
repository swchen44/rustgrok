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
                "rustgrok-full-cli-{}-{}",
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
fn source_full_postings_survive_restart_for_public_c_python_javascript() {
    let f = Fixture::new();
    f.git("alpha/repo");
    f.git("beta/repo");
    fs::write(
        f.source.join("alpha/repo/main.c"),
        include_bytes!("../../../tests/fixtures/public-source/c/main.c"),
    )
    .unwrap();
    fs::write(
        f.source.join("beta/repo/main.py"),
        include_bytes!("../../../tests/fixtures/public-source/python/main.py.fixture"),
    )
    .unwrap();
    fs::write(
        f.source.join("beta/repo/function.js"),
        include_bytes!("../../../tests/fixtures/public-source/javascript/function.js"),
    )
    .unwrap();
    let saved = files(&f.source);
    assert_eq!(
        ok(f.run(&[
            "full-create",
            f.parent(),
            "batch",
            f.src(),
            "utf8-default-tabs"
        ])),
        json!({"operation":"full-create","status":"created"})
    );
    assert_eq!(
        ok(f.run(&["full-index", f.store(), f.src(), "none", "alpha", "beta"])),
        json!({"operation":"full-index","generation":1})
    );
    let got = ok(f.run(&["full-query", f.store(), f.src(), "alpha", "MAIN"]));
    assert_eq!(got["generation"], 1);
    assert_eq!(got["order"], "unspecified");
    assert_eq!(got["matches"][0]["path"], "/alpha/repo/main.c");
    // Exact independent original oracle first `main` in this public file.
    assert_eq!(
        got["matches"][0]["positions"],
        json!([{ "position":6,"start_utf16":25,"end_utf16":29,"start_byte":25,"end_byte":29 }])
    );
    for term in ["def", "function"] {
        let got = ok(f.run(&["full-query", f.store(), f.src(), "beta", term]));
        assert_eq!(got["generation"], 1);
        assert!(!got["matches"].as_array().unwrap().is_empty());
    }
    // Query is answered from persisted derived search data after live files disappear.
    fs::remove_file(f.source.join("alpha/repo/main.c")).unwrap();
    let got = ok(f.run(&["full-query", f.store(), f.src(), "alpha", "main"]));
    assert_eq!(got["matches"][0]["path"], "/alpha/repo/main.c");
    fs::write(
        f.source.join("alpha/repo/main.c"),
        saved[Path::new("alpha/repo/main.c")].as_slice(),
    )
    .unwrap();
    assert_eq!(files(&f.source), saved);
}

#[test]
fn full_expression_returns_original_candidate_sets_after_restart() {
    let f = Fixture::new();
    for (name, text) in [
        ("alpha/beta/one.c", "/* alpha beta one */\n"),
        ("alpha/gamma/two.c", "/* alpha gamma two */\n"),
        ("beta/gamma/three.c", "/* beta gamma three */\n"),
        ("alpha/alone/four.c", "/* alpha alone four */\n"),
        ("other/delta/five.c", "/* other delta five */\n"),
    ] {
        let file = f.source.join("demo").join(name);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    fs::create_dir(f.source.join("otherproject")).unwrap();
    fs::write(f.source.join("otherproject/six.c"), "/* alpha beta */\n").unwrap();
    ok(f.run(&[
        "full-create",
        f.parent(),
        "batch",
        f.src(),
        "utf8-default-tabs",
    ]));
    ok(f.run(&[
        "full-index",
        f.store(),
        f.src(),
        "none",
        "demo",
        "otherproject",
    ]));
    // Expected sets captured from OpenGrok 1.14.20 HTTP, not ranked order.
    for (query, expected) in [
        (
            "ALPHA",
            vec![
                "/demo/alpha/alone/four.c",
                "/demo/alpha/beta/one.c",
                "/demo/alpha/gamma/two.c",
            ],
        ),
        ("alpha beta", vec!["/demo/alpha/beta/one.c"]),
        ("alpha AND beta", vec!["/demo/alpha/beta/one.c"]),
        (
            "alpha OR beta",
            vec![
                "/demo/alpha/alone/four.c",
                "/demo/alpha/beta/one.c",
                "/demo/alpha/gamma/two.c",
                "/demo/beta/gamma/three.c",
            ],
        ),
        (
            "alpha NOT beta",
            vec!["/demo/alpha/alone/four.c", "/demo/alpha/gamma/two.c"],
        ),
        (
            "alpha AND (beta OR gamma)",
            vec!["/demo/alpha/beta/one.c", "/demo/alpha/gamma/two.c"],
        ),
        (
            "(alpha OR beta) AND gamma",
            vec!["/demo/alpha/gamma/two.c", "/demo/beta/gamma/three.c"],
        ),
        (
            "alpha -beta",
            vec!["/demo/alpha/alone/four.c", "/demo/alpha/gamma/two.c"],
        ),
        ("alpha and beta", vec![]),
        ("NOT alpha", vec![]),
        ("-alpha", vec![]),
    ] {
        let got = ok(f.run(&["full-query-expression", f.store(), f.src(), "demo", query]));
        assert_eq!(got["generation"], 1);
        assert_eq!(got["order"], "unspecified");
        let mut paths: Vec<_> = got["paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        paths.sort_unstable();
        assert_eq!(paths, expected, "{query}");
    }
    for query in ["alpha*", "alpha OR beta AND gamma", "alpha\tbeta"] {
        failed(
            f.run(&["full-query-expression", f.store(), f.src(), "demo", query]),
            4,
            "unsupported_query",
        );
    }
}

#[test]
fn full_expression_identifiers_with_digits_and_underscores_match_original() {
    let f = Fixture::new();
    for (name, text) in [
        ("alpha/beta/one.c", "/* alpha1 foo_bar one */\n"),
        ("alpha/gamma/two.c", "/* alpha1 foo_baz two */\n"),
        ("beta/gamma/three.c", "/* foo_bar other2 three */\n"),
        ("alpha/alone/four.c", "/* _hidden foo42 four */\n"),
        ("other/delta/five.c", "/* a1b _hidden five */\n"),
    ] {
        let file = f.source.join("demo").join(name);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    ok(f.run(&[
        "full-create",
        f.parent(),
        "batch",
        f.src(),
        "utf8-default-tabs",
    ]));
    ok(f.run(&["full-index", f.store(), f.src(), "none", "demo"]));
    for (query, expected) in [
        (
            "ALPHA1",
            vec!["/demo/alpha/beta/one.c", "/demo/alpha/gamma/two.c"],
        ),
        (
            "foo_bar",
            vec!["/demo/alpha/beta/one.c", "/demo/beta/gamma/three.c"],
        ),
        (
            "_hidden",
            vec!["/demo/alpha/alone/four.c", "/demo/other/delta/five.c"],
        ),
        ("foo42", vec!["/demo/alpha/alone/four.c"]),
        ("123", vec![]),
        ("alpha1 AND foo_bar", vec!["/demo/alpha/beta/one.c"]),
        (
            "alpha1 OR foo_bar",
            vec![
                "/demo/alpha/beta/one.c",
                "/demo/alpha/gamma/two.c",
                "/demo/beta/gamma/three.c",
            ],
        ),
        ("foo_bar NOT alpha1", vec!["/demo/beta/gamma/three.c"]),
        (
            "foo_bar AND (other2 OR alpha1)",
            vec!["/demo/alpha/beta/one.c", "/demo/beta/gamma/three.c"],
        ),
    ] {
        let got = ok(f.run(&["full-query-expression", f.store(), f.src(), "demo", query]));
        let mut paths: Vec<_> = got["paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap())
            .collect();
        paths.sort_unstable();
        assert_eq!(paths, expected, "{query}");
    }
    // The original splits this word into two terms; this bounded slice must not approximate it.
    failed(
        f.run(&[
            "full-query-expression",
            f.store(),
            f.src(),
            "demo",
            "1alpha",
        ]),
        4,
        "unsupported_query",
    );
}

#[test]
fn source_reader_bom_tabs_and_unicode_positions_match_original() {
    let f = Fixture::new();
    f.git("alpha");
    f.git("beta");
    fs::write(f.source.join("alpha/a.c"), b"\xef\xbb\xbfAlpha\tBeta\r\n").unwrap();
    fs::write(f.source.join("beta/b.py"), "😀\tİ\r\nAΣ 中\n".as_bytes()).unwrap();
    ok(f.run(&[
        "full-create",
        f.parent(),
        "batch",
        f.src(),
        "utf8-default-tabs",
    ]));
    ok(f.run(&["full-index", f.store(), f.src(), "none", "alpha", "beta"]));
    let got = ok(f.run(&["full-query", f.store(), f.src(), "alpha", "Beta"]));
    assert_eq!(
        got["matches"][0]["positions"],
        json!([{ "position":1,"start_utf16":6,"end_utf16":10,"start_byte":9,"end_byte":13 }])
    );
    let got = ok(f.run(&["full-query", f.store(), f.src(), "beta", "İ"]));
    assert_eq!(
        got["matches"][0]["positions"],
        json!([{ "position":0,"start_utf16":3,"end_utf16":5,"start_byte":5,"end_byte":7 }])
    );
    let got = ok(f.run(&["full-query", f.store(), f.src(), "beta", "AΣ"]));
    assert_eq!(
        got["matches"][0]["positions"],
        json!([{ "position":1,"start_utf16":6,"end_utf16":8,"start_byte":9,"end_byte":12 }])
    );
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
fn second_project_bad_source_and_limits_hold_entire_old_batch() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.git("alpha");
    f.git("beta");
    fs::write(f.source.join("alpha/a.c"), b"old alpha").unwrap();
    fs::write(f.source.join("beta/b.py"), b"old beta").unwrap();
    ok(f.run(&[
        "full-create",
        f.parent(),
        "batch",
        f.src(),
        "utf8-default-tabs",
    ]));
    ok(f.run(&["full-index", f.store(), f.src(), "none", "alpha", "beta"]));
    let pointer = fs::read(f.store.join("CURRENT")).unwrap();
    fs::write(f.source.join("alpha/a.c"), b"new alpha").unwrap();
    for (bytes, label) in [
        (vec![0xff, 0xfe, b'A', 0], "unsupported_encoding"),
        (b"new \xff beta".to_vec(), "unsupported_encoding"),
        ("new é".as_bytes().to_vec(), "unsupported_full_input"),
        ("中".repeat(11000).into_bytes(), "unsupported_full_input"),
        (vec![b'a'; 8 * 1024 * 1024 + 1], "unsupported_or_limit"),
    ] {
        fs::write(f.source.join("beta/b.py"), bytes).unwrap();
        let source = files(&f.source);
        failed(
            f.run(&["full-index", f.store(), f.src(), "1", "alpha", "beta"]),
            4,
            label,
        );
        assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
        assert_eq!(files(&f.source), source);
        for p in ["alpha", "beta"] {
            let got = ok(f.run(&["full-query", f.store(), f.src(), p, "old"]));
            assert_eq!(got["generation"], 1);
            assert_eq!(got["matches"].as_array().unwrap().len(), 1);
            assert!(
                ok(f.run(&["full-query", f.store(), f.src(), p, "new"]))["matches"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
    }
    fs::write(f.source.join("beta/b.py"), b"new beta").unwrap();
    fs::set_permissions(
        f.source.join("beta/b.py"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    failed(
        f.run(&["full-index", f.store(), f.src(), "1", "alpha", "beta"]),
        6,
        "io_error",
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
    fs::set_permissions(
        f.source.join("beta/b.py"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(
        ok(f.run(&["full-index", f.store(), f.src(), "1", "beta", "alpha"]))["generation"],
        2
    );
    for p in ["alpha", "beta"] {
        let got = ok(f.run(&["full-query", f.store(), f.src(), p, "new"]));
        assert_eq!(got["generation"], 2);
        assert_eq!(got["matches"].as_array().unwrap().len(), 1);
    }
}

#[cfg(feature = "test-hooks")]
struct PausedFull {
    child: std::process::Child,
    lines: std::sync::mpsc::Receiver<String>,
}
#[cfg(feature = "test-hooks")]
impl PausedFull {
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
            .args(["full-index", f.store(), f.src(), "1", "alpha", "beta"])
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
            .expect("full stderr timeout")
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
impl Drop for PausedFull {
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
    ok(f.run(&[
        "full-create",
        f.parent(),
        "batch",
        f.src(),
        "utf8-default-tabs",
    ]));
    ok(f.run(&["full-index", f.store(), f.src(), "none", "alpha", "beta"]));
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
    let mut running = PausedFull::start(&f, true);
    failed(
        f.run(&["full-query", f.store(), f.src(), "alpha", "old"]),
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
        let view = ok(f.run(&["full-query", f.store(), f.src(), name, "old"]));
        assert_eq!(view["generation"], 1);
        assert_eq!(view["matches"].as_array().unwrap().len(), 1);
        assert!(
            ok(f.run(&["full-query", f.store(), f.src(), name, "new"]))["matches"]
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
    let mut running = PausedFull::start(&f, false);
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
            ok(f.run(&["full-query", f.store(), f.src(), name, "old"]))["generation"],
            1
        );
    }
    assert_eq!(files(&f.source), saved);
}

#[cfg(feature = "test-hooks")]
#[test]
fn process_death_on_both_sides_of_pointer_commit_recovers_one_complete_batch() {
    for (fault, generation, term) in [
        ("after-generation-sync", 1, "old"),
        ("before-rename", 1, "old"),
        ("after-rename", 2, "new"),
    ] {
        let f = indexed_fixture();
        let pointer = fs::read(f.store.join("CURRENT")).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_rustgrok"))
            .args(["full-index", f.store(), f.src(), "1", "alpha", "beta"])
            .env("RUSTGROK_TEST_FULL_FAULT", fault)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(91), "{fault}");
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        if generation == 1 {
            assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
        }
        for p in ["alpha", "beta"] {
            let view = ok(f.run(&["full-query", f.store(), f.src(), p, term]));
            assert_eq!(view["generation"], generation);
            assert!(!view["matches"].as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn restricted_query_reader_configuration_and_path_stores_are_separate() {
    let f = Fixture::new();
    f.git("alpha");
    fs::write(f.source.join("alpha/a.c"), b"old term").unwrap();
    failed(
        f.run(&["full-create", f.parent(), "batch", f.src(), "utf16-tabs4"]),
        4,
        "unsupported_reader",
    );
    assert!(!f.store.exists());
    ok(f.run(&["batch-create", f.parent(), "legacy", f.src()]));
    let legacy = f.parent.join("legacy");
    let legacy_str = legacy.to_str().unwrap();
    ok(f.run(&["batch-index", legacy_str, f.src(), "none", "alpha"]));
    let old = files(&legacy);
    failed(
        f.run(&["full-query", legacy_str, f.src(), "alpha", "old"]),
        5,
        "corrupt_store",
    );
    ok(f.run(&[
        "full-create",
        f.parent(),
        "batch",
        f.src(),
        "utf8-default-tabs",
    ]));
    failed(
        f.run(&["full-query", f.store(), f.src(), "alpha", "old"]),
        4,
        "no_generation",
    );
    ok(f.run(&["full-index", f.store(), f.src(), "none", "alpha"]));
    for q in [
        "old term",
        "old*",
        "full:old",
        "old OR term",
        "\"old\"",
        "AND",
        "old\n",
        "a+b",
        "😀",
        "AAΣ",
        "",
    ] {
        failed(
            f.run(&["full-query", f.store(), f.src(), "alpha", q]),
            4,
            "unsupported_query",
        );
    }
    failed(
        f.run(&["batch-query", f.store(), f.src(), "alpha", "old"]),
        5,
        "corrupt_store",
    );
    assert_eq!(files(&legacy), old);
    assert_eq!(
        ok(f.run(&["batch-query", legacy_str, f.src(), "alpha", "c"]))["generation"],
        1
    );
}

#[cfg(not(feature = "test-hooks"))]
#[test]
fn default_full_binary_ignores_test_environment() {
    let f = Fixture::new();
    f.git("alpha");
    fs::write(f.source.join("alpha/a.c"), b"old").unwrap();
    ok(f.run(&[
        "full-create",
        f.parent(),
        "batch",
        f.src(),
        "utf8-default-tabs",
    ]));
    let output = Command::new(env!("CARGO_BIN_EXE_rustgrok"))
        .args(["full-index", f.store(), f.src(), "none", "alpha"])
        .env("RUSTGROK_TEST_PAUSE", "1")
        .env("RUSTGROK_TEST_FULL_FAULT", "after-rename")
        .output()
        .unwrap();
    assert_eq!(ok(output)["generation"], 1);
}
