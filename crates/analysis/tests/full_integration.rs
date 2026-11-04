// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_analysis::analyze_full;

fn original(raw: &str) -> Vec<(usize, usize, usize, String)> {
    raw.lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            if fields[0] != "T" {
                return None;
            }
            let units = fields[4]
                .as_bytes()
                .chunks_exact(4)
                .map(|u| u16::from_str_radix(std::str::from_utf8(u).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            Some((
                fields[1].parse().unwrap(),
                fields[2].parse().unwrap(),
                fields[3].parse().unwrap(),
                String::from_utf16(&units).unwrap(),
            ))
        })
        .collect()
}

#[test]
fn public_c_python_and_javascript_sources_match_original_full_streams() {
    for (source, oracle) in [
        (include_str!("../../../tests/fixtures/public-source/c/main.c"), include_str!("../../../tests/evidence/full-analysis-2026-11-04/corpus-c-c.stdout")),
        (include_str!("../../../tests/fixtures/public-source/python/main.py.fixture"), include_str!("../../../tests/evidence/full-analysis-2026-11-04/corpus-python-python.stdout")),
        (include_str!("../../../tests/fixtures/public-source/javascript/function.js"), include_str!("../../../tests/evidence/full-analysis-2026-11-04/corpus-javascript-javascript.stdout")),
    ] {
        let tokens = analyze_full(source).unwrap();
        let actual = tokens.iter().map(|t|(t.position,t.start_utf16,t.end_utf16,t.term.clone())).collect::<Vec<_>>();
        assert_eq!(actual,original(oracle));
        for token in tokens { assert_eq!((token.start_byte,token.end_byte),(token.start_utf16,token.end_utf16)); }
    }
}

#[test]
fn ascii_lexical_rules_match_original_full_stream() {
    let source = include_str!("../../../tests/fixtures/full-analysis-2026-11-04/ascii.txt");
    let oracle =
        include_str!("../../../tests/evidence/full-analysis-2026-11-04/ascii-query.stdout");
    let actual = analyze_full(source).unwrap();
    assert_eq!(
        actual
            .iter()
            .map(|t| (t.position, t.start_utf16, t.end_utf16, t.term.clone()))
            .collect::<Vec<_>>(),
        original(oracle)
    );
}

#[test]
fn unicode_full_offsets_preserve_upstream_arithmetic_and_source_byte_spans() {
    let source = include_str!("../../../tests/fixtures/full-analysis-2026-11-04/unicode.txt");
    let oracle =
        include_str!("../../../tests/evidence/full-analysis-2026-11-04/unicode-query.stdout");
    let actual = analyze_full(source).unwrap();
    assert_eq!(
        actual
            .iter()
            .map(|t| (t.position, t.start_utf16, t.end_utf16, t.term.clone()))
            .collect::<Vec<_>>(),
        original(oracle)
    );
    let original_spans = [
        "中文",
        "繁體",
        "abc",
        "ZİX",
        "İSTANBUL",
        "next",
        "AΣ",
        "ΣA",
        "AΣA",
        "Σ",
        "İΣ",
    ];
    assert_eq!(actual.len(), original_spans.len());
    for (token, raw) in actual.iter().zip(original_spans) {
        assert_eq!(&source[token.start_byte..token.end_byte], raw);
    }
    assert_ne!(
        actual[3].end_utf16 - actual[3].start_utf16,
        "ZİX".encode_utf16().count()
    );
}

#[test]
fn finite_sigma_context_matrix_matches_original() {
    let source = include_str!("../../../tests/fixtures/full-analysis-2026-11-04/sigma-matrix.txt");
    let raw =
        include_str!("../../../tests/evidence/full-analysis-2026-11-04/sigma-matrix-query.stdout");
    let tokens = analyze_full(source).unwrap();
    assert_eq!(
        tokens
            .iter()
            .map(|t| (t.position, t.start_utf16, t.end_utf16, t.term.clone()))
            .collect::<Vec<_>>(),
        original(raw)
    );
}

#[test]
fn long_ascii_tokenization_and_index_byte_limit_are_distinct() {
    use rustgrok_analysis::{validate_full_index_terms, FullError};
    let source = include_str!("../../../tests/fixtures/full-analysis-2026-11-04/long.txt");
    let raw = include_str!("../../../tests/evidence/full-analysis-2026-11-04/long-query.stdout");
    let tokens = analyze_full(source).unwrap();
    assert_eq!(
        tokens
            .iter()
            .map(|t| (t.position, t.start_utf16, t.end_utf16, t.term.clone()))
            .collect::<Vec<_>>(),
        original(raw)
    );
    assert_eq!(
        tokens.iter().map(|t| t.term.len()).collect::<Vec<_>>(),
        vec![32766, 32766, 32766]
    );
    assert!(validate_full_index_terms(&tokens).is_ok());
    let chinese = "中".repeat(11000);
    let tokens = analyze_full(&chinese).unwrap();
    assert_eq!(tokens[0].term.len(), 33000);
    assert_eq!(
        validate_full_index_terms(&tokens),
        Err(FullError::TermTooLongForIndex { byte_offset: 0 })
    );
}

#[test]
fn query_term_normalization_is_distinct_from_full_tokenization() {
    use rustgrok_analysis::normalize_full_query_term;
    for (input, raw) in [
        (
            "HELLO_12",
            include_str!(
                "../../../tests/evidence/full-analysis-2026-11-04/norm-ascii-query.stdout"
            ),
        ),
        (
            "İ",
            include_str!(
                "../../../tests/evidence/full-analysis-2026-11-04/norm-turkish-query.stdout"
            ),
        ),
        (
            "AΣ",
            include_str!(
                "../../../tests/evidence/full-analysis-2026-11-04/norm-sigma-query.stdout"
            ),
        ),
        (
            "中文",
            include_str!(
                "../../../tests/evidence/full-analysis-2026-11-04/norm-chinese-query.stdout"
            ),
        ),
    ] {
        let encoded = raw.trim_end().strip_prefix("N\t").unwrap();
        let units = encoded
            .as_bytes()
            .chunks_exact(4)
            .map(|u| u16::from_str_radix(std::str::from_utf8(u).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            normalize_full_query_term(input).unwrap(),
            String::from_utf16(&units).unwrap()
        );
    }
    assert_eq!(analyze_full("İ").unwrap()[0].term, "i\u{307}");
    assert_eq!(analyze_full("AΣ").unwrap()[0].term, "aς");
}

#[test]
fn unsupported_unicode_context_and_query_syntax_fail_without_partial_results() {
    use rustgrok_analysis::{normalize_full_query_term, FullError};
    assert_eq!(
        analyze_full("ok 🌐"),
        Err(FullError::UnsupportedCharacter { byte_offset: 3 })
    );
    assert_eq!(
        analyze_full("ok \u{307}"),
        Err(FullError::UnsupportedCharacter { byte_offset: 3 })
    );
    assert_eq!(
        analyze_full("ok AΣAΣ"),
        Err(FullError::UnsupportedCaseContext { byte_offset: 4 })
    );
    let split = format!("{}𐐀", "a".repeat(32765));
    assert_eq!(
        analyze_full(&split),
        Err(FullError::UnsupportedCharacter { byte_offset: 32765 })
    );
    assert_eq!(
        normalize_full_query_term("hello?"),
        Err(FullError::InvalidQueryTerm)
    );
    assert_eq!(
        normalize_full_query_term(""),
        Err(FullError::InvalidQueryTerm)
    );
    assert_eq!(
        normalize_full_query_term("hello🌐"),
        Err(FullError::UnsupportedCharacter { byte_offset: 5 })
    );
}

#[test]
fn every_ascii_character_matches_original_full_stream() {
    let source = include_str!("../../../tests/fixtures/full-analysis-2026-11-04/ascii-all.txt");
    let raw = include_str!(
        "../../../tests/evidence/full-analysis-2026-11-04/extra/ascii-all-query.stdout"
    );
    let tokens = analyze_full(source).unwrap();
    assert_eq!(
        tokens
            .iter()
            .map(|t| (t.position, t.start_utf16, t.end_utf16, t.term.clone()))
            .collect::<Vec<_>>(),
        original(raw)
    );
}

#[test]
fn cjk_term_stream_and_surrogate_boundary_are_separately_observed() {
    use rustgrok_analysis::{validate_full_index_terms, FullError};
    let source = include_str!("../../../tests/fixtures/full-analysis-2026-11-04/cjk-limit.txt");
    let raw = include_str!(
        "../../../tests/evidence/full-analysis-2026-11-04/extra/cjk-limit-query.stdout"
    );
    let tokens = analyze_full(source).unwrap();
    assert_eq!(
        tokens
            .iter()
            .map(|t| (t.position, t.start_utf16, t.end_utf16, t.term.clone()))
            .collect::<Vec<_>>(),
        original(raw)
    );
    assert_eq!(
        validate_full_index_terms(&tokens),
        Err(FullError::TermTooLongForIndex { byte_offset: 0 })
    );
    let split =
        include_str!("../../../tests/fixtures/full-analysis-2026-11-04/surrogate-boundary.txt");
    let raw = include_str!(
        "../../../tests/evidence/full-analysis-2026-11-04/extra/surrogate-boundary-query.stdout"
    );
    let line = raw.lines().find(|line| line.starts_with("T\t")).unwrap();
    let fields = line.split('\t').collect::<Vec<_>>();
    assert_eq!(&fields[..4], &["T", "0", "0", "32767"]);
    assert_eq!(fields[4].len(), 32766 * 4);
    assert!(fields[4].ends_with("d801"));
    assert_eq!(
        analyze_full(split),
        Err(FullError::UnsupportedCharacter { byte_offset: 32765 })
    );
    assert_eq!(
        analyze_full(&"中".repeat(32767)),
        Err(FullError::UnsupportedLongUnicode { byte_offset: 0 })
    );
}

#[test]
fn finite_query_lowercase_mapping_matches_original() {
    use rustgrok_analysis::normalize_full_query_term;
    let source = include_str!("../../../tests/fixtures/full-analysis-2026-11-04/norm-mapping.txt");
    let raw = include_str!(
        "../../../tests/evidence/full-analysis-2026-11-04/extra/norm-mapping-query.stdout"
    );
    let hex = raw.trim_end().strip_prefix("N\t").unwrap();
    let units = hex
        .as_bytes()
        .chunks_exact(4)
        .map(|chunk| u16::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        normalize_full_query_term(source).unwrap(),
        String::from_utf16(&units).unwrap()
    );
}

#[test]
fn unverified_sigma_context_is_rejected_instead_of_guessed() {
    use rustgrok_analysis::FullError;
    assert_eq!(
        analyze_full("aΣ"),
        Err(FullError::UnsupportedCaseContext { byte_offset: 1 })
    );
    assert_eq!(
        analyze_full("Σabc"),
        Err(FullError::UnsupportedCaseContext { byte_offset: 0 })
    );
}

#[test]
fn sigma_admission_bounds_each_side_without_restricting_query_normalization() {
    use rustgrok_analysis::{normalize_full_query_term, FullError};
    for (input, byte_offset) in [("AAΣ", 2), ("ΣAA", 0), ("AΣΣ", 1), ("aΣ", 1)] {
        assert_eq!(
            analyze_full(input),
            Err(FullError::UnsupportedCaseContext { byte_offset }),
            "{input}"
        );
    }
    assert_eq!(normalize_full_query_term("AAΣ").unwrap(), "aaσ");
    assert_eq!(normalize_full_query_term("ΣAA").unwrap(), "σaa");
}
