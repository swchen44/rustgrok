# Rustgrok restricted path CLI

This unpublished development executable connects the macOS-only source scanner, durable path generation and analyzer. It accepts one project and one path term; it does **not** implement the OpenGrok CLI, ranking, full-text search, source browsing, SCM/history, Ctags, a server, LAN/UI or the full Q8 publication contract. It does not need Java or Git at runtime. Git is used by the isolated e2e fixtures only. Production packaging and D19 helper inventory remain open.

Build and test with a native Mac toolchain:

```sh
cargo build --manifest-path apps/grok-host/Cargo.toml --locked --offline
cargo test --manifest-path apps/grok-host/Cargo.toml --locked --offline
cargo test --manifest-path apps/grok-host/Cargo.toml --locked --offline --features test-hooks -- --test-threads=1
```

`test-hooks` enables a deliberately pausable indexing seam for tests; **never ship or deploy that feature**. The ordinary binary ignores the test-pause environment variable. The test-only build does not represent a release artifact.

Commands take UTF-8 arguments. `PARENT`, `STORE`, and `SOURCE` must be absolute; parent must already exist and be private to the current user. `NAME` is one safe store-directory component and `PROJECT` is nonempty lowercase ASCII letters, digits, `_` or `-`. The source and store must not overlap and no component may be a symlink. The create command exclusively creates a new empty store and refuses an existing target; the source is never written by the CLI.

```text
rustgrok create PARENT NAME SOURCE PROJECT
rustgrok index STORE SOURCE PROJECT EXPECTED
rustgrok query STORE SOURCE PROJECT TERM
```

Use `EXPECTED=none` for the first index, then the decimal ID returned by the prior successful index; stale IDs fail without replacing it. `TERM` is one supported path token, not a general query. Each command runs in a new process. Example success JSON lines:

```json
{"operation":"create","status":"created"}
{"operation":"index","generation":1}
{"operation":"query","generation":1,"order":"unspecified","paths":["/alpha/main.c"]}
```

Query `paths` is an **unordered candidate set**, and the array order can vary. It is not an OpenGrok ordered result. No-match is a successful empty array. Success is stdout/exit 0; failures are stderr only as `{"error":"LABEL"}`. Labels and exit codes for this development interface:

| Exit | Label | Meaning |
|---:|---|---|
| 2 | `usage` | Wrong arguments, non-UTF-8 input, non-absolute required path, invalid project or expected ID |
| 3 | `busy` | Store held by another process |
| 4 | `unsupported_or_limit`, `no_generation`, `stale_generation` | Unsupported input or resource bound, no published generation, or stale expected ID |
| 5 | `corrupt_store`, `source_changed` | Store validation failed, or observed bound source change |
| 6 | `io_error` | Filesystem or signal-monitor I/O failure |
| 7 | `publication_indeterminate` | Pointer rename succeeded but durability confirmation failed; close and reopen before determining current generation |
| 130 | `cancelled` | SIGINT was observed and cancellation won before publication |

SIGINT is cooperative. A signal must be observed by the index worker before publication begins to return `cancelled`; a late signal can result in successful publication and exit 0. A process killed before its handler is installed can terminate normally under OS signal rules. If the store is corrupt, the CLI refuses to serve it; an old generation is not claimed queryable until the store is independently recovered. An indeterminate publication does not assert whether the old or new generation is durable.

The dev profile limits one project to 50,000 documents, 4,096 bytes per logical path, 16 MiB total logical-path bytes, two live in-process generations and at most 1,000,000 generation IDs. Scanner directory-entry and nesting limits are inherited from [path-index](../../crates/path-index/README.md). This is a bounded implementation choice, not a capacity guarantee or SLO. The source scanner only supports its documented narrow filename/type subset and the captured original `.git` filter. Unsupported paths, symlinks and special files fail instead of being silently omitted.

A process holds an exclusive store lock from open until command exit. Therefore a separate query process returns `busy` while an index process holds the lock; it does **not** satisfy Q8 concurrent availability. A later long-lived service must provide that behavior. Source path/ancestor/content changes must be coordinated by the caller throughout indexing; observed metadata changes are rejected, but metadata is not a content hash and the last check is not atomic with publication. macOS-only correctness tests and evidence cannot establish Linux/Windows behavior or physical power-loss guarantees.

`src/main.rs` is independently authored MIT; see [MIT notice](RUST-LICENSE-MIT.txt). The upstream-scenario-derived subprocess test is CDDL-1.0 with its original copyright and [license](LICENSE.txt), [notice](OPENGROK-NOTICE.txt), and source attribution. `rustgrok-analysis` and `rustgrok-path-index` retain their own scoped licenses; see [evidence and dependency inventory](../../tests/evidence/path-cli-2026-11-03/README.md). No release SBOM, bundled helper inventory or completed third-party license review is claimed.
