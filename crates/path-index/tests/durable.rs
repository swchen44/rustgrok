// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
#![cfg(target_os = "macos")]
use rustgrok_path_index::durable::{DurableError, DurablePathIndex};
use rustgrok_path_index::{CancellationToken, IndexLimits};
use std::{
    collections::HashSet,
    fs, io,
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
fn limits() -> IndexLimits {
    IndexLimits {
        max_documents: 10,
        max_path_bytes: 256,
        max_total_path_bytes: 1024,
        max_live_generations: 3,
    }
}
fn items(paths: &[&str]) -> Vec<io::Result<String>> {
    paths.iter().map(|p| Ok((*p).to_string())).collect()
}
fn set(paths: &[&str]) -> HashSet<String> {
    paths.iter().map(|p| (*p).to_string()).collect()
}

#[test]
fn durable_add_remove_reopen_and_retained_reader() {
    let f = Fixture::new();
    fs::write(f.source.join("main.c"), b"int main(void){return 0;}\n").unwrap();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    assert_eq!(
        db.replace(
            None,
            1,
            items(&["/alpha/main.c"]),
            &CancellationToken::new()
        )
        .unwrap(),
        1
    );
    let old = db.snapshot().unwrap();
    fs::write(f.source.join("main.js"), b"export function main() {}\n").unwrap();
    assert_eq!(
        db.replace(
            Some(1),
            2,
            items(&["/alpha/main.c", "/alpha/main.js"]),
            &CancellationToken::new()
        )
        .unwrap(),
        2
    );
    assert_eq!(old.manifest(), set(&["/alpha/main.c"]));
    fs::remove_file(f.source.join("main.js")).unwrap();
    assert_eq!(
        db.replace(
            Some(2),
            1,
            items(&["/alpha/main.c"]),
            &CancellationToken::new()
        )
        .unwrap(),
        3
    );
    assert_eq!(
        fs::read(f.source.join("main.c")).unwrap(),
        b"int main(void){return 0;}\n"
    );
    drop(db);
    let reopened = DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).unwrap();
    let view = reopened.snapshot().unwrap();
    assert_eq!(view.generation(), 3);
    assert_eq!(view.manifest(), set(&["/alpha/main.c"]));
    assert_eq!(view.lookup("main").unwrap(), set(&["/alpha/main.c"]));
}

#[test]
fn failed_input_and_cancel_preserve_previous_durable_generation() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    let broken = vec![
        Ok("/alpha/new.c".to_owned()),
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "corrupt inventory",
        )),
    ];
    assert!(db
        .replace(Some(1), 2, broken, &CancellationToken::new())
        .is_err());
    let cancelled = CancellationToken::new();
    assert!(cancelled.cancel());
    assert!(db
        .replace(Some(1), 1, items(&["/alpha/new.c"]), &cancelled)
        .is_err());
    assert!(db
        .replace(
            Some(1),
            1,
            items(&["/alpha/é.c"]),
            &CancellationToken::new()
        )
        .is_err());
    assert_eq!(db.snapshot().unwrap().generation(), 1);
    drop(db);
    let reopened = DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).unwrap();
    assert_eq!(
        reopened.snapshot().unwrap().manifest(),
        set(&["/alpha/main.c"])
    );
}

#[test]
fn overlap_and_invalid_store_names_fail_without_touching_source() {
    let f = Fixture::new();
    let source_file = f.source.join("keep.txt");
    fs::write(&source_file, b"keep").unwrap();
    assert!(DurablePathIndex::create(&f.source, "idx", &f.source, "alpha", limits(), 64).is_err());
    for invalid in ["..", "../source", "a/b", "", "."] {
        assert!(
            DurablePathIndex::create(&f.parent, invalid, &f.source, "alpha", limits(), 64).is_err()
        );
    }
    assert_eq!(fs::read(source_file).unwrap(), b"keep");
}

#[test]
fn second_handle_is_rejected_while_first_holds_store_lock() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    assert!(matches!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64),
        Err(DurableError::Busy)
    ));
    drop(db);
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_ok());
}

#[test]
fn an_empty_store_cannot_be_reopened_for_a_different_project_or_source_tree() {
    let f = Fixture::new();
    let other = f.root.join("different-source");
    fs::create_dir(&other).unwrap();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    drop(db);
    assert!(DurablePathIndex::open(&f.store(), &f.source, "beta", limits(), 64).is_err());
    assert!(DurablePathIndex::open(&f.store(), &other, "alpha", limits(), 64).is_err());
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_ok());
}

#[test]
fn corrupted_or_truncated_generation_is_never_loaded() {
    use std::io::{Seek, SeekFrom, Write};
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    drop(db);
    let generation = f.store().join("G-1");
    let original = fs::read(&generation).unwrap();
    for corrupt in [
        original[..original.len() - 1].to_vec(),
        {
            let mut b = original.clone();
            b[20] ^= 1;
            b
        },
        {
            let mut b = original.clone();
            b.push(0);
            b
        },
    ] {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&generation)
            .unwrap();
        file.write_all(&corrupt).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        drop(file);
        assert!(matches!(
            DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64),
            Err(DurableError::Corrupt)
        ));
    }
    fs::write(generation, original).unwrap();
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_ok());
}

#[test]
fn symlinked_paths_and_current_pointer_are_rejected_without_touching_source() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let target = f.source.join("keep.txt");
    fs::write(&target, b"keep").unwrap();
    let link = f.root.join("link-to-indexes");
    symlink(&f.parent, &link).unwrap();
    assert!(DurablePathIndex::create(&link, "bad", &f.source, "alpha", limits(), 64).is_err());
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    drop(db);
    fs::remove_file(f.store().join("CURRENT")).unwrap();
    symlink(&target, f.store().join("CURRENT")).unwrap();
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_err());
    assert_eq!(fs::read(target).unwrap(), b"keep");
}

#[test]
fn other_process_cannot_open_while_lock_is_held() {
    const FLAG: &str = "RUSTGROK_DURABLE_LOCK_CHILD";
    if let Some(root) = std::env::var_os(FLAG) {
        let root = PathBuf::from(root);
        assert!(matches!(
            DurablePathIndex::open(
                &root.join("indexes/alpha-store"),
                &root.join("source"),
                "alpha",
                limits(),
                64
            ),
            Err(DurableError::Busy)
        ));
        return;
    }
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("other_process_cannot_open_while_lock_is_held")
        .env(FLAG, &f.root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
    drop(db);
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_ok());
}

#[cfg(feature = "fault-injection")]
#[test]
fn interrupted_subprocess_recovers_only_published_complete_generation() {
    use rustgrok_path_index::durable::FaultPoint;
    const ROOT: &str = "RUSTGROK_DURABLE_CRASH_ROOT";
    const STAGE: &str = "RUSTGROK_DURABLE_CRASH_STAGE";
    if let Some(root) = std::env::var_os(ROOT) {
        let root = PathBuf::from(root);
        let stage = match std::env::var(STAGE).unwrap().as_str() {
            "data" => FaultPoint::ExitAfterGenerationSync,
            "before" => FaultPoint::ExitBeforeRename,
            "after" => FaultPoint::ExitAfterRename,
            _ => panic!("invalid test stage"),
        };
        let db = DurablePathIndex::open(
            &root.join("indexes/alpha-store"),
            &root.join("source"),
            "alpha",
            limits(),
            64,
        )
        .unwrap();
        let _ = db.replace_with_fault(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new(),
            stage,
        );
        panic!("fault did not terminate child");
    }
    for (stage, expected_generation, expected_path) in [
        ("data", 1, "/alpha/main.c"),
        ("before", 1, "/alpha/main.c"),
        ("after", 2, "/alpha/new.c"),
    ] {
        let f = Fixture::new();
        let db =
            DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
                .unwrap();
        db.replace(
            None,
            1,
            items(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
        drop(db);
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("interrupted_subprocess_recovers_only_published_complete_generation")
            .env(ROOT, &f.root)
            .env(STAGE, stage)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(91),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let reopened =
            DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).unwrap();
        assert_eq!(
            reopened.snapshot().unwrap().generation(),
            expected_generation
        );
        assert_eq!(
            reopened.snapshot().unwrap().manifest(),
            set(&[expected_path])
        );
        assert_eq!(
            reopened
                .replace(
                    Some(expected_generation),
                    1,
                    items(&["/alpha/recovered.c"]),
                    &CancellationToken::new()
                )
                .unwrap(),
            3,
            "an orphaned generation must not require deleting previous files"
        );
    }
}

#[cfg(feature = "fault-injection")]
#[test]
fn failure_after_pointer_rename_is_explicitly_indeterminate_and_blocks_handle() {
    use rustgrok_path_index::durable::FaultPoint;
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    let old = db.snapshot().unwrap();
    assert!(matches!(
        db.replace_with_fault(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
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
            1,
            items(&["/alpha/other.c"]),
            &CancellationToken::new()
        ),
        Err(DurableError::PublicationIndeterminate)
    ));
    assert_eq!(old.manifest(), set(&["/alpha/main.c"]));
    drop(db);
    let reopened = DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).unwrap();
    assert_eq!(
        reopened.snapshot().unwrap().manifest(),
        set(&["/alpha/new.c"])
    );
}

#[test]
fn self_checksummed_but_partial_or_duplicate_manifest_is_rejected() {
    use sha2::{Digest, Sha256};
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    drop(db);
    let generation = f.store().join("G-1");
    let current = f.store().join("CURRENT");
    let original = fs::read(&generation).unwrap();
    let original_pointer = fs::read(&current).unwrap();
    let count_offset = 8 + 8 + 8 + "alpha".len();
    for duplicate in [false, true] {
        let mut payload = original[..original.len() - 32].to_vec();
        payload[count_offset..count_offset + 8].copy_from_slice(&2u64.to_le_bytes());
        if duplicate {
            let path = b"/alpha/main.c";
            payload.extend_from_slice(&(path.len() as u64).to_le_bytes());
            payload.extend_from_slice(path);
        }
        let digest = Sha256::digest(&payload);
        payload.extend_from_slice(&digest);
        let mut pointer = original_pointer.clone();
        pointer[16..48].copy_from_slice(&digest);
        let pointer_checksum = Sha256::digest(&pointer[..48]);
        pointer[48..80].copy_from_slice(&pointer_checksum);
        fs::write(&generation, payload).unwrap();
        fs::write(&current, pointer).unwrap();
        assert!(matches!(
            DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64),
            Err(DurableError::Corrupt)
        ));
    }
    fs::write(&generation, original).unwrap();
    fs::write(current, original_pointer).unwrap();
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_ok());
}

#[test]
fn a_fifo_current_is_rejected_without_blocking() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        time::{Duration, Instant},
    };
    const FLAG: &str = "RUSTGROK_DURABLE_FIFO_CHILD";
    if let Some(root) = std::env::var_os(FLAG) {
        let root = PathBuf::from(root);
        assert!(DurablePathIndex::open(
            &root.join("indexes/alpha-store"),
            &root.join("source"),
            "alpha",
            limits(),
            64
        )
        .is_err());
        return;
    }
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    drop(db);
    fs::remove_file(f.store().join("CURRENT")).unwrap();
    let fifo = CString::new(f.store().join("CURRENT").as_os_str().as_bytes()).unwrap();
    // SAFETY: fifo is a valid NUL-terminated path inside the owned fixture.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("a_fifo_current_is_rejected_without_blocking")
        .env(FLAG, &f.root)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.is_some_and(|status| status.success()),
        "opening FIFO blocked or was not rejected"
    );
}

#[cfg(feature = "fault-injection")]
#[test]
fn corrupt_prepublication_files_fail_readback_and_preserve_old_generation() {
    use rustgrok_path_index::durable::FaultPoint;
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    for point in [
        FaultPoint::CorruptGenerationBeforeReadback,
        FaultPoint::CorruptPointerBeforeReadback,
    ] {
        assert!(matches!(
            db.replace_with_fault(
                Some(1),
                1,
                items(&["/alpha/new.c"]),
                &CancellationToken::new(),
                point
            ),
            Err(DurableError::Corrupt)
        ));
        assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
    }
    drop(db);
    let reopened = DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).unwrap();
    assert_eq!(
        reopened.snapshot().unwrap().manifest(),
        set(&["/alpha/main.c"])
    );
}

#[test]
fn symlink_at_next_generation_is_rejected_not_silently_skipped() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let target = f.source.join("keep.txt");
    fs::write(&target, b"keep").unwrap();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    symlink(&target, f.store().join("G-2")).unwrap();
    assert!(db
        .replace(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new()
        )
        .is_err());
    assert_eq!(db.snapshot().unwrap().generation(), 1);
    assert_eq!(fs::read(target).unwrap(), b"keep");
}

#[cfg(feature = "fault-injection")]
#[test]
fn readers_can_query_old_generation_while_post_rename_sync_is_paused() {
    use std::{
        sync::{mpsc, Arc},
        thread,
        time::Duration,
    };
    let f = Fixture::new();
    let db = Arc::new(
        DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
            .unwrap(),
    );
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    let (paused_tx, paused_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let writer_db = Arc::clone(&db);
    let writer = thread::spawn(move || {
        let mut observer = || {
            paused_tx.send(()).unwrap();
            resume_rx.recv().unwrap();
        };
        writer_db.replace_with_observer(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new(),
            &mut observer,
        )
    });
    paused_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let (read_tx, read_rx) = mpsc::channel();
    let reader_db = Arc::clone(&db);
    let reader = thread::spawn(move || {
        read_tx
            .send(reader_db.snapshot().map(|s| s.manifest()))
            .unwrap();
    });
    let observed = read_rx.recv_timeout(Duration::from_secs(2));
    resume_tx.send(()).unwrap();
    assert_eq!(writer.join().unwrap().unwrap(), 2);
    reader.join().unwrap();
    assert_eq!(
        observed
            .expect("reader blocked during directory sync")
            .unwrap(),
        set(&["/alpha/main.c"])
    );
    assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/new.c"]));
}

#[cfg(feature = "fault-injection")]
#[test]
fn observer_panic_after_rename_is_indeterminate_not_silently_old() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    let old = db.snapshot().unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        db.replace_with_observer(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new(),
            &mut || panic!("injected observer failure"),
        )
    }));
    assert!(result.is_err());
    assert!(matches!(
        db.snapshot(),
        Err(DurableError::PublicationIndeterminate)
    ));
    assert_eq!(old.manifest(), set(&["/alpha/main.c"]));
    drop(db);
    assert_eq!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64)
            .unwrap()
            .snapshot()
            .unwrap()
            .manifest(),
        set(&["/alpha/new.c"])
    );
}

#[test]
fn live_writer_refuses_a_replaced_current_symlink() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let target = f.source.join("keep.txt");
    fs::write(&target, b"keep").unwrap();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    fs::remove_file(f.store().join("CURRENT")).unwrap();
    symlink(&target, f.store().join("CURRENT")).unwrap();
    assert!(db
        .replace(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new()
        )
        .is_err());
    assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
    assert_eq!(fs::read(&target).unwrap(), b"keep");
    assert!(fs::symlink_metadata(f.store().join("CURRENT"))
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn exhausted_generation_id_budget_fails_without_replacing_current() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 1)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(matches!(
        db.replace(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new()
        ),
        Err(DurableError::LimitExceeded)
    ));
    assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
}

#[test]
fn iterator_panic_keeps_published_reader_and_reopen_recovers() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    let source =
        std::iter::from_fn(|| -> Option<io::Result<String>> { panic!("injected source failure") });
    assert!(
        std::panic::catch_unwind(|| db.replace(Some(1), 1, source, &CancellationToken::new()))
            .is_err()
    );
    assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
    assert!(db
        .replace(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new()
        )
        .is_err());
    drop(db);
    assert_eq!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64)
            .unwrap()
            .snapshot()
            .unwrap()
            .manifest(),
        set(&["/alpha/main.c"])
    );
}

#[test]
fn retained_old_reader_enforces_the_same_live_generation_limit() {
    let f = Fixture::new();
    let mut budget = limits();
    budget.max_live_generations = 2;
    let db =
        DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", budget, 64).unwrap();
    db.replace(None, 1, items(&["/alpha/old.c"]), &CancellationToken::new())
        .unwrap();
    let old = db.snapshot().unwrap();
    db.replace(
        Some(1),
        1,
        items(&["/alpha/current.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(matches!(
        db.replace(
            Some(2),
            1,
            items(&["/alpha/next.c"]),
            &CancellationToken::new()
        ),
        Err(DurableError::LimitExceeded)
    ));
    assert_eq!(old.manifest(), set(&["/alpha/old.c"]));
    assert_eq!(
        db.snapshot().unwrap().manifest(),
        set(&["/alpha/current.c"])
    );
    drop(old);
    assert_eq!(
        db.replace(
            Some(2),
            1,
            items(&["/alpha/next.c"]),
            &CancellationToken::new()
        )
        .unwrap(),
        3
    );
}

#[cfg(feature = "fault-injection")]
#[test]
fn simulated_disk_full_before_rename_keeps_prior_generation() {
    use rustgrok_path_index::durable::FaultPoint;
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    for point in [
        FaultPoint::NoSpaceGenerationWrite,
        FaultPoint::NoSpacePointerWrite,
    ] {
        match db.replace_with_fault(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new(),
            point,
        ) {
            Err(DurableError::Io(error)) => assert_eq!(error.raw_os_error(), Some(libc::ENOSPC)),
            other => panic!("expected ENOSPC, got {other:?}"),
        }
        assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
    }
    drop(db);
    assert_eq!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64)
            .unwrap()
            .snapshot()
            .unwrap()
            .manifest(),
        set(&["/alpha/main.c"])
    );
}

#[test]
fn cancellation_during_manifest_iteration_leaves_old_generation() {
    use std::sync::{mpsc, Arc};
    let f = Fixture::new();
    let db = Arc::new(
        DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
            .unwrap(),
    );
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    let token = Arc::new(CancellationToken::new());
    let worker_token = Arc::clone(&token);
    let worker_db = Arc::clone(&db);
    let (started_tx, started_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut first = true;
        let source = std::iter::from_fn(move || {
            if !first {
                return None;
            }
            first = false;
            started_tx.send(()).unwrap();
            resume_rx.recv().unwrap();
            Some(Ok("/alpha/new.c".to_owned()))
        });
        worker_db.replace(Some(1), 1, source, &worker_token)
    });
    started_rx.recv().unwrap();
    assert!(token.cancel());
    resume_tx.send(()).unwrap();
    assert!(matches!(
        worker.join().unwrap(),
        Err(DurableError::Index(
            rustgrok_path_index::IndexError::Cancelled
        ))
    ));
    assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
    drop(db);
    assert_eq!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64)
            .unwrap()
            .snapshot()
            .unwrap()
            .manifest(),
        set(&["/alpha/main.c"])
    );
}

#[cfg(feature = "fault-injection")]
#[test]
fn cancellation_at_last_precommit_checkpoint_cannot_publish() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    let token = CancellationToken::new();
    let mut observer = || assert!(token.cancel());
    assert!(matches!(
        db.replace_with_precommit_observer(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &token,
            &mut observer
        ),
        Err(DurableError::Index(
            rustgrok_path_index::IndexError::Cancelled
        ))
    ));
    assert_eq!(db.snapshot().unwrap().manifest(), set(&["/alpha/main.c"]));
    drop(db);
    assert_eq!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64)
            .unwrap()
            .snapshot()
            .unwrap()
            .manifest(),
        set(&["/alpha/main.c"])
    );
}

#[test]
fn symlinked_generation_and_hardlinked_pointer_are_rejected() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let target = f.source.join("keep.txt");
    fs::write(&target, b"keep").unwrap();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    drop(db);
    let generation = f.store().join("G-1");
    let original = fs::read(&generation).unwrap();
    fs::remove_file(&generation).unwrap();
    symlink(&target, &generation).unwrap();
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_err());
    fs::remove_file(&generation).unwrap();
    fs::write(&generation, original).unwrap();
    let current = f.store().join("CURRENT");
    let alias = f.source.join("pointer-hardlink");
    fs::hard_link(&current, &alias).unwrap();
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_err());
    assert_eq!(fs::read(target).unwrap(), b"keep");
}

#[test]
fn missing_current_pointer_is_corruption_even_after_a_published_generation() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    drop(db);
    fs::remove_file(f.store().join("CURRENT")).unwrap();
    assert!(matches!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64),
        Err(DurableError::Corrupt)
    ));
}

#[test]
fn a_corrupted_pointer_cannot_masquerade_as_the_empty_sentinel() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    drop(db);
    let path = f.store().join("CURRENT");
    let mut pointer = fs::read(&path).unwrap();
    pointer[8..48].fill(0);
    fs::write(path, pointer).unwrap();
    assert!(matches!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64),
        Err(DurableError::Corrupt)
    ));
}

#[cfg(feature = "fault-injection")]
#[test]
fn writer_paused_before_lock_fails_closed_after_another_writer_is_indeterminate() {
    use rustgrok_path_index::durable::FaultPoint;
    use std::{
        sync::{mpsc, Arc},
        thread,
        time::Duration,
    };
    let f = Fixture::new();
    let db = Arc::new(
        DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
            .unwrap(),
    );
    db.replace(
        None,
        1,
        items(&["/alpha/main.c"]),
        &CancellationToken::new(),
    )
    .unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let late_db = Arc::clone(&db);
    let late = thread::spawn(move || {
        let mut observe = || {
            ready_tx.send(()).unwrap();
            resume_rx.recv().unwrap();
        };
        late_db.replace_with_prelock_observer(
            Some(1),
            1,
            items(&["/alpha/late.c"]),
            &CancellationToken::new(),
            &mut observe,
        )
    });
    ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(matches!(
        db.replace_with_fault(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new(),
            FaultPoint::FailDirectorySync
        ),
        Err(DurableError::PublicationIndeterminate)
    ));
    let files = || {
        fs::read_dir(f.store())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<HashSet<_>>()
    };
    let before = files();
    resume_tx.send(()).unwrap();
    let result = late.join().unwrap();
    assert_eq!(
        before,
        files(),
        "late writer created files after indeterminate publication"
    );
    assert!(matches!(
        result,
        Err(DurableError::PublicationIndeterminate)
    ));
}

#[test]
fn replacing_source_directory_rejects_snapshots_and_writes_without_touching_either_tree() {
    let f = Fixture::new();
    fs::write(f.source.join("old.c"), b"old source").unwrap();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(None, 1, items(&["/alpha/old.c"]), &CancellationToken::new())
        .unwrap();
    let old = db.snapshot().unwrap();
    let moved = f.root.join("old-source");
    fs::rename(&f.source, &moved).unwrap();
    fs::create_dir(&f.source).unwrap();
    fs::write(f.source.join("new.c"), b"new source").unwrap();
    let files = || {
        fs::read_dir(f.store())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<HashSet<_>>()
    };
    let before = files();
    assert!(db.snapshot().is_err());
    assert!(db
        .replace(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new()
        )
        .is_err());
    assert_eq!(before, files());
    assert_eq!(old.manifest(), set(&["/alpha/old.c"]));
    assert_eq!(fs::read(moved.join("old.c")).unwrap(), b"old source");
    assert_eq!(fs::read(f.source.join("new.c")).unwrap(), b"new source");
    drop(db);
    assert!(DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64).is_err());
    fs::remove_dir_all(&f.source).unwrap();
    fs::rename(&moved, &f.source).unwrap();
    assert_eq!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64)
            .unwrap()
            .snapshot()
            .unwrap()
            .manifest(),
        set(&["/alpha/old.c"])
    );
}

#[test]
fn source_replaced_during_manifest_collection_is_rejected_before_generation_files() {
    let f = Fixture::new();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(None, 1, items(&["/alpha/old.c"]), &CancellationToken::new())
        .unwrap();
    let before = fs::read_dir(f.store())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<HashSet<_>>();
    let changed = std::iter::once_with(|| {
        fs::rename(&f.source, f.root.join("old-source")).unwrap();
        fs::create_dir(&f.source).unwrap();
        Ok("/alpha/new.c".to_string())
    });
    assert!(matches!(
        db.replace(Some(1), 1, changed, &CancellationToken::new()),
        Err(DurableError::SourceChanged)
    ));
    assert_eq!(
        before,
        fs::read_dir(f.store())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect()
    );
}

#[cfg(feature = "fault-injection")]
#[test]
fn source_replaced_immediately_before_publication_cannot_switch_pointer() {
    let f = Fixture::new();
    fs::write(f.source.join("old.c"), b"old source").unwrap();
    let db = DurablePathIndex::create(&f.parent, "alpha-store", &f.source, "alpha", limits(), 64)
        .unwrap();
    db.replace(None, 1, items(&["/alpha/old.c"]), &CancellationToken::new())
        .unwrap();
    let pointer = fs::read(f.store().join("CURRENT")).unwrap();
    let mut replace_source = || {
        fs::rename(&f.source, f.root.join("old-source")).unwrap();
        fs::create_dir(&f.source).unwrap();
    };
    assert!(matches!(
        db.replace_with_precommit_observer(
            Some(1),
            1,
            items(&["/alpha/new.c"]),
            &CancellationToken::new(),
            &mut replace_source
        ),
        Err(DurableError::SourceChanged)
    ));
    assert_eq!(fs::read(f.store().join("CURRENT")).unwrap(), pointer);
    assert_eq!(
        fs::read(f.root.join("old-source/old.c")).unwrap(),
        b"old source"
    );
    drop(db);
    fs::remove_dir(&f.source).unwrap();
    fs::rename(f.root.join("old-source"), &f.source).unwrap();
    assert_eq!(
        DurablePathIndex::open(&f.store(), &f.source, "alpha", limits(), 64)
            .unwrap()
            .snapshot()
            .unwrap()
            .manifest(),
        set(&["/alpha/old.c"])
    );
}
