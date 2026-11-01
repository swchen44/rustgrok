// SPDX-License-Identifier: MIT
// Independently authored diagnostic wrapper around the scoped CDDL tokenizer.
#[path = "../src/full_tokenizer.rs"]
mod full_tokenizer;
use tantivy::tokenizer::{TokenStream, Tokenizer};
fn main() {
    let input = std::env::args().nth(1).expect("input");
    full_tokenizer::validate(&input).expect("unsupported tokenizer input");
    let mut tokenizer = full_tokenizer::FullTokenizer;
    let mut stream = tokenizer.token_stream(&input);
    while stream.advance() {
        let t = stream.token();
        println!(
            "{} {}-{} {}",
            t.position, t.offset_from, t.offset_to, t.text
        );
    }
}
