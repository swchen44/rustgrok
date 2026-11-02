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
 * Copyright (c) 2008, 2023, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2017, 2019, Chris Fraire <cfraire@me.com>.
 * Portions Copyright (c) 2020, Ric Harris <harrisric@users.noreply.github.com>.
 * Portions Copyright (c) 2026, swchen44.
 * Based on the add/remove scenario in OpenGrok IndexerTest.java,
 * commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3.
 * Q8 publication/failure tests are additional Rustgrok requirements.
 */
// SPDX-License-Identifier: CDDL-1.0
use rustgrok_path_index::{CancellationToken, IndexError, IndexLimits, PathIndex};
use std::{
    collections::HashSet,
    io,
    sync::{mpsc, Arc},
    thread,
};

fn limits() -> IndexLimits {
    IndexLimits {
        max_documents: 20,
        max_path_bytes: 128,
        max_total_path_bytes: 1024,
        max_live_generations: 2,
    }
}

fn paths(values: &[&str]) -> Vec<io::Result<String>> {
    values.iter().map(|value| Ok((*value).to_owned())).collect()
}
fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn public_index_add_remove_and_project_query() {
    let index = PathIndex::new("alpha", limits()).unwrap();
    assert!(matches!(
        index.snapshot(),
        Err(IndexError::NoPublishedGeneration)
    ));
    let original = ["/alpha/src/main.c", "/alpha/lib/helper.py"];
    assert_eq!(
        index
            .replace(None, 2, paths(&original), &CancellationToken::new())
            .unwrap(),
        1
    );
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&[original[0]])
    );
    let added = [original[0], original[1], "/alpha/web/main.js"];
    assert_eq!(
        index
            .replace(Some(1), 3, paths(&added), &CancellationToken::new())
            .unwrap(),
        2
    );
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&[added[0], added[2]])
    );
    assert_eq!(
        index
            .replace(Some(2), 1, paths(&[original[1]]), &CancellationToken::new())
            .unwrap(),
        3
    );
    assert!(index.snapshot().unwrap().lookup("main").unwrap().is_empty());
}

#[test]
fn old_readers_remain_available_during_build_and_after_publication() {
    let index = Arc::new(PathIndex::new("alpha", limits()).unwrap());
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    let held = index.snapshot().unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (continue_tx, continue_rx) = mpsc::channel();
    let worker_index = Arc::clone(&index);
    let worker = thread::spawn(move || {
        let mut first = true;
        let iterator = std::iter::from_fn(move || {
            if !first {
                return None;
            }
            first = false;
            started_tx.send(()).unwrap();
            continue_rx.recv().unwrap();
            Some(Ok("/alpha/other.c".to_owned()))
        });
        worker_index.replace(Some(1), 1, iterator, &CancellationToken::new())
    });
    started_rx.recv().unwrap();
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&["/alpha/main.c"])
    );
    assert_eq!(held.lookup("main").unwrap(), set(&["/alpha/main.c"]));
    continue_tx.send(()).unwrap();
    assert_eq!(worker.join().unwrap().unwrap(), 2);
    assert!(index.snapshot().unwrap().lookup("main").unwrap().is_empty());
    assert_eq!(held.generation(), 1);
    assert_eq!(held.lookup("main").unwrap(), set(&["/alpha/main.c"]));
}

#[test]
fn invalid_partial_cancelled_or_io_failed_inputs_never_publish() {
    let index = PathIndex::new("alpha", limits()).unwrap();
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    let attempts: Vec<(usize, Vec<io::Result<String>>)> = vec![
        (
            2,
            vec![
                Ok("/alpha/other.c".into()),
                Err(io::Error::other("source read failed")),
            ],
        ),
        (
            2,
            vec![
                Ok("/alpha/other.c".into()),
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "corrupt manifest",
                )),
            ],
        ),
        (2, paths(&["/alpha/other.c"])),
        (1, paths(&["/alpha/other.c", "/alpha/main.c"])),
        (2, paths(&["/alpha/main.c", "/alpha/main.c"])),
        (1, paths(&["/beta/main.c"])),
        (1, paths(&["/alpha/../main.c"])),
        (1, paths(&["/alpha/é.c"])),
    ];
    for (count, input) in attempts {
        assert!(index
            .replace(Some(1), count, input, &CancellationToken::new())
            .is_err());
        let current = index.snapshot().unwrap();
        assert_eq!(current.generation(), 1);
        assert_eq!(current.lookup("main").unwrap(), set(&["/alpha/main.c"]));
    }
    let cancellation = CancellationToken::new();
    let source = paths(&["/alpha/other.c", "/alpha/third.c"])
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            if index == 1 {
                assert!(cancellation.cancel());
            }
            item
        });
    assert!(matches!(
        index.replace(Some(1), 2, source, &cancellation),
        Err(IndexError::Cancelled)
    ));
    assert_eq!(index.snapshot().unwrap().generation(), 1);
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&["/alpha/main.c"])
    );
    assert!(index.snapshot().unwrap().lookup("main.c").is_err());
}

#[test]
fn lookup_rejects_unimplemented_query_syntax_instead_of_returning_false_no_hits() {
    let index = PathIndex::new("alpha", limits()).unwrap();
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    let view = index.snapshot().unwrap();
    for query in [
        "*",
        "main*",
        "main?",
        "/alpha",
        "main.c",
        "main OR missing",
        "\"main\"",
        "-main",
        "AND",
        "OR",
        "NOT",
        "",
    ] {
        assert!(
            matches!(view.lookup(query), Err(IndexError::UnsupportedQuery)),
            "{query:?}"
        );
    }
    assert_eq!(view.lookup("main").unwrap(), set(&["/alpha/main.c"]));
}

#[test]
fn concurrent_and_stale_writers_fail_closed_without_overwriting() {
    let index = Arc::new(PathIndex::new("alpha", limits()).unwrap());
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (finish_tx, finish_rx) = mpsc::channel();
    let worker_index = Arc::clone(&index);
    let worker = thread::spawn(move || {
        let mut first = true;
        let source = std::iter::from_fn(move || {
            if !first {
                return None;
            }
            first = false;
            started_tx.send(()).unwrap();
            finish_rx.recv().unwrap();
            Some(Ok("/alpha/new.c".to_string()))
        });
        worker_index.replace(Some(1), 1, source, &CancellationToken::new())
    });
    started_rx.recv().unwrap();
    assert!(matches!(
        index.replace(
            Some(1),
            1,
            paths(&["/alpha/late.c"]),
            &CancellationToken::new()
        ),
        Err(IndexError::WriterBusy)
    ));
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&["/alpha/main.c"])
    );
    finish_tx.send(()).unwrap();
    assert_eq!(worker.join().unwrap().unwrap(), 2);
    assert!(matches!(
        index.replace(
            Some(1),
            1,
            paths(&["/alpha/late.c"]),
            &CancellationToken::new()
        ),
        Err(IndexError::StaleGeneration { .. })
    ));
    assert_eq!(
        index.snapshot().unwrap().lookup("new").unwrap(),
        set(&["/alpha/new.c"])
    );
}

#[test]
fn panicking_source_keeps_previous_snapshot_and_poisoned_writer_fails_closed() {
    let index = PathIndex::new("alpha", limits()).unwrap();
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    let source =
        std::iter::from_fn(|| -> Option<io::Result<String>> { panic!("source iterator failed") });
    assert!(std::panic::catch_unwind(|| index.replace(
        Some(1),
        1,
        source,
        &CancellationToken::new()
    ))
    .is_err());
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&["/alpha/main.c"])
    );
    assert!(matches!(
        index.replace(
            Some(1),
            1,
            paths(&["/alpha/new.c"]),
            &CancellationToken::new()
        ),
        Err(IndexError::LockPoisoned)
    ));
}

#[test]
fn caller_supplied_limits_reject_oversized_manifests_and_preserve_generation() {
    let index = PathIndex::new(
        "alpha",
        IndexLimits {
            max_documents: 2,
            max_path_bytes: 24,
            max_total_path_bytes: 24,
            max_live_generations: 2,
        },
    )
    .unwrap();
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    for (count, input) in [
        (3, paths(&["/alpha/a", "/alpha/b", "/alpha/c"])),
        (1, paths(&["/alpha/this-path-is-far-too-long.c"])),
        (2, paths(&["/alpha/first.c", "/alpha/second.c"])),
    ] {
        assert!(matches!(
            index.replace(Some(1), count, input, &CancellationToken::new()),
            Err(IndexError::LimitExceeded)
        ));
        assert_eq!(
            index.snapshot().unwrap().lookup("main").unwrap(),
            set(&["/alpha/main.c"])
        );
    }
    assert!(matches!(
        index.snapshot().unwrap().lookup(&"x".repeat(25)),
        Err(IndexError::LimitExceeded)
    ));
}

#[test]
fn candidate_sets_match_the_recorded_original_for_three_single_terms() {
    let index = PathIndex::new("alpha", limits()).unwrap();
    let corpus = [
        "/alpha/component-c/main.c",
        "/alpha/component-c/nested-js/function.js",
        "/alpha/component-js/function.js",
    ];
    index
        .replace(
            None,
            corpus.len(),
            paths(&corpus),
            &CancellationToken::new(),
        )
        .unwrap();
    for (query, raw) in [
        (
            "main",
            include_str!(
                "../../../tests/evidence/path-ranking-diagnostic-2026-11-01/main.oracle.txt"
            ),
        ),
        (
            "mai",
            include_str!(
                "../../../tests/evidence/path-ranking-diagnostic-2026-11-01/mai.oracle.txt"
            ),
        ),
        (
            "ain",
            include_str!(
                "../../../tests/evidence/path-ranking-diagnostic-2026-11-01/ain.oracle.txt"
            ),
        ),
    ] {
        let expected: HashSet<String> = raw
            .lines()
            .filter(|line| line.starts_with("doc="))
            .filter_map(|line| line.split_once(" path=").map(|(_, path)| path))
            .filter(|path| path.starts_with("/alpha/"))
            .map(str::to_owned)
            .collect();
        assert_eq!(
            index.snapshot().unwrap().lookup(query).unwrap(),
            expected,
            "{query}"
        );
    }
}

#[test]
fn public_synthetic_source_add_then_remove_updates_candidates_without_changing_source() {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("clean up owned fixture");
        }
    }
    let root = std::env::temp_dir().join(format!(
        "rustgrok-path-fixture-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    let main = fixture.0.join("main.c");
    let added = fixture.0.join("main.js");
    let initial_bytes = b"int main(void) { return 0; }\n";
    fs::write(&main, initial_bytes).unwrap();
    let index = PathIndex::new("alpha", limits()).unwrap();
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&["/alpha/main.c"])
    );
    fs::write(&added, b"export function main() {}\n").unwrap();
    index
        .replace(
            Some(1),
            2,
            paths(&["/alpha/main.c", "/alpha/main.js"]),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&["/alpha/main.c", "/alpha/main.js"])
    );
    assert_eq!(fs::read(&main).unwrap(), initial_bytes);
    assert_eq!(fs::read(&added).unwrap(), b"export function main() {}\n");
    fs::remove_file(&added).unwrap();
    index
        .replace(
            Some(2),
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&["/alpha/main.c"])
    );
    assert_eq!(fs::read(&main).unwrap(), initial_bytes);
}

#[test]
fn cancellation_and_publish_have_a_single_atomic_winner() {
    let index = Arc::new(PathIndex::new("alpha", limits()).unwrap());
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    let token = Arc::new(CancellationToken::new());
    let worker_token = Arc::clone(&token);
    let worker_index = Arc::clone(&index);
    let (started_tx, started_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
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
        worker_index.replace(Some(1), 1, source, &worker_token)
    });
    started_rx.recv().unwrap();
    assert!(
        token.cancel(),
        "cancellation must win before publication starts"
    );
    resume_tx.send(()).unwrap();
    assert!(matches!(worker.join().unwrap(), Err(IndexError::Cancelled)));
    assert_eq!(
        index.snapshot().unwrap().lookup("main").unwrap(),
        set(&["/alpha/main.c"])
    );
    let completed = CancellationToken::new();
    assert_eq!(
        index
            .replace(Some(1), 1, paths(&["/alpha/new.c"]), &completed)
            .unwrap(),
        2
    );
    assert!(
        !completed.cancel(),
        "completed publication must report cancellation too late"
    );
    assert!(matches!(
        index.replace(Some(2), 1, paths(&["/alpha/late.c"]), &completed),
        Err(IndexError::CancellationTokenUsed)
    ));
    assert_eq!(
        index.snapshot().unwrap().lookup("new").unwrap(),
        set(&["/alpha/new.c"])
    );
}

#[test]
fn published_generation_retains_the_exact_supplied_manifest() {
    let index = PathIndex::new("alpha", limits()).unwrap();
    let first = ["/alpha/src/main.c", "/alpha/lib/helper.py"];
    index
        .replace(None, first.len(), paths(&first), &CancellationToken::new())
        .unwrap();
    let old = index.snapshot().unwrap();
    assert_eq!(old.manifest(), set(&first));
    let second = ["/alpha/lib/helper.py", "/alpha/web/main.js"];
    index
        .replace(
            Some(1),
            second.len(),
            paths(&second),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(index.snapshot().unwrap().manifest(), set(&second));
    assert_eq!(old.manifest(), set(&first));
}

#[test]
fn cancellation_racing_publication_never_reports_success_for_a_published_generation() {
    for _ in 0..12 {
        let index = Arc::new(PathIndex::new("alpha", limits()).unwrap());
        index
            .replace(
                None,
                1,
                paths(&["/alpha/main.c"]),
                &CancellationToken::new(),
            )
            .unwrap();
        let token = Arc::new(CancellationToken::new());
        let worker_index = Arc::clone(&index);
        let worker_token = Arc::clone(&token);
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let worker_barrier = Arc::clone(&barrier);
        let worker = thread::spawn(move || {
            let mut step = 0;
            let source = std::iter::from_fn(move || {
                step += 1;
                if step == 1 {
                    return Some(Ok("/alpha/new.c".to_owned()));
                }
                worker_barrier.wait();
                None
            });
            worker_index.replace(Some(1), 1, source, &worker_token)
        });
        barrier.wait();
        let cancelled = token.cancel();
        let outcome = worker.join().unwrap();
        let current = index.snapshot().unwrap();
        if cancelled {
            assert!(matches!(outcome, Err(IndexError::Cancelled)));
            assert_eq!(current.lookup("main").unwrap(), set(&["/alpha/main.c"]));
        } else {
            assert_eq!(outcome.unwrap(), 2);
            assert_eq!(current.lookup("new").unwrap(), set(&["/alpha/new.c"]));
        }
    }
}

#[test]
fn an_explicitly_complete_empty_manifest_can_replace_a_nonempty_generation() {
    let index = PathIndex::new("alpha", limits()).unwrap();
    index
        .replace(
            None,
            1,
            paths(&["/alpha/main.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    index
        .replace(Some(1), 0, paths(&[]), &CancellationToken::new())
        .unwrap();
    let view = index.snapshot().unwrap();
    assert_eq!(view.generation(), 2);
    assert!(view.manifest().is_empty());
    assert!(view.lookup("main").unwrap().is_empty());
}

#[test]
fn retained_old_readers_bound_live_generations_without_losing_current() {
    let index = PathIndex::new("alpha", limits()).unwrap();
    index
        .replace(
            None,
            1,
            paths(&["/alpha/first.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    let old = index.snapshot().unwrap();
    index
        .replace(
            Some(1),
            1,
            paths(&["/alpha/second.c"]),
            &CancellationToken::new(),
        )
        .unwrap();
    assert!(matches!(
        index.replace(
            Some(2),
            1,
            paths(&["/alpha/third.c"]),
            &CancellationToken::new()
        ),
        Err(IndexError::LimitExceeded)
    ));
    assert_eq!(old.lookup("first").unwrap(), set(&["/alpha/first.c"]));
    assert_eq!(
        index.snapshot().unwrap().lookup("second").unwrap(),
        set(&["/alpha/second.c"])
    );
    drop(old);
    assert_eq!(
        index
            .replace(
                Some(2),
                1,
                paths(&["/alpha/third.c"]),
                &CancellationToken::new()
            )
            .unwrap(),
        3
    );
}
