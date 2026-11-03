# Restricted path query implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans, sequentially. Checkboxes track the planned behavior, not product acceptance.

**Goal:** Make independently verified path Boolean queries executable against published single-project postings.

**Architecture:** Capture pinned original QueryBuilder parse output and original HTTP hit sets over a public synthetic corpus before defining an accepted subset. A bounded engine-neutral representation evaluates immutable generation postings; explicit CLI and typed local host commands expose it while preserving the existing one-term command. Unsupported or uncertain syntax returns an error, and results remain unranked.

**Tech Stack:** Rust, existing analysis and path-index crates, existing macOS CLI and Unix socket IPC; original OpenGrok 1.14.20 and Lucene 9.12.3 run only as correctness oracle.

**Spec:** `docs/12-rust-compatibility.md` §2, `docs/02-requirements.md`, `docs/08-decision-register.md`, issue #6, existing host and index crate contracts.

## Global Constraints

- No benchmark, profiling, simultaneous tasks, or JVM product dependency. Use only independently authored public synthetic source.
- Do not infer grammar or precedence from conventional Boolean logic: original query parser and HTTP observations determine accepted cases.
- Preserve Q8 published snapshots, cancellation and typed bounded IPC; query does not mutate generation.
- No silent truncation, assumed ranking, or claim of full parser, multi-project, HTTP or product compatibility. Unsupported syntax fails closed.
- Retain source provenance, licensing, raw original parse and response bytes and precise oracle configuration.

## Review Focus

- Nested groups and mixed AND/OR: compare raw parse and complete original candidate sets, then test only the proven subset.
- Unary and pure negative query: reproduce original result and ensure explicit error if not faithfully implemented.
- Reserved words, lowercase variants and multicharacter tokens: reject ambiguous or unsupported analyzer results; never treat syntax as literal.
- Deep, oversized or invalid expressions: bounded parser rejects them without partial results.
- Existing clients and generation lifecycle: explicit new commands preserve existing lookup semantics, host typed IPC, and old snapshot behavior.

### Task 1: Capture independent oracle behavior

**Files:** Add `tests/evidence/path-query-2026-11-03/` and an independently authored correctness capture helper under `tools/`.

**Interfaces:** Input is a fixed query case list and isolated synthetic source tree; output is exact QueryBuilder parse output, raw HTTP responses with statuses, configuration, version and SHA-256 provenance. Capture pure-negative, mixed/grouped Boolean and path case-normalization variants. Reject missing evidence, extra/capped results or incomplete indexing. Record source-derived CDDL and dependency notices.

- [x] Inspect pinned `QueryBuilder`, `CustomQueryParser`, `PathTokenizer`, `SearchEngineTest` and path escaper; record paths and hashes.
- [x] Capture at least `alpha`, `ALPHA`, `alpha beta`, `alpha AND beta`, `alpha OR beta`, `alpha NOT beta`, `NOT alpha`, `alpha OR beta AND gamma`, `alpha AND (beta OR gamma)` and unary `+`/`-`; retain raw parse and HTTP bodies.
- [x] Compare all original hits with the fixture paths and record any ambiguous or unsupported case as blocked.

### Task 2: TDD bounded query and immutable postings

**Files:** Modify `crates/path-index/src/lib.rs`, add focused parser module and public API integration tests under `crates/path-index/tests/`.

**Interfaces:** `PathSnapshot::lookup_expression(&str) -> Result<HashSet<String>, IndexError>`; AST and parser use only the verified subset. Snapshot remains immutable and returns complete unranked candidates.

- [x] Write one public-API integration test with original captured expected hits; run it red, implement only the corresponding verified AST/evaluation behavior, then run green.
- [x] Repeat for nested/negative semantics only if independently verified. Test token, size and depth limits, invalid syntax, and unsupported wildcard, field, fuzzy, regex, range and multi-token analysis errors. Existing `lookup` remains unchanged.
- [x] Verify published old snapshot remains consistent after successful and failed replacement.

### Task 3: Explicit CLI and local IPC adapter

**Files:** Modify `apps/grok-host/src/main.rs`, `apps/grok-host/src/host.rs`, CLI and host subprocess tests, and `apps/grok-host/README.md`.

**Interfaces:** `query-expression STORE SOURCE PROJECT EXPRESSION`; `host-query-expression CONTROL EXPRESSION`; typed IPC request distinguishes expression from existing `query`. Both return generation and explicitly unspecified order, or a bounded explicit error.

- [x] Add real-process red tests for direct and host clients using captured expected paths, unsupported syntax and old generation while indexing; add implementation and rerun.
- [x] Keep frame/response caps and reject oversized complete results instead of truncation. Re-run typed duplicate-field and normal host lifecycle tests.
- [x] Update test mapping, provenance and evidence; run format, offline targeted and existing tests, staged package validation and `git diff --check`; merge current integration tip and commit without pushing.
