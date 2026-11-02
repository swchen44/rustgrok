# Durable single-project path generations implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans with TDD; preserve each failed case and then implement it.

**Goal:** Extend the real `crates/path-index` with a constrained, single-project durable store and restart recovery while keeping complete old readers available during build.

**Architecture:** Keep bounded `PathIndex`/`PathSnapshot` and its atomic cancellation and writer semantics. Add a macOS-native durable wrapper whose storage directory is created as a new, exclusive child outside the explicit source tree. Resolve every directory component with anchored `openat` and `O_NOFOLLOW`; enforce an OS advisory exclusive lock across processes. A bounded canonical binary manifest contains project, generation ID, document count and each logical path, plus SHA-256. Recovery validates the pointer, checksum, exact lengths/count, limits, every path and tokenizer output before making any generation visible. No source contents or reading snapshot are stored.

**Spec:** FR-009/021, NFR-007/012, Q8/Q9, ADR-0003, WP-C07/C09 and tests SRC-001/002, RST-009 and REC-001/002; existing issue #6. The pinned upstream `IndexerTest.testIncrementalIndexAddRemoveFile` informs add/remove correctness, while durable generation/Q8 crash tests are Rustgrok-specific.

## Global constraints and review focus

- Correctness only, no performance testing. Original 1.14.20 commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`; retained CDDL/MIT/third-party provenance. No JVM product runtime.
- Only macOS native behavior can be claimed here; other platforms fail closed rather than inherit unvalidated rename/fsync/lock guarantees. Underlying filesystem/power-loss semantics remain subject to OS and filesystem guarantees.
- Authoritative complete manifest is a caller responsibility; no source discovery, filters, SCM, xref, cross-project atomicity or ranking is invented. No source write/delete or automatic deletion of unrelated files.
- Secure resolution rejects relative paths, dot-dot, symlinked components, source/storage overlap, malicious symlink files and unexpected pre-existing targets. Reject these cases under the stated exclusive same-user store ownership assumption; malicious same-user directory substitutions are outside the contract.
- Prepublication errors/cancellation/crash retain the previous pointer; after pointer rename, any parent-directory fsync failure produces `PublicationIndeterminate` and blocks new writes until reopen. Reopen must validate the exact current pointer and complete generation; never infer a full generation from parseable partial data or scan uncommitted generations.
- Retain caller-supplied path/document/live-generation limits and expose unranked candidate sets only. Cancellation and publication have one atomic winner. Competing writers in process or across processes must be rejected, not merely serialized without stale-generation protection.

## Task 1: durable publish and recovery

**Files:** `crates/path-index/src/durable.rs`, `crates/path-index/src/lib.rs`, `crates/path-index/Cargo.toml` and lock, `crates/path-index/tests/durable.rs`, test-only subprocess fixture, scoped license notices and evidence.

- [x] Write failing integration tests for first publish/reopen, add→replace→query→delete→replace and exact manifest recovery; reopen compares actual on-disk data with the original caller's complete path list, not only a success boolean.
- [x] Write tests for malformed/truncated data, wrong hash, invalid count, extra bytes, invalid/unsupported paths, source iterator error and cancellation; old durable generation stays usable and restart never loads a corrupted or uncommitted generation.
- [x] Test storage/source ancestor relationships, `..`, symlinked directory components, malicious current/generation symlinks and already-existing targets; reject without writing to source or overwriting the target.
- [x] Test a second process cannot acquire the same store lock. Inject process termination at prepared-generation, pre-pointer-rename and post-rename boundaries; restart accepts only an old or fully valid new published generation. Explicitly inject post-rename directory-sync failure, return an indeterminate result and reject subsequent writes on the same handle.
- [x] Implement secure fixed-name operations, immutable generation records with bounded length/count and SHA-256, file and directory synchronization and pointer atomic rename; retain prior immutable generations. Document orphan handling, limits and publication linearization. Do not claim a stable state after a post-rename sync error.
- [ ] Run relevant serial correctness tests, existing analyzer and in-memory integration regressions, fmt/clippy and raw evidence/manifest checks. Stage package inputs, build and verify the planning package, merge the latest integration tip and commit. Full product tests remain planned.
