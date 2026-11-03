# Fixed-registry batch host implementation plan

> **Execution:** Use `superpowers:executing-plans` inline; correctness work runs sequentially under the recorded 2.5 GiB sampled reserve.

**Goal:** Expose existing bounded atomic multi-project source batches through the existing private local host and explicit CLI clients, retaining old search during replacement.

**Architecture:** Keep the existing control socket, same-UID checks, strict request framing, worker state and fault latch. A backend variant binds either the unchanged single-project store or one marked batch store. Batch requests are explicitly typed and carry a complete fixed registry or a project/query. Separate writer validation from bounded serialized reader scratch, so reader requests can wait for other readers but the indexing worker cannot consume their scratch lock. One batch snapshot supplies each response's generation and project results.

**Spec:** D12/Q8, ADR-0003, D16, FR-007/009, issue #6. Q15, source mutation policy and product-level project selection remain undecided.

## Constraints and verification

- Existing single-project commands/store behavior and IPC validation remain unchanged. Batch remains macOS, path-only, fixed registry, no source snapshot, no ranking or Web; the host is loopback-local Unix IPC, not a LAN management endpoint.
- TDD public API reader/registry interleavings and simultaneous bounded reader execution; writer validation must not return Busy to a normal old-generation query. Memory remains bounded by one reader scratch, one worker, host's eight client cap and existing path/project budgets.
- TDD real process: two synthetic Git projects with nested repo; create store, start host, first index; pause while second project scans, clients obtain old per-project generation and matching paths; release and get the new generation. Separate requests may cross a successful commit; each response comes from one snapshot.
- Test second-project unsupported/I/O, cancellation and unchanged old batch, stale/changed registry rejection, real stop/restart, panic and indeterminate latches, duplicate IPC fields, default hook exclusion, same-UID socket and inherited limits/shutdown tests.
- Source/ancestor stability remains caller-coordinated; observed metadata is not a content hash. Preserve existing license notices. Save scoped raw tests and hash-retained actual binary bytes, package/manifest and precise limitations; no benchmark or implicit multi-project migration.
