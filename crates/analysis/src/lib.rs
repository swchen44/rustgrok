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
 * Copyright (c) 2005, 2025, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2018, Chris Fraire <cfraire@me.com>.
 * Portions Copyright (c) 2026, swchen44.
 *
 * Derived from OpenGrok 1.14.20 PathTokenizer.java at commit
 * 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3. This is the restricted
 * bounded Unicode path tokenizer, not the complete upstream analyzer.
 */
// SPDX-License-Identifier: CDDL-1.0

/// A normalized path term and its original tokenizer's UTF-16 offsets.
///
/// These offsets reproduce upstream arithmetic and can differ from the source
/// substring's byte range, including for repeated or leading separators.
#[derive(Debug, PartialEq, Eq)]
pub struct PathToken {
    pub term: String,
    pub position: usize,
    pub start_utf16: usize,
    pub end_utf16: usize,
    /// Start of the original source token as a UTF-8 byte boundary.
    pub start_byte: usize,
    /// Exclusive end of the original source token as a UTF-8 byte boundary.
    pub end_byte: usize,
}

/// An input character or casing context outside the verified subset.
#[derive(Debug, PartialEq, Eq)]
pub struct UnsupportedInput {
    pub byte_offset: usize,
}

fn supported(c: char) -> bool {
    matches!(c, ' '..='~' | '\u{4e00}'..='\u{9fff}' | '🦀' | '😀' | 'İ' | 'Α' | 'α' | 'Ο' | 'ο' | 'Σ' | 'σ' | 'ς')
}

fn sigma_context_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, 'İ' | 'Α' | 'α' | 'Ο' | 'ο' | 'Σ' | 'σ' | 'ς')
}

fn normalize(raw: &str, start_byte: usize) -> Result<String, UnsupportedInput> {
    let chars: Vec<(usize, char)> = raw.char_indices().collect();
    if let Some((sigma_offset, _)) = chars.iter().find(|(_, c)| *c == 'Σ') {
        if chars.len() > 3
            || chars.iter().filter(|(_, c)| *c == 'Σ').count() != 1
            || chars.iter().any(|(_, c)| !sigma_context_letter(*c))
        {
            return Err(UnsupportedInput {
                byte_offset: start_byte + sigma_offset,
            });
        }
    }
    let mut out = String::new();
    for (index, &(_, c)) in chars.iter().enumerate() {
        match c {
            'İ' => out.push_str("i\u{307}"),
            'Α' => out.push('α'),
            'Ο' => out.push('ο'),
            'Σ' if index > 0 && index + 1 == chars.len() => out.push('ς'),
            'Σ' => out.push('σ'),
            c if c.is_ascii() => out.push(c.to_ascii_lowercase()),
            c => out.push(c),
        }
    }
    Ok(out)
}

/// Analyze the verified path subset, rejecting unverified Unicode or casing
/// contexts without returning partial tokens. `*_utf16` values reproduce the
/// original tokenizer's arithmetic; `*_byte` values bound actual source bytes.
pub fn analyze_path(input: &str) -> Result<Vec<PathToken>, UnsupportedInput> {
    if let Some((byte_offset, _)) = input.char_indices().find(|(_, c)| !supported(*c)) {
        return Err(UnsupportedInput { byte_offset });
    }
    let bytes = input.as_bytes();
    let mut cursor = 0;
    let mut offset = 0;
    let mut pending_dot: Option<usize> = None;
    let mut tokens = Vec::new();
    loop {
        if let Some(dot_byte) = pending_dot.take() {
            tokens.push(PathToken {
                term: ".".into(),
                position: tokens.len(),
                start_utf16: offset,
                end_utf16: offset + 1,
                start_byte: dot_byte,
                end_byte: dot_byte + 1,
            });
            offset += 1;
            continue;
        }
        while cursor < bytes.len() && bytes[cursor] == b'/' {
            cursor += 1;
        }
        if cursor == bytes.len() {
            break;
        }
        let start_byte = cursor;
        let first = input[cursor..].chars().next().expect("cursor before end");
        cursor += first.len_utf8();
        let mut end_byte = cursor;
        while cursor < bytes.len() {
            let c = input[cursor..].chars().next().expect("cursor before end");
            let current = cursor;
            cursor += c.len_utf8();
            if c == '.' {
                pending_dot = Some(current);
                break;
            }
            if c == '/' || c == ' ' {
                break;
            }
            end_byte = cursor;
        }
        let term = normalize(&input[start_byte..end_byte], start_byte)?;
        let length = term.encode_utf16().count();
        tokens.push(PathToken {
            term,
            position: tokens.len(),
            start_utf16: offset,
            end_utf16: offset + length,
            start_byte,
            end_byte,
        });
        offset += length + 1;
    }
    Ok(tokens)
}

mod full;
pub use full::{
    analyze_full, normalize_full_query_term, validate_full_index_terms, FullError, FullToken,
};
