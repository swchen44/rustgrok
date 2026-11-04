// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_analysis::analyze_path;

#[test]
fn public_api_accepts_verified_unicode_source_paths_in_the_synthetic_corpus() {
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
    assert!(
        rejected.is_empty(),
        "previously unsupported Unicode paths are now accepted"
    );
    let crab = analyze_path("/alpha/🦀.c").unwrap();
    assert_eq!(crab[1].term, "🦀");
    assert_eq!(&"/alpha/🦀.c"[crab[1].start_byte..crab[1].end_byte], "🦀");
}
