// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_analysis::analyze_path;

#[test]
fn original_utf16_offsets_are_distinct_from_real_utf8_source_spans() {
    let input = "//專案///🦀.İ";
    let tokens = analyze_path(input).unwrap();
    let observed: Vec<_> = tokens
        .iter()
        .map(|t| {
            (
                t.term.as_str(),
                t.start_utf16,
                t.end_utf16,
                t.start_byte,
                t.end_byte,
            )
        })
        .collect();
    assert_eq!(
        observed,
        vec![
            ("專案", 0, 2, 2, 8),
            ("🦀", 3, 5, 11, 15),
            (".", 6, 7, 15, 16),
            ("i\u{307}", 7, 9, 16, 18),
        ]
    );
    for token in &tokens {
        assert!(input.is_char_boundary(token.start_byte));
        assert!(input.is_char_boundary(token.end_byte));
        assert!(!input[token.start_byte..token.end_byte].is_empty());
    }
}

#[test]
fn unsupported_characters_and_unverified_sigma_contexts_fail_without_tokens() {
    for (input, offset) in [
        ("ok/é.c", 3),
        ("ok/\u{3000}.c", 3),
        ("ok/i\u{307}.c", 4),
        ("ok/AΣ_B.c", 4),
        ("FOOΣ", 3),
        ("ok/漢Σ.c", 6),
        ("ok/a\nb.c", 4),
    ] {
        assert_eq!(
            analyze_path(input).unwrap_err().byte_offset,
            offset,
            "{input:?}"
        );
    }
}
