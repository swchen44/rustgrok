# Rustgrok restricted path CLI

This unpublished development executable connects the macOS-only source scanner, durable path generation and analyzer. It accepts one project and one path term; it does **not** implement the OpenGrok CLI, ranking, full-text search, source browsing, SCM/history, Ctags, a web server, LAN/UI or the full Q8 publication contract. It does not need Java or Git at runtime. Git is used by the isolated e2e fixtures only. Production packaging and D19 helper inventory remain open.

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

Direct CLI processes hold an exclusive store lock until command exit, so another direct query process returns `busy` during direct indexing. The restricted local host described below provides concurrent path queries within its own single-project scope; complete Q8 availability remains unimplemented. Source path/ancestor/content changes must be coordinated by the caller throughout indexing; observed metadata changes are rejected, but metadata is not a content hash and the last check is not atomic with publication. macOS-only correctness tests and evidence cannot establish Linux/Windows behavior or physical power-loss guarantees.

`src/main.rs`, `src/host.rs` and `tests/host_e2e.rs` are independently authored MIT; see [MIT notice](RUST-LICENSE-MIT.txt). The upstream-scenario-derived subprocess test is CDDL-1.0 with its original copyright and [license](LICENSE.txt), [notice](OPENGROK-NOTICE.txt), and source attribution. `rustgrok-analysis` and `rustgrok-path-index` retain their own scoped licenses; see [evidence and dependency inventory](../../tests/evidence/path-cli-2026-11-03/README.md). No release SBOM, bundled helper inventory or completed third-party license review is claimed.

## Local path host (development adapter)

The foreground host owns one existing store and serves a private, single-user Unix socket, allowing separate CLI clients to query its in-memory published path generation during a rebuild. It remains single-project, path-only and unranked; it does not implement full Q8, source content/xref/history atomicity, Web or the original API. Existing direct `create/index/query` commands are unchanged. The socket is a development control adapter, not a frozen public protocol or a decision about Q15.

```text
rustgrok host STORE SOURCE PROJECT CONTROL_DIR
rustgrok host-index CONTROL_DIR EXPECTED
rustgrok host-query CONTROL_DIR TERM
rustgrok host-status CONTROL_DIR
rustgrok host-cancel CONTROL_DIR
rustgrok host-stop CONTROL_DIR
```

Create the store first with `create`. `CONTROL_DIR` must already exist, be absolute and owned by the caller with no group/other permission bits; every path component must be non-symlink. It and its ancestors/descendants must not overlap source or store. Each directory from `/` to `CONTROL_DIR` must be on a local APFS volume, report extended-security capability `1` and non-opaque authorization `0`, and be owned by root or the effective UID; group/other writable ancestors are rejected unless sticky, and every extended ACL is rejected, including deny-only ACLs. Non-APFS, remote, opaque or unrecognized capability results fail closed in this development profile. ACL lookup errors other than the locally observed no-ACL `ENOENT` also fail closed. Many macOS home directories have a deny-delete ACL and therefore cannot contain this development control directory. Use a separately created suitable private directory; the program never changes its ancestors' permissions or ACLs. The full `CONTROL_DIR/control.sock` path must fit the existing macOS development limit (at most 103 bytes).

The host retains a descriptor for the validated control directory, temporarily changes cwd to it only during single-threaded host startup, binds a fresh relative `control.sock`, restores cwd before starting workers/clients, sets socket mode 0600 and checks each peer's effective UID. One-shot control clients similarly connect relative to the retained descriptor before starting any application threads. Cleanup uses descriptor-relative inode checks and `unlinkat`; it never follows a replacement pathname. A malicious same-UID process can still race inside or move the trusted control directory and is outside the security guarantee; one same-UID rename/symlink substitution was tested. Other-UID ancestor replacement is prevented by the mode/sticky/ACL trust restrictions, but a separate different-UID run was not performed. Nothing listens on TCP or LAN.

Startup emits `{"operation":"host","state":"listening","published":false}` when no generation exists; `published:true` means a validated generation was opened. Listening with no generation is not search readiness. `host-index` returns `{"operation":"host-index","job":1,"state":"running"}` when the bounded job is accepted; it does not mean indexing succeeded. Poll `host-status` for the last job: `running`, `succeeded` with `generation`, `failed` with `error`, or `cancelled` with `error`. Only one job runs; a second returns `busy`. A stale `EXPECTED` is reported as a failed job, not a successful publication. Job IDs and the last status are in-memory and reset on host restart. A client timeout can happen after a job was accepted; inspect status before retrying.

`host-query` returns `{"operation":"host-query","generation":1,"order":"unspecified","paths":[...]}` or an explicit error. During a replacement it reads the old successfully published generation; after success it reads the new one. The result is an unordered candidate set, not an OpenGrok ranking. A result frame exceeding the limit returns `result_limit`, never truncated results. The inherited source scanner constraints and its external source-stability requirement apply. Existing already-held snapshots and source-root replacement limitations remain as documented by path-index.

`host-cancel` reports `accepted:true` only if the cooperative token won before publication; false means publication may proceed. After an index failure or accepted cancellation the old generation remains queryable. If publication is indeterminate, or the worker's unwind guard has marked a caught panic, the host blocks subsequent query/index until restart rather than asserting which generation is current. The panic latch is set during unwinding; this does not claim an instantaneous zero-time boundary at the physical panic or handle abort-mode process termination. `host-stop` with no running job returns `stopping`, stops accepting, drains clients and joins workers. During a job it requests cancellation and returns `busy`; wait for its terminal status and retry stop. SIGINT/SIGTERM requests shutdown and cancellation, and exits after the worker returns. A blocked filesystem operation may delay cancellation/shutdown; no force-kill or physical-power-loss guarantee is made. The OS store lock stays with the host until it exits; direct CLI opens of the same store return `busy` while it runs.

Requests are one bounded JSON frame per local connection, at most 4,096 bytes; response frames are at most 1 MiB. There are at most eight simultaneous client handlers and one indexing worker. Connect, whole-frame read and whole-frame write each have a three-second deadline; operation execution and a blocked filesystem are not forcibly time-limited. A timeout is an error, not an empty search result. The server validates socket owner/type and the same-UID peer. Graceful shutdown unlinks only the socket inode created by that host. A crash or SIGKILL can leave a stale socket; startup refuses to overwrite it. After independently confirming no host owns the endpoint, an operator must remove that exact stale socket before restarting.

Controlled precommit pause/release, injected panic and injected post-rename sync failure exist only in the `test-hooks` build and must never be shipped. Default host ignores test-mode environment settings and does not offer the test release command. The tests and limits are macOS-only; Linux/Windows, full Q8, installed-product browser e2e and release packaging remain unverified.

### Restricted path expressions

The development commands `query-expression STORE SOURCE PROJECT EXPRESSION` and `host-query-expression CONTROL_DIR EXPRESSION` return the corresponding operation, published `generation`, `order: "unspecified"` and all candidate `paths`. The existing `query` and `host-query` remain single-term commands. Host result frames above 1 MiB fail with `result_limit`; they are not truncated.

The current expression subset accepts ASCII letter/digit/underscore single-token path terms, case-normalized by the path analyzer, implicit conjunction, uppercase `AND`, uppercase `OR`, `NOT` and `-` exclusion, unary `+` on an initial term, and nested parentheses. A pure negative clause returns no candidates. Same-level combinations of `OR` with conjunction or negative clauses and other unsupported forms are rejected: use supported grouping rather than assuming precedence. Only ASCII space is accepted as a whitespace separator; all C0 control characters, including tab, CR, LF, form feed and vertical tab, and DEL are rejected. Query length is bounded by the configured maximum path bytes, with at most 128 lexical tokens and 16 nested groups. Wildcards, regex, fuzzy, ranges, fields, quoted/escaped syntax, Unicode, path punctuation and analyzer expansions are explicitly unsupported. Errors use `unsupported_or_limit`; neither snippets, scores nor original sorting are produced. Accepted results are exhaustive within the currently published single-project manifest, subject to the scanner and generation restrictions above.

Observed original examples and their narrow scope are retained in `tests/evidence/path-query-2026-11-03/`. The single-field subset is not a complete OpenGrok query parser, UI/API compatibility claim, or product acceptance. Source-span fidelity, full Unicode, mixing clauses at one level, multi-field parsing, scoring and ranking remain open under #6.
