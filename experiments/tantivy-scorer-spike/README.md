# Tantivy scoring experiment

This independent, MIT-licensed G1 probe uses Tantivy **0.27.0** (MIT) and serde_json 1.0.140 pinned in Cargo.toml; exact registry dependencies and checksums are in Cargo.lock. This is an evaluation candidate, not a product engine selection or a port of OpenGrok. Each invocation creates an in-memory index for the selected projects; it does not reproduce the original analyzer, query semantics, ranking scope or complete HTTP response.

Build with `cargo build --locked --offline --release` when all locked dependencies are cached. Its input is the JSON output of the search-spike index command; invoke with `--documents FILE --field full|path|def|symbol --query TERM` and repeat `--project NAME` as necessary. The project-scoping and scoring test runs via `python3 -m unittest discover -s experiments/tantivy-scorer-spike/tests -v`.

The seven raw captures and comparison are in `tests/evidence/tantivy-scorer-2026-11-01`. Seven result counts and four ordered path lists matched a single set of original captures. The candidate supplies no original line numbers, snippets, tags, paging or HTTP response. This is neither search parity nor a performance comparison. The original rank also changed between some repeated captures. Full dependency licensing review and release SBOM remain future work.
