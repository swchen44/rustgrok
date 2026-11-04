# Rustgrok analysis

`analyze_path` is an engine-independent path-token analyzer for a deliberately bounded subset: printable ASCII; CJK Unified Ideographs U+4E00–U+9FFF; `🦀`, `😀`, `İ`, `Α`, `α`, `Ο`, `ο`, `Σ`, `σ`, and `ς`. For a token containing uppercase `Σ`, exactly one `Σ` with at most one supported cased letter on each side is accepted; punctuation, CJK, emoji, additional `Σ` or longer contexts in that token are rejected until validated. All other non-ASCII characters and controls are rejected. `UnsupportedInput.byte_offset` identifies an unsupported character or the `Σ` beginning an unsupported context. The function returns an error rather than partial tokens.

Each token provides its normalized term and zero-based position; `start_utf16` and `end_utf16` reproduce the pinned upstream tokenizer's offset arithmetic. `start_byte` and `end_byte` instead delimit the actual UTF-8 bytes of the source token. Leading/repeated slashes and lowercase expansion can make these two coordinate systems disagree; neither is silently substituted for the other. Raw original observations and the exact bounded validation are in [Unicode evidence](../../tests/evidence/unicode-path-2026-11-03/README.md).

This is a partial product component, not a complete search service or a claim of OpenGrok compatibility. Complete OpenGrok search compatibility, general Unicode handling and the remaining language analyzers are not established by this component. An unsupported path must be surfaced to callers; this module does not license silently omitting a document. Earlier original raw observations are in [the prior evidence](../../tests/evidence/path-analyzer-2026-11-03/README.md).

`src/lib.rs` adapts the CDDL-1.0 `PathTokenizer.java` from OpenGrok 1.14.20, commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`, retaining upstream copyright, header, [license](LICENSE.txt) and [notice](OPENGROK-NOTICE.txt). The adapted upstream integration test `tests/upstream_path_integration.rs` is CDDL-1.0 with its original copyright and header. Independently authored Rust tests and diagnostic examples are MIT with the [scoped MIT text](RUST-LICENSE-MIT.txt). OpenGrok and Lucene are used only by the independent test oracle, not as Rust runtime dependencies.

```sh
cargo test --locked --offline --manifest-path crates/analysis/Cargo.toml
python3 -m unittest discover -s crates/analysis/tests -p test_path_oracle.py -v
python3 -m unittest discover -s crates/analysis/tests -p test_unicode_path_oracle.py -v
```

## Restricted FULL analysis

`analyze_full` accepts already decoded, reader-prepared text and returns FULL terms, positions, upstream-style UTF-16 offsets and independently calculated actual UTF-8 byte spans. The Java end offset can overlap a following token after lowercase expansion; byte spans always refer to the input. The original stream's final `end()` offset is not exposed. Ordinary source whitespace, underscores and ASCII controls are accepted. Inputs are bounded to 8 MiB and one million tokens. An unsupported character, context or input returns an error rather than partial tokens.

Non-ASCII support is limited to `中`, `文`, `繁`, `體`, `😀`, `🦀`, `İ`, `Α`, `α`, `Ο`, `ο`, `Σ`, `σ`, and `ς`. Uppercase Σ is supported only once per token, with at most one preceding and one following character selected from `A`, `İ`, `Α`, `Ο`, `α`, `ο`, `σ`, `ς`: 81 finite contexts verified against the pinned original. Other contexts and Unicode terms exceeding 32766 UTF-16 units are rejected. ASCII long tokens reproduce the observed 32766-unit truncation. `validate_full_index_terms` separately rejects terms exceeding 32766 UTF-8 bytes; it is a necessary bounded check, not proof that an indexer will accept a document.

`normalize_full_query_term` is a separate, bounded identifier-like normalization function, not a query parser. For example, FULL tokenization gives `İ` → `i` plus combining dot and contextual sigma, while query normalization gives `İ` → `i` and `Σ` → `σ`. The 32766-byte query restriction is this API's explicit limit, not a claim about all Lucene queries.

The observed C, Python and JavaScript analyzers share this FULL tokenizer for the tested decoded inputs. The original file reader may do BOM/encoding handling or configured tab expansion before analysis; this API does not implement that reader pipeline. No FULL postings, source indexing, search, query grammar or ranking is provided by this slice. Original results, inputs, source hashes, test scope and licensing are in [FULL evidence](../../tests/evidence/full-analysis-2026-11-04/README.md). `src/full.rs` adapts CDDL-1.0 OpenGrok files with notices preserved; independent integration tests and fixtures are MIT under `RUST-LICENSE-MIT.txt`. Java is oracle-only and is not a product runtime dependency.
