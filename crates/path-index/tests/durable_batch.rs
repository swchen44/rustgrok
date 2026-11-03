// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
#![cfg(target_os = "macos")]
use rustgrok_path_index::durable::batch::{BatchLimits, BatchProject, DurableBatchIndex};
use rustgrok_path_index::{CancellationToken, IndexLimits};
use std::{
    collections::HashSet,
    fs, io,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
    thread,
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
                "rustgrok-batch-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let parent = root.join("stores");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&parent).unwrap();
        Self {
            root,
            source,
            parent,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn limits() -> BatchLimits {
    BatchLimits {
        per_project: IndexLimits {
            max_documents: 20,
            max_path_bytes: 128,
            max_total_path_bytes: 2048,
            max_live_generations: 3,
        },
        max_projects: 4,
        max_batch_documents: 40,
        max_batch_path_bytes: 4096,
    }
}
fn project(name: &str, paths: &[&str]) -> BatchProject<Vec<io::Result<String>>> {
    BatchProject {
        project: name.to_owned(),
        expected_count: paths.len(),
        paths: paths.iter().map(|p| Ok((*p).to_owned())).collect(),
    }
}
fn set(paths: &[&str]) -> HashSet<String> {
    paths.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn one_commit_publishes_two_projects_and_old_readers_remain_consistent() {
    let f = Fixture::new();
    let db =
        Arc::new(DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap());
    let initial = vec![
        project("alpha", &["/alpha/old.c"]),
        project("beta", &["/beta/old.c"]),
    ];
    assert_eq!(
        db.replace(None, initial, &CancellationToken::new())
            .unwrap(),
        1
    );
    let old = db.snapshot().unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let worker = Arc::clone(&db);
    let join = thread::spawn(move || {
        let mut first = true;
        let mut alpha = vec![Ok("/alpha/new.c".to_string())].into_iter();
        let mut beta = vec![Ok("/beta/new.c".to_string())].into_iter();
        let alpha_iter = std::iter::from_fn(move || alpha.next());
        let beta_iter = std::iter::from_fn(move || {
            if first {
                first = false;
                started_tx.send(()).unwrap();
                resume_rx.recv().unwrap();
            }
            beta.next()
        });
        let inputs: Vec<BatchProject<Box<dyn Iterator<Item = io::Result<String>> + Send>>> = vec![
            BatchProject {
                project: "alpha".into(),
                expected_count: 1,
                paths: Box::new(alpha_iter),
            },
            BatchProject {
                project: "beta".into(),
                expected_count: 1,
                paths: Box::new(beta_iter),
            },
        ];
        worker.replace(Some(1), inputs, &CancellationToken::new())
    });
    started_rx.recv().unwrap();
    let during = db.snapshot().unwrap();
    assert_eq!(during.generation(), 1);
    for name in ["alpha", "beta"] {
        assert_eq!(
            during.lookup(name, "old").unwrap(),
            set(&[&format!("/{name}/old.c")])
        );
        assert!(during.lookup(name, "new").unwrap().is_empty());
    }
    resume_tx.send(()).unwrap();
    assert_eq!(join.join().unwrap().unwrap(), 2);
    let current = db.snapshot().unwrap();
    assert_eq!(current.generation(), 2);
    for name in ["alpha", "beta"] {
        assert_eq!(
            current.lookup(name, "new").unwrap(),
            set(&[&format!("/{name}/new.c")])
        );
        assert_eq!(
            old.lookup(name, "old").unwrap(),
            set(&[&format!("/{name}/old.c")])
        );
    }
}

#[test]
fn failures_cancellation_and_changed_registry_keep_entire_old_batch() {
    let f = Fixture::new();
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
    db.replace(
        None,
        vec![
            project("alpha", &["/alpha/old.c"]),
            project("beta", &["/beta/old.c"]),
        ],
        &CancellationToken::new(),
    )
    .unwrap();
    let beta_errors: Vec<BatchProject<Vec<io::Result<String>>>> = vec![
        BatchProject {
            project: "beta".into(),
            expected_count: 1,
            paths: vec![Err(io::Error::other("injected read failure"))],
        },
        project("beta", &["/beta/../escape.c"]),
        project("beta", &["/beta/é.c"]),
        BatchProject {
            project: "beta".into(),
            expected_count: 2,
            paths: vec![Ok("/beta/new.c".into())],
        },
        project("beta", &["/alpha/foreign.c"]),
    ];
    for beta in beta_errors {
        assert!(db
            .replace(
                Some(1),
                vec![project("alpha", &["/alpha/new.c"]), beta],
                &CancellationToken::new()
            )
            .is_err());
        let view = db.snapshot().unwrap();
        assert_eq!(view.generation(), 1);
        for name in ["alpha", "beta"] {
            assert_eq!(
                view.lookup(name, "old").unwrap(),
                set(&[&format!("/{name}/old.c")])
            );
        }
    }
    for inputs in [
        vec![project("alpha", &["/alpha/new.c"])],
        vec![project("alpha", &[]), project("alpha", &[])],
        vec![project("alpha", &[]), project("gamma", &[])],
    ] {
        assert!(db
            .replace(Some(1), inputs, &CancellationToken::new())
            .is_err());
        assert_eq!(db.snapshot().unwrap().generation(), 1);
    }
    let cancel = CancellationToken::new();
    let alpha = vec![Ok("/alpha/new.c".to_owned())].into_iter();
    let beta = std::iter::once_with(|| {
        cancel.cancel();
        Ok("/beta/new.c".to_owned())
    });
    let batch: Vec<BatchProject<Box<dyn Iterator<Item = io::Result<String>> + '_>>> = vec![
        BatchProject {
            project: "alpha".into(),
            expected_count: 1,
            paths: Box::new(alpha),
        },
        BatchProject {
            project: "beta".into(),
            expected_count: 1,
            paths: Box::new(beta),
        },
    ];
    assert!(db.replace(Some(1), batch, &cancel).is_err());
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.generation(), 1);
    for name in ["alpha", "beta"] {
        assert_eq!(
            snapshot.lookup(name, "old").unwrap(),
            set(&[&format!("/{name}/old.c")])
        );
    }
}

#[test]
fn batch_restart_and_store_types_remain_separate() {
    use rustgrok_path_index::durable::DurablePathIndex;
    let f = Fixture::new();
    fs::write(f.source.join("unchanged.c"), b"original\n").unwrap();
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
    db.replace(
        None,
        vec![
            project("alpha", &["/alpha/a.c"]),
            project("beta", &["/beta/b.c"]),
        ],
        &CancellationToken::new(),
    )
    .unwrap();
    drop(db);
    assert!(DurablePathIndex::open(
        &f.parent.join("batch"),
        &f.source,
        "rg_batch",
        limits().per_project,
        100
    )
    .is_err());
    let reopened =
        DurableBatchIndex::open(&f.parent.join("batch"), &f.source, limits(), 100).unwrap();
    let batch = reopened.snapshot().unwrap();
    assert_eq!(batch.projects(), vec!["alpha", "beta"]);
    assert_eq!(batch.lookup("alpha", "a").unwrap(), set(&["/alpha/a.c"]));
    assert_eq!(batch.lookup("beta", "b").unwrap(), set(&["/beta/b.c"]));
    drop(reopened);
    let single = DurablePathIndex::create(
        &f.parent,
        "single",
        &f.source,
        "rg_batch",
        limits().per_project,
        100,
    )
    .unwrap();
    single
        .replace(
            None,
            1,
            vec![Ok("/rg_batch/file.c".to_owned())],
            &CancellationToken::new(),
        )
        .unwrap();
    drop(single);
    assert!(DurableBatchIndex::open(&f.parent.join("single"), &f.source, limits(), 100).is_err());
    let single = DurablePathIndex::open(
        &f.parent.join("single"),
        &f.source,
        "rg_batch",
        limits().per_project,
        100,
    )
    .unwrap();
    assert_eq!(
        single.snapshot().unwrap().lookup("file").unwrap(),
        set(&["/rg_batch/file.c"])
    );
    assert_eq!(
        fs::read(f.source.join("unchanged.c")).unwrap(),
        b"original\n"
    );
}

#[test]
fn project_scope_and_path_tokens_exclude_internal_store_prefix() {
    let f = Fixture::new();
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
    db.replace(
        None,
        vec![
            project("alpha", &["/alpha/shared.c", "/alpha/only.c"]),
            project("beta", &["/beta/shared.c"]),
        ],
        &CancellationToken::new(),
    )
    .unwrap();
    let view = db.snapshot().unwrap();
    assert_eq!(
        view.lookup("alpha", "shared").unwrap(),
        set(&["/alpha/shared.c"])
    );
    assert_eq!(
        view.lookup("beta", "shared").unwrap(),
        set(&["/beta/shared.c"])
    );
    assert_eq!(
        view.lookup("alpha", "alpha").unwrap(),
        set(&["/alpha/shared.c", "/alpha/only.c"])
    );
    assert!(view.lookup("alpha", "beta").unwrap().is_empty());
    assert!(view.lookup("alpha", "rg_batch").unwrap().is_empty());
    assert!(view.lookup("missing", "shared").is_err());
    assert_eq!(
        view.lookup_expression("beta", "beta AND shared").unwrap(),
        set(&["/beta/shared.c"])
    );
}

#[test]
fn concurrent_writer_is_busy_and_stale_writer_cannot_publish() {
    use rustgrok_path_index::durable::DurableError;
    let f = Fixture::new();
    let db =
        Arc::new(DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap());
    db.replace(
        None,
        vec![
            project("alpha", &["/alpha/old.c"]),
            project("beta", &["/beta/old.c"]),
        ],
        &CancellationToken::new(),
    )
    .unwrap();
    let (start_tx, start_rx) = mpsc::channel();
    let (finish_tx, finish_rx) = mpsc::channel();
    let worker = Arc::clone(&db);
    let join = thread::spawn(move || {
        let mut input = vec![Ok("/beta/new.c".to_owned())].into_iter();
        let mut first = true;
        let beta = std::iter::from_fn(move || {
            if first {
                first = false;
                start_tx.send(()).unwrap();
                finish_rx.recv().unwrap();
            }
            input.next()
        });
        let a = vec![Ok("/alpha/new.c".to_owned())].into_iter();
        let items: Vec<BatchProject<Box<dyn Iterator<Item = io::Result<String>> + Send>>> = vec![
            BatchProject {
                project: "alpha".into(),
                expected_count: 1,
                paths: Box::new(a),
            },
            BatchProject {
                project: "beta".into(),
                expected_count: 1,
                paths: Box::new(beta),
            },
        ];
        worker.replace(Some(1), items, &CancellationToken::new())
    });
    start_rx.recv().unwrap();
    assert!(matches!(
        db.replace(
            Some(1),
            vec![project("alpha", &[]), project("beta", &[])],
            &CancellationToken::new()
        ),
        Err(DurableError::Busy)
    ));
    finish_tx.send(()).unwrap();
    assert_eq!(join.join().unwrap().unwrap(), 2);
    assert!(db
        .replace(
            Some(1),
            vec![project("alpha", &[]), project("beta", &[])],
            &CancellationToken::new()
        )
        .is_err());
    let current = db.snapshot().unwrap();
    assert_eq!(current.generation(), 2);
    for name in ["alpha", "beta"] {
        assert_eq!(
            current.lookup(name, "new").unwrap(),
            set(&[&format!("/{name}/new.c")])
        );
    }
}

#[test]
fn reopening_with_stricter_project_limits_rejects_incomplete_generation() {
    let f = Fixture::new();
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
    db.replace(
        None,
        vec![
            project("alpha", &["/alpha/a.c", "/alpha/b.c"]),
            project("beta", &["/beta/c.c"]),
        ],
        &CancellationToken::new(),
    )
    .unwrap();
    drop(db);
    let mut narrow = limits();
    narrow.per_project.max_documents = 1;
    assert!(DurableBatchIndex::open(&f.parent.join("batch"), &f.source, narrow, 100).is_err());
    let reopened =
        DurableBatchIndex::open(&f.parent.join("batch"), &f.source, limits(), 100).unwrap();
    assert_eq!(
        reopened.snapshot().unwrap().lookup("alpha", "a").unwrap(),
        set(&["/alpha/a.c"])
    );
}

#[cfg(feature = "fault-injection")]
#[test]
fn interrupted_subprocess_recovers_whole_batch_at_single_commit_point() {
    use rustgrok_path_index::durable::FaultPoint;
    const ROOT: &str = "RUSTGROK_BATCH_TEST_ROOT";
    const STAGE: &str = "RUSTGROK_BATCH_TEST_STAGE";
    if let (Some(root), Some(stage)) = (std::env::var_os(ROOT), std::env::var_os(STAGE)) {
        let root = PathBuf::from(root);
        let fault = match stage.to_str().unwrap() {
            "data" => FaultPoint::ExitAfterGenerationSync,
            "before" => FaultPoint::ExitBeforeRename,
            "after" => FaultPoint::ExitAfterRename,
            _ => panic!("invalid fault stage"),
        };
        let db = DurableBatchIndex::open(
            &root.join("stores/batch"),
            &root.join("source"),
            limits(),
            100,
        )
        .unwrap();
        let _ = db.replace_with_fault(
            Some(1),
            vec![
                project("alpha", &["/alpha/new.c"]),
                project("beta", &["/beta/new.c"]),
            ],
            &CancellationToken::new(),
            fault,
        );
        panic!("fault did not exit subprocess");
    }
    for (stage, expected_generation, term) in [
        ("data", 1, "old"),
        ("before", 1, "old"),
        ("after", 2, "new"),
    ] {
        let f = Fixture::new();
        let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
        db.replace(
            None,
            vec![
                project("alpha", &["/alpha/old.c"]),
                project("beta", &["/beta/old.c"]),
            ],
            &CancellationToken::new(),
        )
        .unwrap();
        drop(db);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("interrupted_subprocess_recovers_whole_batch_at_single_commit_point")
            .env(ROOT, &f.root)
            .env(STAGE, stage)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(91),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let reopened =
            DurableBatchIndex::open(&f.parent.join("batch"), &f.source, limits(), 100).unwrap();
        let view = reopened.snapshot().unwrap();
        assert_eq!(view.generation(), expected_generation);
        for name in ["alpha", "beta"] {
            assert_eq!(
                view.lookup(name, term).unwrap(),
                set(&[&format!("/{name}/{term}.c")])
            );
        }
    }
}

#[cfg(feature = "fault-injection")]
#[test]
fn failed_directory_sync_blocks_batch_handle_until_consistent_reopen() {
    use rustgrok_path_index::durable::{DurableError, FaultPoint};
    let f = Fixture::new();
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
    db.replace(
        None,
        vec![
            project("alpha", &["/alpha/old.c"]),
            project("beta", &["/beta/old.c"]),
        ],
        &CancellationToken::new(),
    )
    .unwrap();
    let held = db.snapshot().unwrap();
    assert!(matches!(
        db.replace_with_fault(
            Some(1),
            vec![
                project("alpha", &["/alpha/new.c"]),
                project("beta", &["/beta/new.c"])
            ],
            &CancellationToken::new(),
            FaultPoint::FailDirectorySync
        ),
        Err(DurableError::PublicationIndeterminate)
    ));
    assert!(matches!(
        db.snapshot(),
        Err(DurableError::PublicationIndeterminate)
    ));
    assert!(matches!(
        db.replace(
            Some(1),
            vec![project("alpha", &[]), project("beta", &[])],
            &CancellationToken::new()
        ),
        Err(DurableError::PublicationIndeterminate)
    ));
    for name in ["alpha", "beta"] {
        assert_eq!(
            held.lookup(name, "old").unwrap(),
            set(&[&format!("/{name}/old.c")])
        );
    }
    drop(db);
    let reopened =
        DurableBatchIndex::open(&f.parent.join("batch"), &f.source, limits(), 100).unwrap();
    let view = reopened.snapshot().unwrap();
    assert_eq!(view.generation(), 2);
    for name in ["alpha", "beta"] {
        assert_eq!(
            view.lookup(name, "new").unwrap(),
            set(&[&format!("/{name}/new.c")])
        );
    }
}

#[cfg(feature = "fault-injection")]
#[test]
fn precommit_storage_faults_do_not_publish_any_project() {
    use rustgrok_path_index::durable::FaultPoint;
    for fault in [
        FaultPoint::NoSpaceGenerationWrite,
        FaultPoint::NoSpacePointerWrite,
        FaultPoint::CorruptGenerationBeforeReadback,
        FaultPoint::CorruptPointerBeforeReadback,
    ] {
        let f = Fixture::new();
        let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
        db.replace(
            None,
            vec![
                project("alpha", &["/alpha/old.c"]),
                project("beta", &["/beta/old.c"]),
            ],
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(db
            .replace_with_fault(
                Some(1),
                vec![
                    project("alpha", &["/alpha/new.c"]),
                    project("beta", &["/beta/new.c"])
                ],
                &CancellationToken::new(),
                fault
            )
            .is_err());
        let view = db.snapshot().unwrap();
        assert_eq!(view.generation(), 1);
        for name in ["alpha", "beta"] {
            assert_eq!(
                view.lookup(name, "old").unwrap(),
                set(&[&format!("/{name}/old.c")])
            );
        }
        drop(view);
        drop(db);
        let reopened =
            DurableBatchIndex::open(&f.parent.join("batch"), &f.source, limits(), 100).unwrap();
        assert_eq!(reopened.snapshot().unwrap().generation(), 1);
    }
}

#[test]
fn limits_source_identity_and_exclusive_store_lock_fail_closed() {
    use rustgrok_path_index::durable::DurableError;
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let mut bounded = limits();
    bounded.max_batch_documents = 2;
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, bounded, 100).unwrap();
    db.replace(
        None,
        vec![
            project("alpha", &["/alpha/a.c"]),
            project("beta", &["/beta/b.c"]),
        ],
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(matches!(
        DurableBatchIndex::open(&f.parent.join("batch"), &f.source, bounded, 100),
        Err(DurableError::Busy)
    ));
    assert!(db
        .replace(
            Some(1),
            vec![
                project("alpha", &["/alpha/a.c", "/alpha/x.c"]),
                project("beta", &["/beta/b.c"])
            ],
            &CancellationToken::new()
        )
        .is_err());
    assert_eq!(db.snapshot().unwrap().generation(), 1);
    fs::rename(&f.source, f.root.join("original-source")).unwrap();
    fs::create_dir(&f.source).unwrap();
    assert!(matches!(db.snapshot(), Err(DurableError::SourceChanged)));
    assert!(matches!(
        db.replace(
            Some(1),
            vec![project("alpha", &[]), project("beta", &[])],
            &CancellationToken::new()
        ),
        Err(DurableError::SourceChanged)
    ));
    drop(db);
    let symlink_source = f.root.join("source-link");
    symlink(&f.source, &symlink_source).unwrap();
    assert!(
        DurableBatchIndex::open(&f.parent.join("batch"), &symlink_source, bounded, 100).is_err()
    );
    assert!(DurableBatchIndex::create(&f.source, "bad-store", &f.source, bounded, 100).is_err());
    assert!(!f.source.join("bad-store").exists());
}

#[cfg(feature = "fault-injection")]
#[test]
fn snapshot_started_before_sync_failure_does_not_escape_indeterminate_state() {
    use rustgrok_path_index::durable::{DurableError, FaultPoint};
    let f = Fixture::new();
    let db =
        Arc::new(DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap());
    db.replace(
        None,
        vec![
            project("alpha", &["/alpha/old.c"]),
            project("beta", &["/beta/old.c"]),
        ],
        &CancellationToken::new(),
    )
    .unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let reader = Arc::clone(&db);
    let join = thread::spawn(move || {
        reader.snapshot_with_observer(&mut || {
            ready_tx.send(()).unwrap();
            resume_rx.recv().unwrap();
        })
    });
    ready_rx.recv().unwrap();
    assert!(matches!(
        db.replace_with_fault(
            Some(1),
            vec![
                project("alpha", &["/alpha/new.c"]),
                project("beta", &["/beta/new.c"])
            ],
            &CancellationToken::new(),
            FaultPoint::FailDirectorySync
        ),
        Err(DurableError::PublicationIndeterminate)
    ));
    resume_tx.send(()).unwrap();
    assert!(matches!(
        join.join().unwrap(),
        Err(DurableError::PublicationIndeterminate)
    ));
}

#[test]
fn batch_lock_is_exclusive_across_processes() {
    use rustgrok_path_index::durable::DurableError;
    const ROOT: &str = "RUSTGROK_BATCH_LOCK_ROOT";
    if let Some(root) = std::env::var_os(ROOT) {
        let root = PathBuf::from(root);
        assert!(matches!(
            DurableBatchIndex::open(
                &root.join("stores/batch"),
                &root.join("source"),
                limits(),
                100
            ),
            Err(DurableError::Busy)
        ));
        return;
    }
    let f = Fixture::new();
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("batch_lock_is_exclusive_across_processes")
        .env(ROOT, &f.root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    drop(db);
    assert!(DurableBatchIndex::open(&f.parent.join("batch"), &f.source, limits(), 100).is_ok());
}

#[test]
fn empty_project_at_maximum_name_length_fits_batch_metadata_budget() {
    let f = Fixture::new();
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
    let name = "a".repeat(limits().per_project.max_path_bytes);
    assert_eq!(
        db.replace(None, vec![project(&name, &[])], &CancellationToken::new())
            .unwrap(),
        1
    );
    assert_eq!(db.snapshot().unwrap().projects(), vec![name]);
}

#[test]
fn source_batch_rejects_aggregate_limit_without_publication() {
    use rustgrok_path_index::durable::DurableError;
    let f = Fixture::new();
    fs::create_dir(f.source.join("alpha")).unwrap();
    fs::create_dir(f.source.join("beta")).unwrap();
    fs::write(f.source.join("alpha/a.c"), b"alpha\n").unwrap();
    fs::write(f.source.join("beta/b.py"), b"beta\n").unwrap();
    let mut bounded = limits();
    bounded.max_batch_documents = 1;
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, bounded, 100).unwrap();
    assert!(matches!(
        db.replace_from_source(
            None,
            &["alpha".into(), "beta".into()],
            &CancellationToken::new()
        ),
        Err(DurableError::LimitExceeded)
    ));
    assert!(db.snapshot().is_err());
    assert_eq!(fs::read(f.source.join("alpha/a.c")).unwrap(), b"alpha\n");
    assert_eq!(fs::read(f.source.join("beta/b.py")).unwrap(), b"beta\n");
}

#[test]
fn source_batch_rejects_project_symlink_and_duplicate_names() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    fs::create_dir(f.source.join("alpha")).unwrap();
    fs::write(f.source.join("alpha/a.c"), b"a\n").unwrap();
    symlink("alpha", f.source.join("beta")).unwrap();
    let db = DurableBatchIndex::create(&f.parent, "batch", &f.source, limits(), 100).unwrap();
    for names in [
        vec!["alpha".into(), "beta".into()],
        vec!["alpha".into(), "alpha".into()],
        vec!["../alpha".into()],
    ] {
        assert!(db
            .replace_from_source(None, &names, &CancellationToken::new())
            .is_err());
        assert!(db.snapshot().is_err());
    }
}
