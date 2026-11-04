/*
 * CDDL HEADER START
 * The contents of this file are subject to the terms of the Common
 * Development and Distribution License (the "License"). You may not use
 * this file except in compliance with the License. See LICENSE.txt.
 * When distributing Covered Code, include this CDDL HEADER and LICENSE.txt.
 * CDDL HEADER END
 */
/*
 * Copyright (c) 2005, 2021, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2017-2018, Chris Fraire <cfraire@me.com>.
 * JFlexTokenizer.java: Copyright (c) 2009, 2025, Oracle and/or its affiliates.
 * Portions Copyright (c) 2017, 2020, Chris Fraire <cfraire@me.com>.
 * JFlexSymbolMatcher.java: Copyright (c) 2017, 2020, Chris Fraire <cfraire@me.com>.
 * CompatibleAnalyser.java: Copyright (c) 2005, 2018, Oracle and/or its affiliates.
 * Portions Copyright (c) 2026, swchen44.
 * Adapted from OpenGrok 1.14.20 commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3,
 * PlainFullTokenizer.lex, JFlexTokenizer.java and JFlexSymbolMatcher.java.
 * This restricted decoded-text API is not the complete upstream analyzer.
 */
// SPDX-License-Identifier: CDDL-1.0

/// A FULL token; UTF-16 offsets follow the original analyzer while byte spans
/// point to the original decoded UTF-8 input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullToken {
    pub term: String,
    pub position: usize,
    pub start_utf16: usize,
    pub end_utf16: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Rejection covers the complete input; no partial token stream is returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FullError {
    UnsupportedCharacter { byte_offset: usize },
    UnsupportedCaseContext { byte_offset: usize },
    UnsupportedLongUnicode { byte_offset: usize },
    TermTooLongForIndex { byte_offset: usize },
    InputLimit,
    TokenLimit,
    InvalidQueryTerm,
}

const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_TOKENS: usize = 1_000_000;
const MAX_TOKEN_UTF16: usize = 32_766;

fn supported(c: char) -> bool {
    c.is_ascii()
        || matches!(
            c,
            '中' | '文'
                | '繁'
                | '體'
                | '😀'
                | '🦀'
                | 'İ'
                | 'Α'
                | 'α'
                | 'Ο'
                | 'ο'
                | 'Σ'
                | 'σ'
                | 'ς'
        )
}
fn letter(c: char) -> bool {
    c.is_ascii_alphabetic()
        || matches!(
            c,
            '中' | '文' | '繁' | '體' | 'İ' | 'Α' | 'α' | 'Ο' | 'ο' | 'Σ' | 'σ' | 'ς'
        )
}
fn sigma_context_letter(c: char) -> bool {
    matches!(c, 'A' | 'İ' | 'Α' | 'Ο' | 'α' | 'ο' | 'Σ' | 'σ' | 'ς')
}
fn lower_token(raw: &str, start: usize) -> Result<(String, usize), FullError> {
    let count = raw.chars().count();
    if let Some((offset, sigma)) = raw.char_indices().find(|(_, c)| *c == 'Σ') {
        if raw[..offset].chars().count() > 1
            || raw[offset + sigma.len_utf8()..].chars().count() > 1
            || raw.chars().filter(|c| *c == 'Σ').count() != 1
            || raw.chars().any(|c| !sigma_context_letter(c))
        {
            return Err(FullError::UnsupportedCaseContext {
                byte_offset: start + offset,
            });
        }
    }
    let mut term = String::new();
    let mut total_units = 0usize;
    for (index, c) in raw.chars().enumerate() {
        let mut ascii = [0u8; 4];
        let lowered = match c {
            'İ' => "i\u{307}",
            'Α' => "α",
            'Ο' => "ο",
            'Σ' if index > 0 && index + 1 == count => "ς",
            'Σ' => "σ",
            c if c.is_ascii() => c.to_ascii_lowercase().encode_utf8(&mut ascii),
            c => c.encode_utf8(&mut ascii),
        };
        for value in lowered.chars() {
            let width = value.len_utf16();
            if total_units < MAX_TOKEN_UTF16 {
                if total_units + width > MAX_TOKEN_UTF16 {
                    return Err(FullError::UnsupportedCharacter { byte_offset: start });
                }
                term.push(value);
            }
            total_units += width;
        }
    }
    if !raw.is_ascii() && total_units > MAX_TOKEN_UTF16 {
        return Err(FullError::UnsupportedLongUnicode { byte_offset: start });
    }
    Ok((term, total_units))
}

/// Analyze already-decoded text in the supported FULL subset. The caller is
/// responsible for upstream reader decoding and any project tab expansion.
pub fn analyze_full(input: &str) -> Result<Vec<FullToken>, FullError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(FullError::InputLimit);
    }
    if let Some((byte_offset, _)) = input.char_indices().find(|(_, c)| !supported(*c)) {
        return Err(FullError::UnsupportedCharacter { byte_offset });
    }
    let bytes = input.as_bytes();
    let mut at = 0;
    let mut utf16 = 0usize;
    let mut result = Vec::new();
    while at < bytes.len() {
        let start = at;
        let start_utf16 = utf16;
        let c = input[at..].chars().next().expect("valid UTF-8 boundary");
        if letter(c) || c == '_' {
            at += c.len_utf8();
            utf16 += c.len_utf16();
            while at < bytes.len() {
                let next = input[at..].chars().next().expect("valid UTF-8 boundary");
                if !(letter(next) || next.is_ascii_digit() || next == '_') {
                    break;
                }
                at += next.len_utf8();
                utf16 += next.len_utf16();
            }
        } else if c.is_ascii_digit() {
            if c == '0'
                && at + 5 < bytes.len()
                && bytes[at + 1] == b'['
                && bytes[at + 2].eq_ignore_ascii_case(&b'x')
                && bytes[at + 3].eq_ignore_ascii_case(&b'x')
                && bytes[at + 4] == b']'
                && bytes[at + 5].is_ascii_hexdigit()
            {
                at += 6;
                while at < bytes.len() && bytes[at].is_ascii_hexdigit() {
                    at += 1;
                }
            } else {
                at += 1;
                while at < bytes.len() && bytes[at].is_ascii_digit() {
                    at += 1;
                }
                if at + 1 < bytes.len() && bytes[at] == b'.' && bytes[at + 1].is_ascii_digit() {
                    at += 2;
                    while at < bytes.len() && bytes[at].is_ascii_digit() {
                        at += 1;
                    }
                }
            }
            utf16 += at - start;
        } else if c.is_ascii() && b"@$%^&-+=?.:![]{}*/|#<>(),;~".contains(&(c as u8)) {
            at += 1;
            utf16 += 1;
        } else {
            at += c.len_utf8();
            utf16 += c.len_utf16();
            continue;
        }
        if result.len() >= MAX_TOKENS {
            return Err(FullError::TokenLimit);
        }
        let (term, normalized_units) = lower_token(&input[start..at], start)?;
        result.push(FullToken {
            term,
            position: result.len(),
            start_utf16,
            end_utf16: start_utf16 + normalized_units,
            start_byte: start,
            end_byte: at,
        });
    }
    Ok(result)
}

/// Check the UTF-8 byte length of already-tokenized terms against the pinned
/// Lucene term-byte limit. This alone does not establish successful indexing.
pub fn validate_full_index_terms(tokens: &[FullToken]) -> Result<(), FullError> {
    for token in tokens {
        if token.term.len() > MAX_TOKEN_UTF16 {
            return Err(FullError::TermTooLongForIndex {
                byte_offset: token.start_byte,
            });
        }
    }
    Ok(())
}

/// Normalize one restricted identifier-like FULL query term using the
/// separately observed Lucene lowercase filter behavior. This does not parse a
/// query or tokenize a whole string.
pub fn normalize_full_query_term(input: &str) -> Result<String, FullError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(FullError::InputLimit);
    }
    if input.is_empty() {
        return Err(FullError::InvalidQueryTerm);
    }
    let mut result = String::new();
    for (offset, c) in input.char_indices() {
        if !supported(c) {
            return Err(FullError::UnsupportedCharacter {
                byte_offset: offset,
            });
        }
        if !(letter(c) || c.is_ascii_digit() || c == '_') {
            return Err(FullError::InvalidQueryTerm);
        }
        match c {
            'İ' => result.push('i'),
            'Α' => result.push('α'),
            'Ο' => result.push('ο'),
            'Σ' => result.push('σ'),
            c if c.is_ascii() => result.push(c.to_ascii_lowercase()),
            c => result.push(c),
        }
        if result.len() > MAX_TOKEN_UTF16 {
            return Err(FullError::TermTooLongForIndex { byte_offset: 0 });
        }
    }
    Ok(result)
}
