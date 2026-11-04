// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
#![cfg(all(target_os = "macos", feature = "fault-injection"))]
use rustgrok_path_index::durable::full::{DurableFullIndex, FullLimits, FullStoreError};
use rustgrok_path_index::durable::DurableError;
use rustgrok_path_index::CancellationToken;
use std::{
    fs,
    path::PathBuf,
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
        let tmp = fs::canonicalize(std::env::temp_dir()).unwrap();
        let root = tmp.join(format!(
            "rustgrok-durable-{}-{}",
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
    fn store(&self) -> PathBuf {
        self.parent.join("alpha-store")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("owned fixture cleanup");
    }
}

fn publish(f: &Fixture) -> DurableFullIndex {
    fs::create_dir(f.source.join("alpha")).unwrap();
    fs::write(f.source.join("alpha/a.c"), b"alpha beta").unwrap();
    let db = DurableFullIndex::create(
        &f.parent,
        "alpha-store",
        &f.source,
        "utf8-default-tabs",
        FullLimits::default(),
    )
    .unwrap();
    db.replace_from_source(None, &["alpha".into()], &CancellationToken::new())
        .unwrap();
    db
}

#[cfg(feature = "fault-injection")]
#[test]
fn old_snapshot_is_available_after_rename_before_directory_sync_completes() {
    use std::{sync::mpsc, thread, time::Duration};
    let f = Fixture::new();
    let db = publish(&f);
    fs::write(f.source.join("alpha/a.c"), b"new gamma").unwrap();
    let (paused_tx, paused_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (read_tx, read_rx) = mpsc::channel();
    thread::scope(|scope| {
        let writer_db = &db;
        let writer = scope.spawn(move || {
            writer_db.replace_with_publication_observer(
                Some(1),
                &["alpha".into()],
                &CancellationToken::new(),
                &mut || {
                    paused_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                },
            )
        });
        paused_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        scope.spawn(|| {
            let snapshot = db.snapshot().unwrap();
            read_tx
                .send((
                    snapshot.generation(),
                    snapshot.lookup("alpha", "beta").unwrap().len(),
                ))
                .unwrap();
        });
        let old = read_rx.recv_timeout(Duration::from_secs(1));
        release_tx.send(()).unwrap();
        assert_eq!(writer.join().unwrap().unwrap(), 2);
        assert_eq!(
            old,
            Ok((1, 1)),
            "old immutable generation must not wait on directory fsync"
        );
    });
    assert_eq!(db.snapshot().unwrap().generation(), 2);
}

#[test]
fn panic_after_rename_blocks_new_snapshots_until_reopen() {
    let f = Fixture::new();
    let db = publish(&f);
    fs::write(f.source.join("alpha/a.c"), b"new gamma").unwrap();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = db.replace_with_publication_observer(
            Some(1),
            &["alpha".into()],
            &CancellationToken::new(),
            &mut || panic!("post-rename observer panic"),
        );
    }));
    assert!(panic.is_err());
    assert!(matches!(
        db.snapshot(),
        Err(FullStoreError::Storage(
            DurableError::PublicationIndeterminate
        ))
    ));
    drop(db);
    let db = DurableFullIndex::open(&f.store(), &f.source, FullLimits::default()).unwrap();
    assert_eq!(db.snapshot().unwrap().generation(), 2);
    assert_eq!(
        db.snapshot().unwrap().lookup("alpha", "new").unwrap().len(),
        1
    );
}
