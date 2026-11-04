# Bounded FULL local host evidence, 2026-11-05

Issue #6; based on integration `603945346fd8523ca31ad9839b6ea477b866fff0`. This adds the FULL backend to the existing private macOS host and typed local IPC. It preserves path single/batch modes and store schemas. No Web, ranking, grammar, reader/Unicode expansion, source reading snapshot, engine selection or benchmark/profile is introduced. The 80 complete product tests remain planned; Linux/Windows and full product acceptance are unperformed.

## Behavior and boundary

Commands are `full-host STORE SOURCE CONTROL`, `full-host-index CONTROL EXPECTED PROJECT...`, `full-host-query CONTROL PROJECT TERM`, plus `full-host-status`, `full-host-cancel`, `full-host-stop`. Explicit test-feature builds also expose `full-host-test-release`; default builds ignore test modes and reject that command. Generic status/cancel/stop bodies retain the existing `host-*` operation names. Startup distinguishes listening with/without a published generation. Query returns one retained snapshot's generation and complete unordered FULL matches with source hash/positions; different requests may cross a publication boundary.

The complete fixed registry is rebuilt through `DurableFullIndex`, with one durable commit for all projects. Queries clone immutable search data and never read source file contents or hold the writer lock. Source-root identity checks remain. The current Arc now changes only after the final successful directory sync; no current write lock is held across durable I/O. A new test-only observer pauses after pointer rename, before the final directory sync. Both the API and actual host clients read complete old generation 1 before that gate is released, then generation 2 after success. This is a controlled correctness boundary, not an in-kernel I/O stall or a latency/SLO measurement.

Cancellation before publication, unsupported reader/analysis/index term, I/O or observed source mutation preserve the previous batch. Cancellation after publication begins returns `accepted:false`. Panic/indeterminate states reject new host queries/writes until restart; durable availability is checked again after lookup, alongside the existing pre/post worker panic latch. A post-rename observer panic explicitly marks the store indeterminate, and reopening recovers the complete new durable generation.

All existing local IPC controls remain: same-user peers, trusted APFS/private control directory/ACL and ancestor descriptor binding, no-follow paths, bounded typed frames, whole-frame deadlines, eight clients and worker cleanup. Requests stay capped at 4 KiB, responses at 1 MiB. Oversized FULL results return `result_limit`, never truncated/partial matches. Direct CLI access to the host-owned store returns busy. No LAN/HTTP service is added.

## Actual results and provenance

- FULL host default 5/5; feature hooks 9/9, including two-project C/Python/JS public fixtures with real nested Git, four separate clients reading old positions/hash during second-project pause, atomic success, cancel/reader/analysis/term-byte/I/O/source-change, stale/registry, restart, panic/indeterminate and result limits.
- Publication API 2/2: old snapshots before final sync and post-rename panic/reopen.
- Affected FULL store/fault API regression 5/5; direct FULL CLI default 5/5 and hooks 7/7.
- Existing path host regression default 15/15 and hooks 23/23; batch host default 3/3 and hooks 5/5. Includes IPC/ACL/ancestor/deadline/client-cap/cleanup and panic-gap tests.
- Index and CLI all-targets Clippy with their explicit test features and `-D warnings` passed; formatting applied. Evidence/package checks have their own final receipts.

`receipt.json` binds source and public fixture hashes, runner, each unedited run stream and run receipt. Each run retains executed native binaries and the appropriate product binary separately under actual SHA-256 in `~/Library/Caches/rustgrok/evidence-binaries`; changing build features never reuses an earlier artifact claim. `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1` and sampled 2.5 GiB reserve are explicit. No reserve cleanup was needed in this slice.

Raw intended RED failures remain. Two fixture mistakes also remain identified separately: an incorrectly transcribed synthetic source SHA suffix and an invalid no-hit assumption for the real JavaScript `new` term. Fixes changed test expectations/markers, with no production token/hash behavior change. The original fixed reader/analyzer oracle was not rerun; its pinned source/license/raw provenance is linked by `receipt.json`. New Rust/diagnostic Python/tests are independently authored MIT, copyright swchen44; existing analyzer/scanner CDDL and public fixture notices remain unchanged.
