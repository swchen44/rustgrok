# Rustgrok analysis

`analyze_path` is an engine-independent path-token analyzer for printable ASCII only. The API returns the normalized term, zero-based position and UTF-16 offsets reported by the pinned OpenGrok tokenizer. These offsets reproduce upstream arithmetic; they are not necessarily byte ranges into the source. Unicode, controls and DEL return `UnsupportedInput` with the first unsupported UTF-8 byte offset, rather than silently returning approximate output.

This is a partial product component, not a complete search service or a claim of OpenGrok compatibility. Indexing, query grammar, cross-project order, generation publication and the rest of the language analyzers remain unimplemented. An unsupported path must be surfaced to callers; this module does not license silently omitting a document. Original raw token observations and their limits are in [the evidence](../../tests/evidence/path-analyzer-2026-11-03/README.md).

`src/lib.rs` adapts the CDDL-1.0 `PathTokenizer.java` from OpenGrok 1.14.20, commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`, retaining upstream copyright, header, [license](LICENSE.txt) and [notice](OPENGROK-NOTICE.txt). Independently authored Rust tests and the diagnostic example are MIT with the [scoped MIT text](RUST-LICENSE-MIT.txt). OpenGrok and Lucene are used only by the independent test oracle, not as Rust runtime dependencies.

```sh
cargo test --locked --offline --manifest-path crates/analysis/Cargo.toml
python3 -m unittest discover -s crates/analysis/tests -p test_path_oracle.py -v
```
