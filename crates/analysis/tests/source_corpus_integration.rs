// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_analysis::analyze_path;

#[test]
fn public_api_reports_every_unsupported_source_path_in_the_synthetic_corpus() {
    let corpus =
        include_str!("../../../tests/fixtures/upstream-integration-2026-11-03/source-paths.tsv");
    let query = analyze_path("main").unwrap();
    let mut matches = Vec::new();
    let mut rejected = Vec::new();
    for line in corpus.lines().filter(|line| !line.starts_with('#')) {
        let mut fields = line.splitn(3, '\t');
        let language = fields.next().unwrap();
        let path = fields.next().unwrap();
        let source = fields.next().unwrap();
        assert!(!source.is_empty(), "fixture source must be present");
        match analyze_path(path) {
            Ok(tokens) => {
                if query
                    .iter()
                    .all(|q| tokens.iter().any(|token| token.term == q.term))
                {
                    matches.push((language, path));
                }
            }
            Err(error) => rejected.push((language, path, error.byte_offset)),
        }
    }
    assert_eq!(
        matches,
        vec![
            ("c", "/alpha/lib/main.c"),
            ("python", "/beta/tools/main.py")
        ]
    );
    assert_eq!(
        rejected,
        vec![("c", "/alpha/🦀.c", 7), ("javascript", "/beta/İ.js", 6)]
    );
}
