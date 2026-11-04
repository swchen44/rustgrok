# Atomic multi-project manifest batch implementation plan

> **For agentic workers:** Use `superpowers:executing-plans` sequentially; preserve the acceptance and evidence boundaries below.

**Goal:** Provide a durable, explicitly bounded multi-project path-manifest batch API with one atomic publication point.

**Architecture:** A new, separately marked batch store uses the existing macOS durable generation mechanism as its single commit point. Its generation contains a complete fixed project registry and the logical paths for every registered project; each project is independently validated by the existing path index before any generation is published. A batch reader retains one durable snapshot and reconstructs a project's postings from that same manifest. No existing single-project store is converted or automatically migrated.

**Tech Stack:** Existing Rust path-index and analysis crates, existing macOS descriptor-relative durable storage, sha2, offline Cargo tests. No JVM product dependency.

**Spec:** D12 and `docs/adr/0003-published-generation-switch.md`, FR-007/009, NFR-005/007/012/016, issue #6.

## Global constraints

- A job covering multiple projects publishes none until all have succeeded; failure or cancellation keeps the previous complete batch.
- Existing single-project stores and interfaces remain usable; a batch store is separately marked and must not open through single-project API.
- Caller supplies the complete fixed registry and complete per-project path manifests. This first API explicitly rejects changing the registry or partial/subset updates; it does not decide adding/removing projects or product-level job scoping.
- Every project path must belong to that project, and project/document/path byte budgets are checked. The store source root and storage are disjoint using existing descriptor-relative checks. API reads no source file data and does not mutate sources.
- The initial interface accepts caller-supplied manifests, not automatic source discovery or an assertion that the caller captured a stable source. Full Q8, other related data, CLI/Web and multi-project ranked search remain incomplete.
- macOS native proof only. No benchmark, profiling, other agents, or simultaneous build/test jobs.

## Review focus

- Two projects, old reader held during blocked second project: both old until publication; both new after; old held reader remains old.
- Unsupported path, count mismatch, input I/O and cancellation in the second project never publish the first.
- Partial or changed registry, duplicate projects, escaped paths and project/total budgets fail closed.
- Concurrent/stale writers cannot publish competing batches; no process may open batch storage through the ordinary single-project API.
- Failed directory sync after rename yields indeterminate, blocks reads/writes until reopen; restart and crash observe one whole old or new batch, never a mixture.

### Task 1: Fixed-registry, single-commit batch core

**Files:** Create `crates/path-index/src/durable/batch.rs`, `crates/path-index/tests/durable_batch.rs`; modify `crates/path-index/src/durable.rs` only to expose batch module and reject the distinct batch marker from regular single-project open. Update `crates/path-index/README.md`.

**Interfaces:** `BatchLimits`, `BatchProject<I>`, `DurableBatchIndex::{create,open,replace,snapshot}`, and `BatchSnapshot::{generation,projects,lookup,lookup_expression}`; `BatchProject` has project identity, expected count, and an iterator of `io::Result<String>`. `lookup` and `lookup_expression` use the existing path index on restored original paths; the same retained batch snapshot selects every project.

- [x] Write public-API red test for two projects, a paused second manifest, old/new readers and fixed registry; run it before implementation.
- [x] Implement one generation containing registry markers and all project paths, validated by existing path index; only one durable pointer rename publishes it. Keep aggregate limits and both ordinary/batch store identification explicit.
- [x] Write red/green tests for invalid, missing, duplicate, I/O, cancelled or changed project manifests and source isolation; verify the old batch remains.
- [x] Test stale and concurrent writers and bounded live generations. Test distinct batch marker against single-project API and no implicit migration of single stores.

### Task 2: Recovery, fault and evidence

**Files:** Extend `durable_batch.rs`, add `tests/evidence/batch-publication-2026-11-03/` and focused contract checks; update docs and package manifest.

- [x] Exercise real child process interruption before/after the one durable pointer rename, reopen and check both project generations together; test injected directory-sync failure blocks reads/writes until reopen.
- [x] Test symlink/path isolation and re-opening, retained reader, low-space/corrupt generation and cancellation behavior via existing fault seams where available.
- [x] Run sequential offline correctness tests, formatting and Clippy; retain commands, logs, hashes, provenance and limitations. Package integrity is not full product acceptance.
- [x] Confirm integration ancestor, preserve issue7 changes, run diff check, commit and hand off for independent review.
