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
 * Copyright (c) 2005, 2021, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2017-2018, Chris Fraire <cfraire@me.com>.
 *
 * SearchEngine.java: Copyright (c) 2005, 2026, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2018, Chris Fraire <cfraire@me.com>.
 * JFlexTokenizer.java: Copyright (c) 2009, 2025, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2017, 2020, Chris Fraire <cfraire@me.com>.
 *
 * Adapted to Rust/Tantivy in 2026 for a restricted G1 experiment.
 * Upstream OpenGrok 1.14.20, commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3,
 * opengrok-indexer/src/main/jflex/analysis/plain/PlainFullTokenizer.lex,
 * opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/JFlexTokenizer.java:135-146,
 * opengrok-indexer/src/main/java/org/opengrok/indexer/search/SearchEngine.java:218-226.
 * Unicode version, offset conventions, and long-term limits are not yet proven equivalent.
 */
// SPDX-License-Identifier: CDDL-1.0
use regex::Regex;
use std::sync::OnceLock;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[derive(Clone, Default)]
pub struct FullTokenizer;
pub struct FullStream {
    tokens: std::vec::IntoIter<Token>,
    current: Token,
}
impl Tokenizer for FullTokenizer {
    type TokenStream<'a> = FullStream;
    fn token_stream<'a>(&'a mut self, text: &'a str) -> FullStream {
        let tokens = pattern()
            .find_iter(text)
            .enumerate()
            .map(|(position, found)| Token {
                offset_from: found.start(),
                offset_to: found.end(),
                position,
                text: truncate_utf16(&found.as_str().to_lowercase(), 32_766),
                position_length: 1,
            })
            .collect::<Vec<_>>()
            .into_iter();
        FullStream {
            tokens,
            current: Token::default(),
        }
    }
}
impl TokenStream for FullStream {
    fn advance(&mut self) -> bool {
        if let Some(token) = self.tokens.next() {
            self.current = token;
            true
        } else {
            false
        }
    }
    fn token(&self) -> &Token {
        &self.current
    }
    fn token_mut(&mut self) -> &mut Token {
        &mut self.current
    }
}

// Rust strings cannot contain the isolated surrogate Java substring can create.
// Stop before a supplementary character that straddles the boundary.
fn truncate_utf16(value: &str, max_units: usize) -> String {
    let mut units = 0;
    value
        .chars()
        .take_while(|c| {
            let next = units + c.len_utf16();
            if next > max_units {
                false
            } else {
                units = next;
                true
            }
        })
        .collect()
}

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"[\p{L}_][\p{L}\p{N}_]*|0\[[xX][xX]\][0-9a-fA-F]+|[0-9]+\.[0-9]+|[0-9]+|[@$%^&+?\.:!\[\]{}*/|#<>()\-,;~=]").expect("static tokenizer expression"))
}

pub fn validate(value: &str) -> Result<(), &'static str> {
    for found in pattern().find_iter(value) {
        let mut units = 0;
        let lower = found.as_str().to_lowercase();
        for c in lower.chars() {
            if units < 32_766 && units + c.len_utf16() > 32_766 {
                return Err("unsupported UTF-16 surrogate split at token limit");
            }
            units += c.len_utf16();
            if units >= 32_766 {
                break;
            }
        }
        if truncate_utf16(&lower, 32_766).len() > 32_766 {
            return Err("unsupported token exceeding Lucene byte limit");
        }
    }
    Ok(())
}

// Java's TreeSet<String> orders projects by UTF-16 code units.
#[allow(dead_code)] // The token-only diagnostic includes this module too.
pub fn compare_projects(left: &str, right: &str) -> std::cmp::Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}
