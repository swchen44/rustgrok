// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_analysis::analyze_path;
use std::io::{self, BufRead};

fn main() {
    for line in io::stdin().lock().lines() {
        let input = line.expect("stdin read failed");
        let tokens = analyze_path(&input).expect("input outside supported sigma context");
        assert_eq!(tokens.len(), 1);
        let token = &tokens[0];
        println!(
            "{}\t{}\t{}\t{}\t{}",
            input, token.position, token.start_utf16, token.end_utf16, token.term
        );
    }
}
