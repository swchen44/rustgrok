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
                "rg-fhost-{}-{}",
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
        ok(self.run(&[
            "full-create",
            self.p(),
            "batch",
            self.s(),
            "utf8-default-tabs",
        ]));
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
        cmd.args(["full-host", f.t(), f.s(), f.c()])
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
            Some(json!({"operation":"full-host","state":"listening","published":published}))
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
        let value = ok(f.run(&["full-host-status", f.c()]));
        if value["job"]["id"] == id && value["job"]["state"] == state {
            return value;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("job {id} did not become {state}")
}
#[test]
fn full_host_indexes_postings_queries_and_restarts() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start(&f, "none", false);
    assert_eq!(
        ok(f.run(&["full-host-index", f.c(), "none", "alpha", "beta"]))["job"],
        1
    );
    assert_eq!(wait_job(&f, 1, "succeeded")["job"]["generation"], 1);
    assert_eq!(
        ok(f.run(&["full-host-query", f.c(), "alpha", "old"])),
        json!({"operation":"full-host-query","generation":1,"order":"unspecified","matches":[{"path":"/alpha/repo/old.c","source_sha256":"01d09d19c2139a46aebfb577780d123d7396e97201bc7ead210a2ebff8239dee","positions":[{"position":0,"start_utf16":0,"end_utf16":3,"start_byte":0,"end_byte":3}]}]})
    );
    fs::remove_file(f.source.join("alpha/repo/old.c")).unwrap();
    assert_eq!(
        ok(f.run(&["full-host-query", f.c(), "alpha", "old"]))["generation"],
        1
    );
    fs::write(f.source.join("alpha/repo/old.c"), b"old\n").unwrap();
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
    let mut host = Host::start(&f, "none", true);
    assert_eq!(
        ok(f.run(&["full-host-query", f.c(), "beta", "old"]))["generation"],
        1
    );
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

#[test]
fn full_host_expression_scopes_candidates_and_rejects_unsupported_syntax() {
    let f = Fixture::new();
    f.initialize();
    fs::write(
        f.source.join("alpha/repo/old.c"),
        b"old alpha alpha1 foo_bar\n",
    )
    .unwrap();
    let mut host = Host::start(&f, "none", false);
    ok(f.run(&["full-host-index", f.c(), "none", "alpha", "beta"]));
    wait_job(&f, 1, "succeeded");
    assert_eq!(
        ok(f.run(&[
            "full-host-query-expression",
            f.c(),
            "alpha",
            "old AND alpha"
        ])),
        json!({"operation":"full-host-query-expression","generation":1,"order":"unspecified","paths":["/alpha/repo/old.c"]})
    );
    assert_eq!(
        ok(f.run(&["full-host-query-expression", f.c(), "beta", "old AND alpha"]))["paths"],
        json!([])
    );
    assert_eq!(
        ok(f.run(&[
            "full-host-query-expression",
            f.c(),
            "alpha",
            "alpha1 AND foo_bar"
        ]))["paths"],
        json!(["/alpha/repo/old.c"])
    );
    failed(
        f.run(&[
            "full-host-query-expression",
            f.c(),
            "alpha",
            "old* OR alpha",
        ]),
        4,
        "unsupported_query",
    );
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

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
        let v = ok(f.run(&["full-host-status", f.c()]));
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
    fs::write(
        f.source.join("alpha/repo/public.c"),
        include_bytes!("../../../tests/fixtures/public-source/c/main.c"),
    )
    .unwrap();
    fs::write(
        f.source.join("beta/repo/public.py"),
        include_bytes!("../../../tests/fixtures/public-source/python/main.py.fixture"),
    )
    .unwrap();
    fs::write(
        f.source.join("alpha/repo/nested/public.js"),
        include_bytes!("../../../tests/fixtures/public-source/javascript/function.js"),
    )
    .unwrap();
    let host = Host::start(f, mode, false);
    ok(f.run(&["full-host-index", f.c(), "none", "alpha", "beta"]));
    assert_eq!(wait_job(f, 1, "succeeded")["job"]["generation"], 1);
    host
}

#[cfg(feature = "test-hooks")]
#[test]
fn old_batch_stays_queryable_during_second_scan_then_switches_together() {
    let f = Fixture::new();
    let mut host = started(&f, "pause");
    fs::write(f.source.join("alpha/repo/new.c"), b"freshbatchtoken\n").unwrap();
    fs::write(f.source.join("beta/repo/new.py"), b"freshbatchtoken\n").unwrap();
    let before = source_files(&f.source);
    assert_eq!(
        ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]))["job"],
        2
    );
    wait_paused(&f, 2);
    let mut children = Vec::new();
    for name in ["alpha", "beta", "alpha", "beta"] {
        children.push((
            name,
            Command::new(env!("CARGO_BIN_EXE_rustgrok"))
                .args(["full-host-query", f.c(), name, "old"])
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
            v["matches"][0]["source_sha256"],
            "01d09d19c2139a46aebfb577780d123d7396e97201bc7ead210a2ebff8239dee"
        );
        assert_eq!(
            v["matches"][0]["positions"],
            json!([{"position":0,"start_utf16":0,"end_utf16":3,"start_byte":0,"end_byte":3}])
        );
        assert_eq!(
            v["matches"][0]["path"],
            json!(format!(
                "/{name}/repo/old.{}",
                if name == "alpha" { "c" } else { "py" }
            ))
        );
    }
    assert_eq!(
        ok(f.run(&["full-host-query", f.c(), "alpha", "freshbatchtoken"]))["matches"],
        json!([])
    );
    let prior = ok(f.run(&[
        "full-host-query-expression",
        f.c(),
        "alpha",
        "old OR freshbatchtoken",
    ]));
    assert_eq!(prior["generation"], 1);
    assert_eq!(prior["paths"], json!(["/alpha/repo/old.c"]));
    ok(f.run(&["full-host-test-release", f.c()]));
    assert_eq!(wait_job(&f, 2, "succeeded")["job"]["generation"], 2);
    for name in ["alpha", "beta"] {
        let v = ok(f.run(&["full-host-query", f.c(), name, "freshbatchtoken"]));
        assert_eq!(v["generation"], 2);
        assert_eq!(v["matches"].as_array().unwrap().len(), 1);
    }
    let current = ok(f.run(&[
        "full-host-query-expression",
        f.c(),
        "alpha",
        "old OR freshbatchtoken",
    ]));
    assert_eq!(current["generation"], 2);
    assert_eq!(
        current["paths"],
        json!(["/alpha/repo/new.c", "/alpha/repo/old.c"])
    );
    assert_eq!(source_files(&f.source), before);
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
    let mut host = Host::start(&f, "none", true);
    for name in ["alpha", "beta"] {
        assert_eq!(
            ok(f.run(&["full-host-query", f.c(), name, "freshbatchtoken"]))["generation"],
            2
        );
    }
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn cancelled_failed_and_stale_batch_jobs_keep_both_old_projects() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let mut host = started(&f, "pause");
    let pointer = fs::read(f.store.join("CURRENT")).unwrap();
    fs::write(f.source.join("alpha/repo/new.c"), b"freshbatchtoken\n").unwrap();
    fs::write(f.source.join("beta/repo/new.py"), b"freshbatchtoken\n").unwrap();
    ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]));
    wait_paused(&f, 2);
    assert_eq!(
        ok(f.run(&["full-host-cancel", f.c()])),
        json!({"operation":"host-cancel","accepted":true})
    );
    assert_eq!(wait_job(&f, 2, "cancelled")["job"]["error"], "cancelled");
    fs::write(f.source.join("beta/repo/bad.txt"), b"bad\n").unwrap();
    ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]));
    wait_paused(&f, 3);
    ok(f.run(&["full-host-test-release", f.c()]));
    assert_eq!(
        wait_job(&f, 3, "failed")["job"]["error"],
        "unsupported_or_limit"
    );
    fs::remove_file(f.source.join("beta/repo/bad.txt")).unwrap();
    let file = f.source.join("beta/repo/new.py");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]));
    wait_paused(&f, 4);
    ok(f.run(&["full-host-test-release", f.c()]));
    assert_eq!(wait_job(&f, 4, "failed")["job"]["error"], "io_error");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    ok(f.run(&["full-host-index", f.c(), "99", "alpha", "beta"]));
    assert_eq!(
        wait_job(&f, 5, "failed")["job"]["error"],
        "stale_generation"
    );
    ok(f.run(&["full-host-index", f.c(), "1", "alpha"]));
    assert_eq!(
        wait_job(&f, 6, "failed")["job"]["error"],
        "unsupported_or_limit"
    );
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
    for name in ["alpha", "beta"] {
        let view = ok(f.run(&["full-host-query", f.c(), name, "old"]));
        assert_eq!(view["generation"], 1);
        assert_eq!(view["matches"].as_array().unwrap().len(), 1);
    }
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn worker_panic_and_indeterminate_block_reads_until_restart() {
    for (mode, label, exit, expected_gen, term) in [
        ("panic", "worker_panic", 5, 1, "old"),
        (
            "indeterminate",
            "publication_indeterminate",
            7,
            2,
            "freshbatchtoken",
        ),
    ] {
        let f = Fixture::new();
        let mut host = started(&f, mode);
        fs::write(f.source.join("alpha/repo/new.c"), b"freshbatchtoken\n").unwrap();
        fs::write(f.source.join("beta/repo/new.py"), b"freshbatchtoken\n").unwrap();
        ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]));
        assert_eq!(wait_job(&f, 2, "failed")["job"]["error"], label);
        failed(
            f.run(&["full-host-query", f.c(), "alpha", "old"]),
            exit,
            label,
        );
        failed(
            f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]),
            exit,
            label,
        );
        ok(f.run(&["full-host-stop", f.c()]));
        host.wait();
        let mut host = Host::start(&f, "none", true);
        for name in ["alpha", "beta"] {
            let view = ok(f.run(&["full-host-query", f.c(), name, term]));
            assert_eq!(view["generation"], expected_gen);
            assert_eq!(view["matches"].as_array().unwrap().len(), 1);
        }
        ok(f.run(&["full-host-stop", f.c()]));
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
    for frame in ["{\"command\":\"status\",\"command\":\"stop\"}\n", "{\"command\":\"status\",\"unknown\":1}\n", "{\"command\":\"full-index\",\"value\":\"{\\\"expected\\\":\\\"none\\\",\\\"expected\\\":\\\"none\\\",\\\"projects\\\":[\\\"alpha\\\"]}\"}\n"] {
        let mut stream=UnixStream::connect(f.control.join("control.sock")).unwrap();stream.write_all(frame.as_bytes()).unwrap();let mut raw=Vec::new();stream.read_to_end(&mut raw).unwrap();let response:Value=serde_json::from_slice(&raw).unwrap();assert_eq!(response["code"],4);assert_eq!(response["body"]["error"],"protocol_error");
    }
    assert_eq!(ok(f.run(&["full-host-status", f.c()]))["job"], Value::Null);
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

#[cfg(not(feature = "test-hooks"))]
#[test]
fn default_batch_host_ignores_pause_mode() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start(&f, "pause", false);
    ok(f.run(&["full-host-index", f.c(), "none", "alpha", "beta"]));
    wait_job(&f, 1, "succeeded");
    fs::write(f.source.join("alpha/repo/new.c"), b"freshbatchtoken\n").unwrap();
    ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]));
    wait_job(&f, 2, "succeeded");
    failed(f.run(&["full-host-test-release", f.c()]), 2, "usage");
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn reader_analysis_and_observed_source_changes_hold_the_old_full_batch() {
    let f = Fixture::new();
    let mut host = started(&f, "pause");
    let pointer = fs::read(f.store.join("CURRENT")).unwrap();
    for (i, bytes, label) in [
        (2, b"bad\xff".to_vec(), "unsupported_encoding"),
        (3, "é".as_bytes().to_vec(), "unsupported_full_input"),
        (4, "中".repeat(11000).into_bytes(), "unsupported_full_input"),
    ] {
        fs::write(f.source.join("beta/repo/old.py"), bytes).unwrap();
        ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]));
        wait_paused(&f, i);
        ok(f.run(&["full-host-test-release", f.c()]));
        assert_eq!(wait_job(&f, i, "failed")["job"]["error"], label);
        assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
        for project in ["alpha", "beta"] {
            let got = ok(f.run(&["full-host-query", f.c(), project, "old"]));
            assert_eq!(got["generation"], 1);
            assert_eq!(got["matches"].as_array().unwrap().len(), 1);
        }
    }
    fs::write(f.source.join("beta/repo/old.py"), b"old\n").unwrap();
    ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]));
    wait_paused(&f, 5);
    fs::write(
        f.source.join("alpha/repo/old.c"),
        b"changed after first project read\n",
    )
    .unwrap();
    let before = source_files(&f.source);
    ok(f.run(&["full-host-test-release", f.c()]));
    assert_eq!(wait_job(&f, 5, "failed")["job"]["error"], "source_changed");
    assert_eq!(fs::read(f.store.join("CURRENT")).unwrap(), pointer);
    assert_eq!(source_files(&f.source), before);
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

#[test]
fn full_host_rejects_other_query_modes_and_direct_writer_is_busy() {
    let f = Fixture::new();
    f.initialize();
    let mut host = Host::start(&f, "none", false);
    failed(
        f.run(&["full-index", f.t(), f.s(), "none", "alpha", "beta"]),
        3,
        "busy",
    );
    ok(f.run(&["full-host-index", f.c(), "none", "alpha", "beta"]));
    wait_job(&f, 1, "succeeded");
    failed(f.run(&["host-query", f.c(), "old"]), 2, "usage");
    failed(
        f.run(&["batch-host-query", f.c(), "alpha", "old"]),
        2,
        "usage",
    );
    failed(
        f.run(&["full-host-query", f.c(), "alpha", "old* OR x"]),
        4,
        "unsupported_query",
    );
    failed(
        f.run(&["full-host-query", f.c(), "unknown", "old"]),
        4,
        "unsupported_or_limit",
    );
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

#[test]
fn oversized_complete_full_positions_return_limit_without_partial_matches() {
    let f = Fixture::new();
    f.initialize();
    fs::write(f.source.join("alpha/repo/old.c"), "old ".repeat(15000)).unwrap();
    let mut host = Host::start(&f, "none", false);
    ok(f.run(&["full-host-index", f.c(), "none", "alpha", "beta"]));
    wait_job(&f, 1, "succeeded");
    failed(
        f.run(&["full-host-query", f.c(), "alpha", "old"]),
        4,
        "result_limit",
    );
    let got = ok(f.run(&["full-host-query", f.c(), "beta", "old"]));
    assert_eq!(got["generation"], 1);
    assert_eq!(got["matches"].as_array().unwrap().len(), 1);
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn host_keeps_old_full_postings_while_final_publication_sync_is_pending() {
    let f = Fixture::new();
    let mut host = started(&f, "publication-pause");
    fs::write(f.source.join("alpha/repo/new.c"), b"freshbatchtoken\n").unwrap();
    fs::write(f.source.join("beta/repo/new.py"), b"freshbatchtoken\n").unwrap();
    ok(f.run(&["full-host-index", f.c(), "1", "alpha", "beta"]));
    let mut paused = false;
    for _ in 0..150 {
        let v = ok(f.run(&["full-host-status", f.c()]));
        assert_ne!(
            v["job"]["state"], "succeeded",
            "publication-boundary gate missing"
        );
        if v["job"]["paused"] == true {
            paused = true;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(paused, "publication did not pause");
    for project in ["alpha", "beta"] {
        let old = ok(f.run(&["full-host-query", f.c(), project, "old"]));
        assert_eq!(old["generation"], 1);
        assert_eq!(old["matches"].as_array().unwrap().len(), 1);
        assert!(
            ok(f.run(&["full-host-query", f.c(), project, "freshbatchtoken"]))["matches"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(ok(f.run(&["full-host-cancel", f.c()]))["accepted"], false);
    ok(f.run(&["full-host-test-release", f.c()]));
    assert_eq!(wait_job(&f, 2, "succeeded")["job"]["generation"], 2);
    for project in ["alpha", "beta"] {
        let new = ok(f.run(&["full-host-query", f.c(), project, "freshbatchtoken"]));
        assert_eq!(new["generation"], 2);
        assert_eq!(new["matches"].as_array().unwrap().len(), 1);
    }
    ok(f.run(&["full-host-stop", f.c()]));
    host.wait();
}
