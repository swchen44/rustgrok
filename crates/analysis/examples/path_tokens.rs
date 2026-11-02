// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_analysis::analyze_path;

fn main() {
    let Some(input) = std::env::args().nth(1) else {
        eprintln!("expected path argument");
        std::process::exit(2);
    };
    match analyze_path(&input) {
        Ok(tokens) => {
            for token in tokens {
                println!(
                    "{}\t{}\t{}\t{}",
                    token.position, token.start_utf16, token.end_utf16, token.term
                );
            }
        }
        Err(error) => {
            eprintln!("unsupported byte at offset {}", error.byte_offset);
            std::process::exit(2);
        }
    }
}
