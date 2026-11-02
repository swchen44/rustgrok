// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_analysis::{analyze_path, PathToken};

fn token(term: &str, position: usize, start: usize, end: usize) -> PathToken {
    PathToken {
        term: term.into(),
        position,
        start_utf16: start,
        end_utf16: end,
    }
}

#[test]
fn reproduces_original_path_tokens_and_observed_offsets() {
    let cases: Vec<(&str, Vec<PathToken>)> = vec![
        ("", vec![]),
        (
            "/A/B.c",
            vec![
                token("a", 0, 0, 1),
                token("b", 1, 2, 3),
                token(".", 2, 4, 5),
                token("c", 3, 5, 6),
            ],
        ),
        (
            "//A///B.c",
            vec![
                token("a", 0, 0, 1),
                token("b", 1, 2, 3),
                token(".", 2, 4, 5),
                token("c", 3, 5, 6),
            ],
        ),
        (".foo", vec![token(".foo", 0, 0, 4)]),
        ("foo.", vec![token("foo", 0, 0, 3), token(".", 1, 4, 5)]),
        (
            "a..b",
            vec![
                token("a", 0, 0, 1),
                token(".", 1, 2, 3),
                token(".b", 2, 3, 5),
            ],
        ),
        ("a b", vec![token("a", 0, 0, 1), token("b", 1, 2, 3)]),
        ("  a", vec![token(" ", 0, 0, 1), token("a", 1, 2, 3)]),
        ("ab c", vec![token("ab", 0, 0, 2), token("c", 1, 3, 4)]),
        ("/", vec![]),
        ("foo//", vec![token("foo", 0, 0, 3)]),
    ];
    for (input, expected) in cases {
        assert_eq!(analyze_path(input).unwrap(), expected, "input {input:?}");
    }
}

#[test]
fn long_tokens_are_not_truncated() {
    assert_eq!(
        analyze_path(&"A".repeat(100)).unwrap(),
        vec![token(&"a".repeat(100), 0, 0, 100)]
    );
}

#[test]
fn unicode_and_control_inputs_are_explicitly_unsupported() {
    for (input, offset) in [("a🦀.c", 1), ("İ.c", 0), ("x\ny", 1), ("x\u{7f}", 1)] {
        assert_eq!(
            analyze_path(input).unwrap_err().byte_offset,
            offset,
            "{input:?}"
        );
    }
}

#[test]
fn index_and_query_terms_match_the_recorded_public_corpus_counts() {
    let paths = [
        "/beta/component-c/main.c",
        "/beta/component-py/main.py",
        "/alpha/component-c/main.c",
        "/alpha/component-c/nested-js/function.js",
        "/alpha/component-js/function.js",
    ];
    for (query, expected_count) in [("main", 3), ("mai", 0), ("ain", 0)] {
        let query_terms = analyze_path(query).unwrap();
        let count = paths
            .iter()
            .filter(|path| {
                let terms = analyze_path(path).unwrap();
                query_terms
                    .iter()
                    .all(|query_term| terms.iter().any(|term| term.term == query_term.term))
            })
            .count();
        assert_eq!(count, expected_count, "{query}");
    }
}
