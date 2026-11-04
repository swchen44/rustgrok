# Single-project path candidate index implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans with TDD. Steps use checkboxes.

**Goal:** Build a real, bounded-scope Rust path posting index with atomic single-process generation replacement and explicit failure behavior.

**Architecture:** `crates/path-index` consumes a caller-supplied complete logical-path manifest and `rustgrok-analysis`; it builds immutable token postings off to the side, then atomically swaps an `Arc` under a short write lock. A snapshot keeps its original generation alive during replacement. Query returns an unordered set of candidate paths for one analyzed term, never an ordered OpenGrok result list. This does not traverse source, index contents, implement Lucene ranking or persist across restarts. Full source selection/filtering, durable recovery and multi-project publication remain blocked until their contracts are validated.

**Tech stack:** Rust standard library and the local CDDL-scoped `rustgrok-analysis` crate; no JVM in product.

**Spec:** `docs/02-requirements.md` FR-001/002/003/007/008/009/021; `docs/03-system-design.md` 3–6; `docs/adr/0003-published-generation-switch.md`; issue #6, WP-C07/C08/C09, RST-003/009, SRC-001/002, REC-001/002.

## Constraints and limits

- Single project and a trusted, explicitly complete logical manifest; the caller supplies a declared count, and missing/extra, duplicate, malformed, wrong-project, unsupported, source I/O or cancellation errors abort publication. A caller's incorrect claim of completeness cannot be detected without the unimplemented scanner; this API must not be wired as a complete source-tree index until that contract exists.
- OpenGrok 1.14.20 `IndexDatabase.accept` applies `pathAccepter`, readability, symlink and optional versioned-only rules. Do not silently choose any of these policies. This slice consumes selected paths only and never touches source files.
- Q8 within this process: existing successful generation and snapshots remain queryable during build; publish once only after every supplied document succeeds. Query before first success fails explicitly. Cancellation uses an atomic token, cannot interrupt a blocked iterator and reports whether cancellation or publication won; panic/lock poisoning fails closed. Caller-selected limits also bound input paths and live generations; retained readers can block a future build. In-memory state is not crash-safe persistence and cannot meet full recovery acceptance.
- Q9: no source bytes, xref, history, raw or reading snapshot. No API/URL changes. No result order claim; full ranking, full grammar, multi repo detection, source acquisition and other language features remain required.
- No perf measurement, JVM runtime, copying user data or parallel agent jobs. CDDL analysis dependency and any adapted upstream test retain their original notices; independent index code uses MIT swchen44.

## Test-first task

**Interfaces:** `PathIndex::new(project: &str, limits: IndexLimits) -> Result<Self, IndexError>`; `replace<I>(&self, expected_generation: Option<u64>, expected_count: usize, paths: I, cancellation: &CancellationToken) -> Result<u64, IndexError>` where `I: IntoIterator<Item=std::io::Result<String>>`; `snapshot(&self) -> Result<PathSnapshot, IndexError>`; `PathSnapshot::generation(&self) -> u64`; `PathSnapshot::manifest(&self) -> HashSet<String>`; `PathSnapshot::lookup(&self, query: &str) -> Result<HashSet<String>, IndexError>`. This intentionally returns candidates, not ranked user-visible search results.

- [x] Write external integration tests using independently generated C/Python/JS logical source paths; verify add→replace→query→remove→replace→no-hit, based on upstream `IndexerTest.testIncrementalIndexAddRemoveFile` without claiming full parity.
- [x] Test old snapshot and concurrent readers during a deliberately paused next build; after successful publish fresh snapshots see only the full new generation, retained old snapshots remain unchanged.
- [x] Test error after a partial inventory, explicit cancellation, a mismatched count, duplicate/malformed/wrong-project path, unsupported Unicode and simulated I/O errors; each leaves the successful generation intact. Verify missing initial generation and unsupported multi-token query are errors. No skip/stub is treated as a pass.
- [x] Run tests red first; implement bounded API using `rustgrok-analysis` for both index and query. A nonblocking `Mutex` rejects concurrent writers and generation preconditions reject stale writers and an `RwLock<Option<Arc<_>>>` protects only publication and snapshot acquisition; reader operations never use the writer lock. Reject invalid input rather than partially publishing.
- [ ] Run the actual crate integration tests, analyzer regression suite, fmt/clippy and evidence/manifest checks. Document whether package build actually ran, preserve pinned upstream source hash and scope. Merge the current integration tip and commit for independent review.
