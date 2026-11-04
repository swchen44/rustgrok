/*
 * CDDL HEADER START
 *
 * The contents of this file are subject to the terms of the
 * Common Development and Distribution License (the "License").
 * You may not use this file except in compliance with the License.
 * See LICENSE.txt included in this distribution for the specific
 * language governing permissions and limitations under the License.
 * When distributing Covered Code, include this CDDL HEADER in each
 * file and include the License file at LICENSE.txt.
 * If applicable, add the following below this CDDL HEADER, with the
 * fields enclosed by brackets "[]" replaced with your own identifying
 * information: Portions Copyright [yyyy] [name of copyright owner]
 *
 * CDDL HEADER END
 */
/*
 * Copyright (c) 2008, 2025, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2017, 2020, Chris Fraire <cfraire@me.com>.
 * Portions Copyright (c) 2026, swchen44.
 * Derived behavior and source provenance: OpenGrok 1.14.20,
 * commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3.
 */
// SPDX-License-Identifier: CDDL-1.0
#![cfg(target_os = "macos")]
use rustgrok_path_index::durable::DurablePathIndex;
use rustgrok_path_index::{CancellationToken, IndexLimits};
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    source: PathBuf,
    parent: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "rustgrok-scanner-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let parent = root.join("indexes");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&parent).unwrap();
        Self {
            root,
            source,
            parent,
        }
    }
    fn git(&self, name: &str) {
        let path = self.source.join(name);
        fs::create_dir_all(&path).unwrap();
        let output = Command::new("git")
            .args(["init", "--quiet"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success(), "git init failed");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn limits() -> IndexLimits {
    IndexLimits {
        max_documents: 10,
        max_path_bytes: 256,
        max_total_path_bytes: 1024,
        max_live_generations: 3,
    }
}
fn set(paths: &[&str]) -> HashSet<String> {
    paths.iter().map(|s| (*s).to_owned()).collect()
}
#[test]
fn nested_git_source_scan_add_remove_and_restart() {
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
    let db =
        DurablePathIndex::create(&f.parent, "store", &f.source, "alpha", limits(), 64).unwrap();
    assert_eq!(
        db.replace_from_source(None, &CancellationToken::new())
            .unwrap(),
        1
    );
    let prior = db.snapshot().unwrap();
    assert_eq!(
        fs::read(f.source.join("component-c/main.c")).unwrap(),
        include_bytes!("../../../tests/fixtures/public-source/c/main.c")
    );
    assert_eq!(
        fs::read(f.source.join("component-c/nested-js/function.js")).unwrap(),
        include_bytes!("../../../tests/fixtures/public-source/javascript/function.js")
    );
    assert_eq!(
        prior.manifest(),
        set(&[
            "/alpha/component-c/main.c",
            "/alpha/component-c/nested-js/function.js",
            "/alpha/component-js/function.js"
        ])
    );
    assert_eq!(
        prior.lookup("function").unwrap(),
        set(&[
            "/alpha/component-c/nested-js/function.js",
            "/alpha/component-js/function.js"
        ])
    );
    fs::remove_file(f.source.join("component-js/function.js")).unwrap();
    fs::write(
        f.source.join("component-js/main.py"),
        include_bytes!("../../../tests/fixtures/public-source/python/main.py.fixture"),
    )
    .unwrap();
    assert_eq!(
        db.replace_from_source(Some(1), &CancellationToken::new())
            .unwrap(),
        2
    );
    assert_eq!(prior.lookup("function").unwrap().len(), 2);
    assert_eq!(
        db.snapshot().unwrap().lookup("function").unwrap(),
        set(&["/alpha/component-c/nested-js/function.js"])
    );
    drop(db);
    let db =
        DurablePathIndex::open(&f.parent.join("store"), &f.source, "alpha", limits(), 64).unwrap();
    assert_eq!(
        db.snapshot().unwrap().manifest(),
        set(&[
            "/alpha/component-c/main.c",
            "/alpha/component-c/nested-js/function.js",
            "/alpha/component-js/main.py"
        ])
    );
}

#[test]
fn unsupported_entries_fail_without_replacing_durable_generation() {
    use std::{
        ffi::CString,
        os::unix::{
            ffi::OsStrExt,
            fs::{symlink, PermissionsExt},
        },
    };
    for kind in [
        "symlink",
        "fifo",
        "hidden",
        "unicode",
        "unreadable",
        "unsupported-extension",
    ] {
        let f = Fixture::new();
        fs::write(f.source.join("main.c"), b"main\n").unwrap();
        let db =
            DurablePathIndex::create(&f.parent, "store", &f.source, "alpha", limits(), 64).unwrap();
        db.replace_from_source(None, &CancellationToken::new())
            .unwrap();
        let old = db.snapshot().unwrap();
        let bad = f.source.join(match kind {
            "symlink" => "bad.c",
            "fifo" => "bad.c",
            "hidden" => ".hidden",
            "unicode" => "檔案.c",
            "unreadable" => "bad.c",
            _ => "bad.txt",
        });
        match kind {
            "symlink" => symlink(f.source.join("main.c"), &bad).unwrap(),
            "fifo" => {
                let path = CString::new(bad.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            _ => {
                fs::write(&bad, b"x").unwrap();
                if kind == "unreadable" {
                    fs::set_permissions(&bad, fs::Permissions::from_mode(0o0)).unwrap();
                }
            }
        }
        assert!(
            db.replace_from_source(Some(1), &CancellationToken::new())
                .is_err(),
            "{kind}"
        );
        assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
        assert_eq!(old.manifest(), set(&["/alpha/main.c"]));
        drop(db);
        assert_eq!(
            DurablePathIndex::open(&f.parent.join("store"), &f.source, "alpha", limits(), 64)
                .unwrap()
                .snapshot()
                .unwrap()
                .manifest(),
            set(&["/alpha/main.c"])
        );
        if kind == "unreadable" {
            fs::set_permissions(&bad, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
}

#[cfg(feature = "fault-injection")]
#[test]
fn observed_source_change_before_publication_preserves_prior_generation() {
    let f = Fixture::new();
    fs::write(f.source.join("main.c"), b"original\n").unwrap();
    let db =
        DurablePathIndex::create(&f.parent, "store", &f.source, "alpha", limits(), 64).unwrap();
    db.replace_from_source(None, &CancellationToken::new())
        .unwrap();
    fs::write(f.source.join("extra.js"), b"function example() {}\n").unwrap();
    let pointer = fs::read(f.parent.join("store/CURRENT")).unwrap();
    let mut modify = || fs::write(f.source.join("extra.js"), b"different content\n").unwrap();
    let result = db.replace_from_source_with_precommit_observer(
        Some(1),
        &CancellationToken::new(),
        &mut modify,
    );
    assert!(matches!(
        result,
        Err(rustgrok_path_index::durable::DurableError::SourceChanged)
    ));
    assert_eq!(pointer, fs::read(f.parent.join("store/CURRENT")).unwrap());
    assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
}

#[cfg(feature = "fault-injection")]
#[test]
fn old_generation_remains_queryable_during_scanner_build_and_second_writer_is_busy() {
    use std::{
        sync::{mpsc, Arc},
        thread,
        time::Duration,
    };
    let f = Fixture::new();
    fs::write(f.source.join("main.c"), b"original\n").unwrap();
    let db = Arc::new(
        DurablePathIndex::create(&f.parent, "store", &f.source, "alpha", limits(), 64).unwrap(),
    );
    db.replace_from_source(None, &CancellationToken::new())
        .unwrap();
    fs::write(f.source.join("extra.js"), b"more\n").unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let worker = Arc::clone(&db);
    let thread = thread::spawn(move || {
        let mut pause = || {
            ready_tx.send(()).unwrap();
            resume_rx.recv().unwrap();
        };
        worker.replace_from_source_with_precommit_observer(
            Some(1),
            &CancellationToken::new(),
            &mut pause,
        )
    });
    ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
    assert!(matches!(
        db.replace_from_source(Some(1), &CancellationToken::new()),
        Err(rustgrok_path_index::durable::DurableError::Busy)
    ));
    resume_tx.send(()).unwrap();
    assert_eq!(thread.join().unwrap().unwrap(), 2);
    assert_eq!(
        db.snapshot().unwrap().manifest(),
        set(&["/alpha/main.c", "/alpha/extra.js"])
    );
}
