// SPDX-License-Identifier: MIT
// Independently authored diagnostic wrapper for candidate norm decoding.
use tantivy::fieldnorm::FieldNormReader;
fn main() {
    for id in 0..=255u8 {
        println!("{} {}", id, FieldNormReader::id_to_fieldnorm(id));
    }
}
