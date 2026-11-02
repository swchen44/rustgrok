# Rustgrok analysis

`analyze_path` is an engine-independent path-token analyzer for a deliberately bounded subset: printable ASCII; CJK Unified Ideographs U+4E00–U+9FFF; `🦀`, `😀`, `İ`, `Α`, `α`, `Ο`, `ο`, `Σ`, `σ`, and `ς`. For a token containing uppercase `Σ`, exactly one `Σ` with at most one supported cased letter on each side is accepted; punctuation, CJK, emoji, additional `Σ` or longer contexts in that token are rejected until validated. All other non-ASCII characters and controls are rejected. `UnsupportedInput.byte_offset` identifies an unsupported character or the `Σ` beginning an unsupported context. The function returns an error rather than partial tokens.

Each token provides its normalized term and zero-based position; `start_utf16` and `end_utf16` reproduce the pinned upstream tokenizer's offset arithmetic. `start_byte` and `end_byte` instead delimit the actual UTF-8 bytes of the source token. Leading/repeated slashes and lowercase expansion can make these two coordinate systems disagree; neither is silently substituted for the other. Raw original observations and the exact bounded validation are in [Unicode evidence](../../tests/evidence/unicode-path-2026-11-03/README.md).

This is a partial product component, not a complete search service or a claim of OpenGrok compatibility. Indexing, query grammar, cross-project order, generation publication, general Unicode handling and the rest of the language analyzers remain unimplemented. An unsupported path must be surfaced to callers; this module does not license silently omitting a document. Earlier original raw observations are in [the prior evidence](../../tests/evidence/path-analyzer-2026-11-03/README.md).

`src/lib.rs` adapts the CDDL-1.0 `PathTokenizer.java` from OpenGrok 1.14.20, commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`, retaining upstream copyright, header, [license](LICENSE.txt) and [notice](OPENGROK-NOTICE.txt). The adapted upstream integration test `tests/upstream_path_integration.rs` is CDDL-1.0 with its original copyright and header. Independently authored Rust tests and diagnostic examples are MIT with the [scoped MIT text](RUST-LICENSE-MIT.txt). OpenGrok and Lucene are used only by the independent test oracle, not as Rust runtime dependencies.

```sh
cargo test --locked --offline --manifest-path crates/analysis/Cargo.toml
python3 -m unittest discover -s crates/analysis/tests -p test_path_oracle.py -v
python3 -m unittest discover -s crates/analysis/tests -p test_unicode_path_oracle.py -v
```
