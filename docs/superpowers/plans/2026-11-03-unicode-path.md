# Unicode path analyzer implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans and test-driven development. Steps use checkboxes.

**Goal:** Extend the production path analyzer with a deliberately bounded set of Unicode characters proven against the pinned original, including Traditional Chinese, supplementary-plane characters and verified case mapping.

**Architecture:** Keep one engine-independent public path analyzer. Expose upstream-reported UTF-16 offsets separately from true UTF-8 source byte spans. Reproduce the pinned tokenizer's delimiter, normalization and offset arithmetic; unverified characters or case-mapping contexts return an explicit error before any token list is returned. The entire Unicode repertoire and other analyzers remain incomplete.

**Tech Stack:** Rust standard library; independently executed OpenGrok 1.14.20 Java oracle for correctness only, not a product dependency.

**Spec:** `docs/02-requirements.md` FR-001/002/008/021/022, `docs/03-system-design.md` analyzer coordinates, `docs/04-implementation-plan.md` WP-B02/C08; #6, RST-003 and SRC-001.

## Constraints and review focus

- Preserve CDDL-1.0 headers, original copyright, LICENSE and NOTICE on all upstream-derived code; independent Rust tests remain MIT.
- Reference commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`, original raw output and hashes. Do not normalize away original offset quirks.
- Cover repeated separators, `.`, spaces, Traditional Chinese, non-BMP input, `İ` expansion and contextual sigma with real oracle streams; verify UTF-8 slicing of every token source span independently.
- Test non-ASCII whitespace, unverified letters/combining marks and mixed case contexts as explicit unsupported errors. Never drop a document or return partial output on unsupported input.
- Retain existing ASCII integration and public-corpus cases; adapt Unicode rejection assertions only for explicitly supported cases. Product indexer, generation, query grammar, UI and complete Unicode parity remain unimplemented.
- Sequential work only; no performance measurements or added JVM product dependency.

## Task: original observations, failing test, implementation and verification

**Files:** modify `crates/analysis/src/lib.rs`, relevant tests and example, crate README, source evidence/fixtures and planning manifest. Add original raw observations under `tests/evidence/unicode-path-2026-11-03/` and an external Rust integration test; preserve original raw UTF-8/base64 streams and all upstream/JDK/jar provenance.

- [x] Capture fixed Chinese, emoji, case expansion, sigma context, adjacent separators and original offset behavior, plus an exhaustive supported CJK-range case. Save input bytes, original stdout/stderr, exit codes and hashes. Compare only actual observed output.
- [x] Write failing external Rust API tests against saved original streams for term/position/UTF-16 offsets. Assert source byte spans select the actual original substrings, including mapped/expanded terms. Assert unsupported contexts and controls fail at the first unsupported byte, without partial results. Update old corpus integration expectations for supported Unicode.
- [x] Run the tests and capture red; then implement minimal UTF-8 iteration, bounded character support, context-correct normalization for the verified subset, separately maintained source byte ranges and original UTF-16 offset arithmetic.
- [x] Run crate tests, oracle/provenance contract tests, formatting and clippy. Label finite observations and unsupported cases; do not mark full RST-003 or product tests passed.
- [ ] Stage and build the planning package with its validated ZIP evidence; check manifest, diff and integration tip, commit and request independent review.
