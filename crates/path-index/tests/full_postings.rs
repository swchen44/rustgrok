// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
#![cfg(target_os = "macos")]
use rustgrok_path_index::durable::full::{DurableFullIndex, FullLimits, FullStoreError};
use rustgrok_path_index::durable::DurableError;
use rustgrok_path_index::CancellationToken;
use sha2::{Digest, Sha256};
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
fn u64at(b: &[u8], i: &mut usize) -> usize {
    let n = u64::from_le_bytes(b[*i..*i + 8].try_into().unwrap()) as usize;
    *i += 8;
    n
}
fn skipstr(b: &[u8], i: &mut usize) {
    let n = u64at(b, i);
    *i += n;
}
#[test]
fn checksum_valid_generation_missing_a_term_is_rejected() {
    let f = Fixture::new();
    drop(publish(&f));
    let file = f.store().join("F-1");
    let mut bytes = fs::read(&file).unwrap();
    // Reframe a structurally valid one-term record, dropping beta and fixing SHA.
    let mut at = 16;
    let count = u64at(&bytes, &mut at);
    for _ in 0..count {
        skipstr(&bytes, &mut at);
    }
    let count = u64at(&bytes, &mut at);
    for _ in 0..count {
        skipstr(&bytes, &mut at);
        at += 8 + 32 + 8 + 32;
    }
    let terms = at;
    assert_eq!(u64at(&bytes, &mut at), 2);
    skipstr(&bytes, &mut at);
    let n = u64at(&bytes, &mut at);
    for _ in 0..n {
        at += 8;
        let n = u64at(&bytes, &mut at);
        at += n * 40;
    }
    bytes.truncate(at);
    bytes[terms..terms + 8].copy_from_slice(&1u64.to_le_bytes());
    let digest = Sha256::digest(&bytes);
    bytes.extend_from_slice(&digest);
    fs::write(&file, &bytes).unwrap();
    let ptr = f.store().join("CURRENT");
    let mut bytes = fs::read(&ptr).unwrap();
    bytes[16..48].copy_from_slice(&digest);
    let checksum = Sha256::digest(&bytes[..48]);
    bytes[48..].copy_from_slice(&checksum);
    fs::write(ptr, bytes).unwrap();
    assert!(matches!(
        DurableFullIndex::open(&f.store(), &f.source, FullLimits::default()),
        Err(FullStoreError::Storage(DurableError::Corrupt))
    ));
}

#[test]
fn reopened_postings_match_raw_original_queries_and_positions() {
    let cases: [(&str, &str, &[u8], &str); 5] = [
        (
            "main.c",
            "c",
            include_bytes!("../../../tests/fixtures/public-source/c/main.c"),
            include_str!("../../../tests/evidence/full-postings-2026-11-05/corpus-c.stdout"),
        ),
        (
            "main.py",
            "python",
            include_bytes!("../../../tests/fixtures/public-source/python/main.py.fixture"),
            include_str!("../../../tests/evidence/full-postings-2026-11-05/corpus-python.stdout"),
        ),
        (
            "function.js",
            "javascript",
            include_bytes!("../../../tests/fixtures/public-source/javascript/function.js"),
            include_str!("../../../tests/evidence/full-postings-2026-11-05/corpus-js.stdout"),
        ),
        (
            "unicode.c",
            "unicode",
            "😀\tİ\r\nAΣ 中\n".as_bytes(),
            include_str!("../../../tests/evidence/full-postings-2026-11-05/unicode-c.stdout"),
        ),
        (
            "plain.py",
            "plain",
            b"Alpha\tBeta\r\nalpha_1\x00end\n",
            include_str!("../../../tests/evidence/full-postings-2026-11-05/plain-python.stdout"),
        ),
    ];
    for (name, _, bytes, raw) in cases {
        let f = Fixture::new();
        fs::create_dir(f.source.join("alpha")).unwrap();
        fs::write(f.source.join("alpha").join(name), bytes).unwrap();
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
        drop(db);
        let db = DurableFullIndex::open(&f.store(), &f.source, FullLimits::default()).unwrap();
        let snapshot = db.snapshot().unwrap();
        for line in raw.lines().filter(|l| l.starts_with("QUERY\t")) {
            let parts: Vec<_> = line.split('\t').collect();
            let utf16 = parts[1]
                .as_bytes()
                .chunks_exact(4)
                .map(|chunk| u16::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            let term = String::from_utf16(&utf16).unwrap();
            let got = snapshot.lookup("alpha", &term).unwrap();
            assert_eq!(
                got.len(),
                parts[4].parse::<usize>().unwrap(),
                "{name} {term}"
            );
            let expected = raw
                .lines()
                .filter(|l| l.starts_with(&format!("P\t{}\t", parts[1])))
                .map(|l| {
                    let p: Vec<_> = l.split('\t').collect();
                    (
                        p[3].parse::<usize>().unwrap(),
                        p[4].parse::<usize>().unwrap(),
                        p[5].parse::<usize>().unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            let actual = got
                .iter()
                .flat_map(|m| {
                    m.positions
                        .iter()
                        .map(|p| (p.position, p.start_utf16, p.end_utf16))
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{name} {term}");
        }
    }
}

#[test]
fn aggregate_limits_fixed_registry_and_retained_reader_hold_previous_batch() {
    let f = Fixture::new();
    fs::create_dir(f.source.join("alpha")).unwrap();
    fs::create_dir(f.source.join("beta")).unwrap();
    fs::write(f.source.join("alpha/a.c"), b"old").unwrap();
    fs::write(f.source.join("beta/b.js"), b"old").unwrap();
    let limits = FullLimits {
        max_source_bytes: 12,
        max_postings: 4,
        ..FullLimits::default()
    };
    let db = DurableFullIndex::create(
        &f.parent,
        "alpha-store",
        &f.source,
        "utf8-default-tabs",
        limits,
    )
    .unwrap();
    let projects = vec!["alpha".into(), "beta".into()];
    db.replace_from_source(None, &projects, &CancellationToken::new())
        .unwrap();
    let old = db.snapshot().unwrap();
    fs::write(f.source.join("alpha/a.c"), b"a b c").unwrap();
    fs::write(f.source.join("beta/b.js"), b"d e").unwrap();
    assert!(matches!(
        db.replace_from_source(Some(1), &projects, &CancellationToken::new()),
        Err(FullStoreError::LimitExceeded)
    ));
    fs::write(f.source.join("beta/b.js"), b"abcdefgh").unwrap();
    assert!(matches!(
        db.replace_from_source(Some(1), &projects, &CancellationToken::new()),
        Err(FullStoreError::LimitExceeded)
    ));
    assert!(db
        .replace_from_source(Some(1), &["alpha".into()], &CancellationToken::new())
        .is_err());
    fs::write(f.source.join("alpha/a.c"), b"new").unwrap();
    fs::write(f.source.join("beta/b.js"), b"new").unwrap();
    assert_eq!(
        db.replace_from_source(Some(1), &projects, &CancellationToken::new())
            .unwrap(),
        2
    );
    for p in ["alpha", "beta"] {
        assert_eq!(old.lookup(p, "old").unwrap().len(), 1);
        assert_eq!(db.snapshot().unwrap().lookup(p, "new").unwrap().len(), 1);
    }
    // Bounds live memory generations as well as individual files and the batch.
    assert!(matches!(
        db.replace_from_source(Some(2), &projects, &CancellationToken::new()),
        Err(FullStoreError::LimitExceeded)
    ));
    drop(old);
    assert_eq!(
        db.replace_from_source(Some(2), &projects, &CancellationToken::new())
            .unwrap(),
        3
    );
}

#[cfg(feature = "fault-injection")]
#[test]
fn storage_failures_keep_old_generation_and_indeterminate_reopen_recovers_new() {
    use rustgrok_path_index::durable::FaultPoint;
    for fault in [
        FaultPoint::NoSpaceGenerationWrite,
        FaultPoint::NoSpacePointerWrite,
        FaultPoint::CorruptGenerationBeforeReadback,
        FaultPoint::CorruptPointerBeforeReadback,
        FaultPoint::FailDirectorySync,
    ] {
        let f = Fixture::new();
        let db = publish(&f);
        let pointer = fs::read(f.store().join("CURRENT")).unwrap();
        fs::write(f.source.join("alpha/a.c"), b"new alpha").unwrap();
        let result =
            db.replace_with_fault(Some(1), &["alpha".into()], &CancellationToken::new(), fault);
        assert!(result.is_err());
        if fault == FaultPoint::FailDirectorySync {
            assert!(matches!(
                result,
                Err(FullStoreError::Storage(
                    DurableError::PublicationIndeterminate
                ))
            ));
            assert!(matches!(
                db.snapshot(),
                Err(FullStoreError::Storage(
                    DurableError::PublicationIndeterminate
                ))
            ));
        } else {
            assert_eq!(fs::read(f.store().join("CURRENT")).unwrap(), pointer);
            assert_eq!(
                db.snapshot()
                    .unwrap()
                    .lookup("alpha", "beta")
                    .unwrap()
                    .len(),
                1
            );
        }
        drop(db);
        let db = DurableFullIndex::open(&f.store(), &f.source, FullLimits::default()).unwrap();
        let snapshot = db.snapshot().unwrap();
        assert_eq!(
            snapshot.generation(),
            if fault == FaultPoint::FailDirectorySync {
                2
            } else {
                1
            }
        );
    }
}

#[test]
fn checksum_valid_document_with_hidden_unsupported_path_is_rejected() {
    let f = Fixture::new();
    drop(publish(&f));
    let file = f.store().join("F-1");
    let mut bytes = fs::read(&file).unwrap();
    let at = bytes.windows(10).position(|b| b == b"/alpha/a.c").unwrap();
    bytes[at..at + 10].copy_from_slice(b"/alpha/.c.");
    let len = bytes.len();
    let digest = Sha256::digest(&bytes[..len - 32]);
    bytes[len - 32..].copy_from_slice(&digest);
    fs::write(file, bytes).unwrap();
    let ptr = f.store().join("CURRENT");
    let mut bytes = fs::read(&ptr).unwrap();
    bytes[16..48].copy_from_slice(&digest);
    let checksum = Sha256::digest(&bytes[..48]);
    bytes[48..].copy_from_slice(&checksum);
    fs::write(ptr, bytes).unwrap();
    assert!(matches!(
        DurableFullIndex::open(&f.store(), &f.source, FullLimits::default()),
        Err(FullStoreError::Storage(DurableError::Corrupt))
    ));
}
