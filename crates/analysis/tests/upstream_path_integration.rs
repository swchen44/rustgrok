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
 * Copyright (c) 2013, 2021, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2026, swchen44.
 * Adapted test scenario: OpenGrok PathTokenizerTest.testIncrementToken,
 * commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3.
 * The complete reference stream is independently captured from the original.
 */
// SPDX-License-Identifier: CDDL-1.0
use rustgrok_analysis::analyze_path;

fn base64_ascii(input: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in input.as_bytes().chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        result.push(TABLE[(a >> 2) as usize] as char);
        result.push(TABLE[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        result.push(if chunk.len() > 1 {
            TABLE[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            TABLE[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    result
}

#[test]
fn public_api_matches_the_original_full_path_token_stream() {
    let original = include_str!(
        "../../../tests/evidence/upstream-integration-2026-11-03/path-tokenizer.stdout"
    );
    let actual = analyze_path("alpha/beta/gamma/delta.ext").unwrap();
    let lines: Vec<String> = actual
        .iter()
        .map(|token| {
            format!(
                "{}\t{}\t{}\t{}",
                token.position,
                token.start_utf16,
                token.end_utf16,
                base64_ascii(&token.term)
            )
        })
        .collect();
    let original_tokens: Vec<&str> = original
        .lines()
        .filter(|line| !line.starts_with("END\t"))
        .collect();
    assert_eq!(lines, original_tokens);
    assert_eq!(
        actual.last().unwrap().term,
        "ext",
        "the original test stops at the dot; this assertion uses the independently captured stream"
    );
}
