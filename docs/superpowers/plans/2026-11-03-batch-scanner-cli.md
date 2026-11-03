# Fixed-registry batch scanner and CLI implementation plan

> **Execution:** Use `superpowers:executing-plans` inline, one task at a time; do not run benchmarks.

**Goal:** Scan all explicitly named project directories, validate the entire observed batch and publish it through one existing durable batch commit; expose it through a restricted macOS direct CLI.

**Architecture:** `sourceRoot/<project>` is a one-level explicit mapping, with validated single-component project names and no source auto-discovery. The existing scanner is reused with no-follow descriptors and its current file restrictions. The batch writer lock covers scanning, per-project builds, a full second observation immediately before the existing one-pointer commit, and cooperative cancellation. Existing single-project stores and commands are unchanged.

**Tech Stack:** Rust path-index, analysis and macOS direct CLI; public/synthetic fixtures and existing Git binary only in tests.

**Spec:** D12, ADR-0003, FR-007/009 and issue #6. This is single-process direct CLI; no host old-generation concurrency, full Q8, source snapshot, Web, ranking, SCM history or platform acceptance.

## Contracts and tests

- `batch-create PARENT NAME SOURCE_ROOT` creates a new marked store; first successful `batch-index STORE SOURCE_ROOT none PROJECT...` fixes the complete registry. Subsequent indexing requires its exact registry. Names are lowercase ASCII letters/digits/`_`/`-`; each maps to a distinct direct child directory with no symlink. Other source-root entries are outside the explicitly requested scope.
- `batch-query STORE SOURCE_ROOT PROJECT TERM` and `batch-query-expression ... EXPRESSION` select one existing project, return generation, unordered full paths and explicit errors. Direct process lock may return busy during index.
- Extend the existing scanner to observe a descriptor and check cancellation during traversal; existing filtering and metadata limitations remain. Both initial and precommit observations of all named projects must agree, including paths/metadata and root identity. Source mutation remains externally coordinated; a metadata comparison is not a content hash.
- Test-first: real subprocess two-project Git and nested Git fixture, create → index → query → change → second-project failure → same old `CURRENT`/both old views → successful reindex → both new views; changed registry, absent/duplicate/unsafe/symlink project and store mismatch reject explicitly, source bytes remain unchanged.
- Feature-only controlled precommit hook must verify real SIGINT causes cancellation and preserves old batch, a separate CLI sees busy while indexing, and source mutation after scanning fails before publication. Default binary ignores the hook.
- Run sequential offline crate and CLI correctness suites under a sampled 2.5 GiB free-space reserve; record commands, source and raw output hashes, actual test/product binaries by hash, and package integrity separately from product acceptance. Keep issue7 files untouched; no large corpus or deletion.
