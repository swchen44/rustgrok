/*
 * CDDL HEADER START
 *
 * The contents of this file are subject to the terms of the
 * Common Development and Distribution License (the "License").
 * You may not use this file except in compliance with the License.
 *
 * See LICENSE.txt included in this distribution for the specific
 * language governing permissions and limitations under the License.
 *
 * When distributing Covered Code, include this CDDL HEADER in each
 * file and include the License file at LICENSE.txt.
 * If applicable, add the following below this CDDL HEADER, with the
 * fields enclosed by brackets "[]" replaced with your own identifying
 * information: Portions Copyright [yyyy] [name of copyright owner]
 *
 * CDDL HEADER END
 */
/*
 * SearchEngine.java: Copyright (c) 2005, 2026, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2018, Chris Fraire <cfraire@me.com>.
 * PathTokenizer.java: Copyright (c) 2005, 2025, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2018, Chris Fraire <cfraire@me.com>.
 * SearchController.java: Copyright (c) 2018, 2026, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2020, Chris Fraire <cfraire@me.com>.
 *
 * Adapted to Rust in 2026 for the restricted path-only no-context response.
 * Original: OpenGrok 1.14.20, commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3,
 * opengrok-indexer/src/main/java/org/opengrok/indexer/search/SearchEngine.java:440-510,
 * opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/PathTokenizer.java:50-131,
 * opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SearchController.java:122-134.
 * This module does not implement the general original API. See ../OPENGROK-NOTICE.txt.
 */
// SPDX-License-Identifier: CDDL-1.0
use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;
use serde_json::Value;

struct OrderedPaths<'a>(&'a [Value]);
impl Serialize for OrderedPaths<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for value in self.0 {
            let path = value["path"]
                .as_str()
                .ok_or_else(|| serde::ser::Error::custom("missing path"))?;
            let hit = serde_json::json!([{"line":"...","lineNumber":"","tag":null}]);
            map.serialize_entry(path, &hit)?;
        }
        map.end()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response<'a> {
    time: u128,
    result_count: u64,
    results: OrderedPaths<'a>,
    start_document: usize,
    end_document: usize,
}

pub fn render(raw: &str, start: usize, max: usize, elapsed_ms: u128) -> Result<String, String> {
    let data: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let total = data["resultCount"].as_u64().ok_or("missing result count")?;
    let results = data["results"].as_array().ok_or("missing results")?;
    let end = start.saturating_add(max).min(results.len());
    let page = if start < end {
        &results[start..end]
    } else {
        &results[0..0]
    };
    let end_document = if page.is_empty() {
        start
    } else {
        start + page.len() - 1
    };
    serde_json::to_string(&Response {
        time: elapsed_ms,
        result_count: total,
        results: OrderedPaths(page),
        start_document: start,
        end_document,
    })
    .map_err(|e| e.to_string())
}

// Port of the path token boundaries used by the pinned upstream analyzer.
// Query parsing is a restricted spike and does not implement Lucene syntax.
fn path_terms(value: &str) -> Vec<String> {
    let mut chars = value.chars().peekable();
    let mut output = Vec::new();
    while let Some(mut current) = chars.next() {
        while current == '/' {
            match chars.next() {
                Some(next) => current = next,
                None => return output,
            }
        }
        let mut term = String::new();
        term.push(current);
        loop {
            match chars.next() {
                Some('.') => {
                    output.push(term.to_lowercase());
                    output.push(".".into());
                    break;
                }
                Some('/') => {
                    output.push(term.to_lowercase());
                    break;
                }
                Some(c) if c.is_whitespace() => {
                    output.push(term.to_lowercase());
                    break;
                }
                Some(c) => term.push(c),
                None => {
                    output.push(term.to_lowercase());
                    return output;
                }
            }
        }
    }
    output
}

pub fn path_matches(path: &str, query: &str) -> bool {
    let terms = path_terms(path);
    let query_terms = path_terms(query);
    !query_terms.is_empty() && query_terms.iter().all(|term| terms.contains(term))
}
