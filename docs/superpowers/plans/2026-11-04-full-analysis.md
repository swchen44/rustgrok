# Restricted FULL analyzer implementation plan

> **Execution:** Use the existing TDD workflow sequentially; no benchmark or product JVM dependency.

**Goal:** Provide an engine-neutral public FULL token stream and distinct query-term normalization for the independently verified C/Python/JavaScript subset.

**Architecture:** The API accepts already decoded reader text with no implicit BOM removal or project-specific tab expansion; it exposes normalized term, position, upstream UTF-16 offsets and actual source UTF-8 byte span separately. A bounded lexer implements the verified original rules; unsupported Unicode, context or index term limits return a whole-input error, never partial tokens. Query-term normalization is a separate operation from stream tokenization. Existing path and storage APIs stay unchanged.

**Spec:** FR-001/002/008, WP-C08/C09, `docs/12-rust-compatibility.md`, pinned OpenGrok 1.14.20 commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3` and Lucene 9.12.3.

## Constraints

- Original `PlainAnalyzer` adds FULL from a decoded reader and can wrap `ExpandTabsReader` for custom project tab settings. No source encoding, BOM handling, custom tab mapping, source indexing, path rewriting or reading snapshot is introduced by this decoded-text API.
- Do not guess Java `Locale.ROOT` full lowercase, JFlex Unicode classification, Lucene per-term `LowerCaseFilter`, supplementary/surrogate splits or 32,766-byte limits. Accept only bounded cases backed by original raw observations; reject uncertain cases explicitly.
- Capture Java original tokenizer and per-term normalization via actual C/Python/JS analyzers and `CompatibleAnalyser`; bind verified binaries, release libraries, source files, fixtures and raw output hashes. Preserve OpenGrok CDDL adaptation, copyright and notices; tests independently authored in Rust are MIT.
- No inference that a process exit 0 proves every source file indexed; retain prior long-term per-file failure evidence. No claim of complete languages, Unicode, ranking, full-text search service or performance.

## Tasks

1. Capture original raw token streams for three public files, ASCII/Unicode/casing/long-term synthetic cases and independent query normalization, with exact inputs, version, hashes and status; preserve raw outputs separately from projections. Verify language factory inheritance and configured reader branch.
2. Add a public-API integration tracer from the captured original data, watch it fail, and implement a bounded stream. Add incremental red/green cases for numeric/printable tokens, no silent Unicode omission, Greek/İ casing, UTF-16-vs-byte spans, truncation and unindexable term errors.
3. Add independently verified single-term query normalization separately from tokenization. Test unsupported input and source bytes unchanged. Record default no-custom-tab and BOM/encoding scope.
4. Run offline native correctness suites, formatting, Clippy and evidence contracts serially with a 2.5 GiB sampled free-space guard; preserve actual test executable bytes by SHA and publish package/manifest integrity separately from product acceptance. Merge integration tip, commit and hand off for independent review; do not push/merge.
