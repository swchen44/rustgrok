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
 * printable-ASCII path tokenizer, not the complete upstream analyzer.
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
}

/// The first input byte outside the currently supported printable ASCII range.
#[derive(Debug, PartialEq, Eq)]
pub struct UnsupportedInput {
    pub byte_offset: usize,
}

/// Analyze a path using the evidenced printable-ASCII subset of the pinned
/// OpenGrok tokenizer. Unicode and ASCII controls are explicitly unsupported.
pub fn analyze_path(input: &str) -> Result<Vec<PathToken>, UnsupportedInput> {
    let bytes = input.as_bytes();
    if let Some(byte_offset) = bytes.iter().position(|byte| !(0x20..=0x7e).contains(byte)) {
        return Err(UnsupportedInput { byte_offset });
    }

    let mut cursor = 0;
    let mut offset = 0;
    let mut pending_dot = false;
    let mut tokens = Vec::new();
    loop {
        if pending_dot {
            tokens.push(PathToken {
                term: ".".into(),
                position: tokens.len(),
                start_utf16: offset,
                end_utf16: offset + 1,
            });
            offset += 1;
            pending_dot = false;
            continue;
        }
        while cursor < bytes.len() && bytes[cursor] == b'/' {
            cursor += 1;
        }
        if cursor == bytes.len() {
            break;
        }
        let mut term = String::new();
        term.push(char::from(bytes[cursor]).to_ascii_lowercase());
        cursor += 1;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            cursor += 1;
            if byte == b'.' {
                pending_dot = true;
                break;
            }
            if byte == b'/' || byte == b' ' {
                break;
            }
            term.push(char::from(byte).to_ascii_lowercase());
        }
        let length = term.len();
        tokens.push(PathToken {
            term,
            position: tokens.len(),
            start_utf16: offset,
            end_utf16: offset + length,
        });
        offset += length + 1;
    }
    Ok(tokens)
}
