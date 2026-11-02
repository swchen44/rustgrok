# Path analyzer implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox syntax.

**Goal:** Add an engine-independent, production Rust path-token analyzer for the evidenced printable-ASCII subset of OpenGrok 1.14.20; explicitly reject unsupported inputs and preserve offset behavior.

**Architecture:** An `analysis` crate exposes `analyze_path(&str) -> Result<Vec<PathToken>, UnsupportedInput>`. A token carries the normalized term, zero-based token position and upstream UTF-16 start/end offsets. Since upstream offsets can differ from the source span, do not label them UTF-8 byte offsets. Inputs outside printable ASCII are rejected until exact Java Unicode and whitespace behavior is implemented. Upstream's exact offset quirks and no whole-product compatibility claim are retained.

**Tech Stack:** Rust standard library; independent Java correctness oracle using pinned OpenGrok release and its Lucene jars; no Java in product runtime.

**Spec:** `docs/02-requirements.md` FR-001/002/008/021/022; `docs/03-system-design.md` sections 3–4; `docs/04-implementation-plan.md` WP-A08/B01/B02 and future C08; GitHub issue #6; `tests/matrix.json` RST-002/003 and SRC-001.

## Global constraints

- Reference OpenGrok 1.14.20, commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`; source attribution, CDDL-1.0 and notices accompany derived code.
- No performance measurements; no feature-completion or G1/G2 acceptance claim. The full analyzer, query grammar, indexing, order and generation remain incomplete.
- Rust, source rebuild and Q8/Q9 requirements remain; Ctags/SCM helpers can be external and must eventually be bundled under D19.
- Sequential execution; do not touch the separate issue7 worktree's uncommitted files.

## Review focus

- Leading and repeated separators preserve the original unusual offsets, not inferred source offsets.
- A dot at the start, end, or after a dot emits the exact original token sequence.
- Empty input, whitespace and end-of-stream behavior are explicitly checked against the oracle.
- Non-ASCII UTF-8, supplementary characters and control characters return explicit unsupported errors rather than approximations.
- Very long printable ASCII tokens preserve full term and offset without a 40-character truncation.

### Task 1: Capture oracle and verify the analyzer

**Files:** create `crates/analysis/Cargo.toml`, `crates/analysis/src/lib.rs`, `crates/analysis/tests/path.rs`, `crates/analysis/tests/test_path_oracle.py`, `crates/analysis/examples/path_tokens.rs`, `crates/analysis/LICENSE.txt`, `crates/analysis/OPENGROK-NOTICE.txt`, `crates/analysis/RUST-LICENSE-MIT.txt`, and public oracle fixture/evidence under `tests/evidence/path-analyzer-2026-11-03/`. Update `crates/README.md` and `planning-manifest.json` when code is ready.

**Interfaces:** `pub struct PathToken { pub term: String, pub position: usize, pub start_utf16: usize, pub end_utf16: usize }`; `pub struct UnsupportedInput { pub byte_offset: usize }`; `pub fn analyze_path(input: &str) -> Result<Vec<PathToken>, UnsupportedInput>`.

- [x] Capture original tokens and offsets for a fixed public printable-ASCII corpus with independent `PathTokenizer` oracle; save source, commands, pinned source/release identities and raw stdout. Include empty, `/A/B.c`, repeated slashes, leading/trailing/consecutive dots, spaces, and a long token. Record final offset separately; no fabricated expected outputs.
- [x] Write Rust tests from captured oracle outputs. Assert complete term, position and UTF-16 offset list, and explicit failure on a supplementary emoji, non-ASCII lowercasing and ASCII control character; no implicit conversion to source spans.
- [x] Run `cargo test --manifest-path crates/analysis/Cargo.toml --offline` and confirm it fails for missing implementation.
- [x] Port the original tokenizer's supported ASCII state transitions, original UTF-16 offset arithmetic and sequential positions; reject any byte outside `0x20..=0x7e` with its byte offset. Keep CDDL header, original copyright and source provenance; avoid unrelated engine dependencies.
- [x] Run the same tests, `cargo fmt --check --manifest-path crates/analysis/Cargo.toml`, and the public evidence integrity check. Capture actual outputs and do not mark full RST-003 passed. Refresh and independently verify the planning manifest without claiming a full package build if it is not run.
- [ ] Review scope, attribution, raw source identity and `git diff --check`, merge the current integration tip, then commit the reviewable stage.
