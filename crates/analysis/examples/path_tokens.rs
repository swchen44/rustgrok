// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
use rustgrok_analysis::analyze_path;
use std::io::Read;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(argument) = args.next() else {
        eprintln!("expected path argument");
        std::process::exit(2);
    };
    let input = if argument == "--stdin" {
        let mut value = String::new();
        std::io::stdin()
            .read_to_string(&mut value)
            .expect("stdin read failed");
        value
    } else {
        argument
    };
    let spans = args.next().as_deref() == Some("--spans");
    match analyze_path(&input) {
        Ok(tokens) => {
            for token in tokens {
                if spans {
                    println!(
                        "{}\t{}\t{}\t{}\t{}\t{}",
                        token.position,
                        token.start_utf16,
                        token.end_utf16,
                        token.start_byte,
                        token.end_byte,
                        token.term
                    );
                } else {
                    println!(
                        "{}\t{}\t{}\t{}",
                        token.position, token.start_utf16, token.end_utf16, token.term
                    );
                }
            }
        }
        Err(error) => {
            eprintln!("unsupported byte at offset {}", error.byte_offset);
            std::process::exit(2);
        }
    }
}
