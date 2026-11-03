// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_path_index::{CancellationToken, IndexLimits, PathIndex};
use std::collections::HashSet;

fn set(paths: &[&str]) -> HashSet<String> {
    paths.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn oracle_implicit_and_intersects_published_path_postings() {
    let db = PathIndex::new(
        "demo",
        IndexLimits {
            max_documents: 10,
            max_path_bytes: 256,
            max_total_path_bytes: 2048,
            max_live_generations: 3,
        },
    )
    .unwrap();
    let paths = [
        "/demo/alpha/beta/one.c",
        "/demo/alpha/gamma/two.c",
        "/demo/beta/gamma/three.c",
    ];
    db.replace(
        None,
        paths.len(),
        paths.iter().map(|p| Ok((*p).to_owned())),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(
        db.snapshot()
            .unwrap()
            .lookup_expression("alpha beta")
            .unwrap(),
        set(&["/demo/alpha/beta/one.c"])
    );
}

#[test]
fn oracle_boolean_subset_and_rejected_syntax() {
    let db = PathIndex::new(
        "demo",
        IndexLimits {
            max_documents: 10,
            max_path_bytes: 256,
            max_total_path_bytes: 2048,
            max_live_generations: 3,
        },
    )
    .unwrap();
    let paths = [
        "/demo/alpha/beta/one.c",
        "/demo/alpha/gamma/two.c",
        "/demo/beta/gamma/three.c",
        "/demo/alpha/alone/four.c",
        "/demo/other/delta/five.c",
    ];
    db.replace(
        None,
        paths.len(),
        paths.iter().map(|p| Ok((*p).to_owned())),
        &CancellationToken::new(),
    )
    .unwrap();
    let view = db.snapshot().unwrap();
    for (query, expected) in [
        ("ALPHA", vec![paths[0], paths[1], paths[3]]),
        ("alpha AND beta", vec![paths[0]]),
        (
            "alpha OR beta",
            vec![paths[0], paths[1], paths[2], paths[3]],
        ),
        ("alpha NOT beta", vec![paths[1], paths[3]]),
        ("alpha -beta", vec![paths[1], paths[3]]),
        ("NOT alpha", vec![]),
        ("-alpha", vec![]),
        ("+alpha", vec![paths[0], paths[1], paths[3]]),
        ("alpha AND (beta OR gamma)", vec![paths[0], paths[1]]),
        ("(alpha OR beta) AND gamma", vec![paths[1], paths[2]]),
        ("alpha and beta", vec![]),
    ] {
        assert_eq!(
            view.lookup_expression(query).unwrap(),
            set(&expected),
            "{query}"
        );
    }
    for query in [
        "",
        "alpha OR beta AND gamma",
        "alpha OR NOT beta",
        "alpha +beta",
        "alpha-beta",
        "alpha*",
        "alpha~",
        "alpha:beta",
        "/alpha/",
        "[alpha TO beta]",
        "alpha.beta",
        "absent bad*",
        "alpha OR",
        "(alpha",
        "alpha)",
        "α",
        "alpha\\ beta",
    ] {
        assert!(view.lookup_expression(query).is_err(), "{query}");
    }
    assert!(view.lookup_expression(&"a ".repeat(129)).is_err());
    assert!(view
        .lookup_expression(&format!("{}a{}", "(".repeat(17), ")".repeat(17)))
        .is_err());
}

#[test]
fn ast_preserves_lucene_clause_occurrences_for_future_scoring() {
    use rustgrok_path_index::{parse_path_expression, PathClause, PathOccur, PathQuery};
    use PathOccur::{Must, MustNot, Should};
    let term = |s: &str| PathQuery::Term(s.to_owned());
    assert_eq!(parse_path_expression("ALPHA", 256).unwrap(), term("alpha"));
    let clause = |occur, query| PathClause { occur, query };
    assert_eq!(
        parse_path_expression("alpha OR beta", 256).unwrap(),
        PathQuery::Boolean(vec![
            clause(Should, term("alpha")),
            clause(Should, term("beta"))
        ])
    );
    assert_eq!(
        parse_path_expression("alpha NOT beta", 256).unwrap(),
        PathQuery::Boolean(vec![
            clause(Must, term("alpha")),
            clause(MustNot, term("beta"))
        ])
    );
    assert_eq!(
        parse_path_expression("alpha NOT beta gamma", 256).unwrap(),
        PathQuery::Boolean(vec![
            clause(Must, term("alpha")),
            clause(MustNot, term("beta")),
            clause(Must, term("gamma"))
        ])
    );
    assert_eq!(
        parse_path_expression("NOT alpha", 256).unwrap(),
        PathQuery::Boolean(vec![clause(MustNot, term("alpha"))])
    );
    assert_eq!(
        parse_path_expression("+alpha", 256).unwrap(),
        PathQuery::Boolean(vec![clause(Must, term("alpha"))])
    );
}

#[test]
fn accepted_short_expressions_match_retained_original_http_candidates() {
    let db = PathIndex::new(
        "demo",
        IndexLimits {
            max_documents: 10,
            max_path_bytes: 256,
            max_total_path_bytes: 2048,
            max_live_generations: 3,
        },
    )
    .unwrap();
    let paths = [
        "/demo/alpha/beta/one.c",
        "/demo/alpha/gamma/two.c",
        "/demo/beta/gamma/three.c",
        "/demo/alpha/alone/four.c",
        "/demo/other/delta/five.c",
    ];
    db.replace(
        None,
        paths.len(),
        paths.iter().map(|p| Ok((*p).to_owned())),
        &CancellationToken::new(),
    )
    .unwrap();
    let view = db.snapshot().unwrap();
    let fixture = include_str!("../../../tests/evidence/path-query-2026-11-03/oracle-accepted.tsv");
    for line in fixture.lines() {
        let (query, expected) = line.split_once('\t').unwrap();
        let paths: Vec<_> = if expected.is_empty() {
            vec![]
        } else {
            expected.split('|').collect()
        };
        assert_eq!(
            view.lookup_expression(query).unwrap(),
            set(&paths),
            "{query}"
        );
    }
}

#[test]
fn control_bytes_are_rejected_instead_of_treated_as_separators() {
    use rustgrok_path_index::IndexError;
    let db = PathIndex::new(
        "demo",
        IndexLimits {
            max_documents: 10,
            max_path_bytes: 256,
            max_total_path_bytes: 2048,
            max_live_generations: 3,
        },
    )
    .unwrap();
    let paths = [
        "/demo/alpha/beta/one.c",
        "/demo/alpha/gamma/two.c",
        "/demo/beta/gamma/three.c",
    ];
    db.replace(
        None,
        paths.len(),
        paths.iter().map(|p| Ok((*p).to_owned())),
        &CancellationToken::new(),
    )
    .unwrap();
    let view = db.snapshot().unwrap();
    for operator in ["AND", "OR", "NOT"] {
        for control in std::iter::once(12u8)
            .chain(0u8..=31)
            .chain(std::iter::once(127))
        {
            let input = format!("alpha {operator}{}beta", control as char);
            assert!(
                matches!(
                    view.lookup_expression(&input),
                    Err(IndexError::UnsupportedQuery)
                ),
                "{input:?}"
            );
        }
    }
    assert_eq!(
        view.lookup_expression("alpha AND beta").unwrap(),
        set(&[paths[0]])
    );
    assert_eq!(
        view.lookup_expression("alpha OR beta").unwrap(),
        set(&paths)
    );
    assert_eq!(
        view.lookup_expression("alpha NOT beta").unwrap(),
        set(&[paths[1]])
    );
    assert_eq!(view.generation(), 1);
}
