# 20 MiB path instrumentation and experimental cache (2026-11-02)

This is a partial Mac arm64 experiment using the same deterministic 10-project, 80-file, 20,971,520-byte synthetic corpus as the earlier sequential experiment. It contains no real project, Git history or enterprise source. It is not a 500 MB workload or product acceptance. The corpus and candidate index were verified by the existing tools; the original API returned all 80 paths in this run. The original indexer logs are from the prior indexing run. The retained input/index provenance and its limitations are in `../synthetic-20m-sequential-2026-11-01/README.md`.

## Instrumented source path

The standalone `experiments/search-spike/examples/profile_path.rs` measures sequential CLI stages using the same path matcher and JSON document shape, not the HTTP service or a sampling profiler. It includes timer overhead, drops the full parsed document collection during the scan interval, and does not distinguish filesystem cache hits from physical I/O. With 60 iterations per case, discarding the first 10 as warmups, the medians below are the arithmetic means of the two middle values among the remaining 50 samples:

| Query in one project | Read | Deserialize | Scan, including drop | Matcher, within scan | Serialize |
| --- | ---: | ---: | ---: | ---: | ---: |
| Single hit | 3.329 ms | 9.192 ms | 0.701 ms | 0.005 ms | 0.001 ms |
| No hit | 3.269 ms | 8.897 ms | 0.593 ms | 0.004 ms | 0.001 ms |

The matcher interval is a subset of scan, not an additional cost. These observations suggest where this experimental CLI spent time; they do not quantify synchronous HTTP service overhead, prove physical disk reads, or isolate all causes of the prior HTTP latency. The example checks each iteration for the same result, byte length, document count and selected-document count; it does not verify that the complete input bytes stayed identical across iterations. The receipt records an index hash before each run, not a per-iteration input hash. The example was formatted and its diagnostic wording clarified after the profiled build; the profile binary hash is recorded, but no reproducible-build attestation is claimed.

## Optional cache and correctness scope

`serve --cache-mode exact` opts into a one-entry parsed-index cache; default `off` preserves the uncached mode. Every request still reads the index file. Cached parsed data is reused only when the complete file bytes match, never based on timestamps or hashes alone. A changed valid file is parsed before it is used; an unreadable or missing file, or invalid JSON or document shape, produces an error rather than returning the older result. A syntactically valid array containing only some expected documents is accepted, since this experiment has no completeness manifest or generation validation; the CLI parser regression test demonstrates this limitation without starting a server. Retained memory includes the prior bytes and parsed documents, and a further copy is read per request; this is a bounded one-entry design, not bounded memory independent of index size. Concurrent writing can expose a partial read, just as in the original spike; source snapshots and Q8 generation-publish semantics are not implemented here.

The recorded contract test ran uncached and cached servers sequentially and checked repeated reads, atomic replacement, same-length in-place replacement with restored mtime, malformed data, missing file and recovery. It used a small one-document fixture, not the OpenGrok full compatibility suite or concurrent-writer proof. The unknown-mode check also passed. This spike does not establish Q8 publication semantics, cross-project tie ordering, SCM behavior or product compatibility.

## Restricted comparison and interruption

Original, baseline and exact-cache preflights were executed with one server at a time. The original and each candidate matched status, body and result order, relevant headers and observed first/follow-up session behavior for single-hit and no-hit path queries. All eight cross-project multi-hit cases retained their ordering differences and remained blocked. A fresh original timing run and baseline timing run completed with 20 warmups and 50 timed samples per load for each case and 1/5 clients; all 400 timed responses reported no errors. Their medians below are the arithmetic means of the 25th and 26th ordered values for each set of 50 samples; the raw measurement reports use nearest-rank p50 (the 25th value), so the report p50 may differ. Values are in milliseconds:

| Query | Clients | Original | Baseline Rust |
| --- | ---: | ---: | ---: |
| Single hit | 1 | 2.657 | 14.727 |
| Single hit | 5 | 3.928 | 41.710 |
| No hit | 1 | 2.220 | 13.801 |
| No hit | 5 | 3.179 | 38.989 |

These are uncontrolled sequential windows on synthetic inputs, not product performance. Baseline binary SHA-256 was `0d06036588295b7ef1cfb88a597ea6f94c244eb6e74d1cf983681073b06d275a`; cache binary SHA-256 was `f9964a4f085f0bf069295328aa05420a166df2ed2926007c7f38c0df0008d53d`. The exact-cache binary passed preflight but **was not timed**. Before its planned fresh original comparison, the disk guard stopped the job before launch: free space was observed at 3,444,535,296 bytes, below the 3,758,096,384-byte sampled reserve. There is no cache speed claim or fair cache performance comparison in this evidence.

The previous 4.5 GiB engineering guard initially stopped a test before launch at 4,638,330,880 free bytes. For this limited reuse of existing 20 MiB inputs and a small build, the supervising agent authorized a 3.5 GiB floor; it is a sampled guard, not a filesystem quota. Guards sample approximately every 0.25 seconds and cannot prevent a large write between samples. No 500 MB corpus was created. The receipts report stopped process groups, but do not independently establish a complete timestamped OS process history across all stages.

`raw-observations.zip` contains original response bodies, redacted session observations, timed sample bodies and reports, instrumentation outputs, and guard receipts. `raw-observations-manifest.json` records each member's hash and the archive hash. Runtime Tomcat deployments and index copies are excluded. `measurement-helper-experiment.patch.b64` contains the base64-encoded exact temporary helper edits used for the run; they were reverted from active helpers after interruption and are not reviewed as a general-purpose change. Existing planning quality report describes an earlier package run; no full package build or product tests were run for this commit.

## Space inventory (read-only)

At the time of inspection, allocated sizes from `du -sk` were: `experiments/search-spike/target` 174,424 KiB (shared existing build target, not all newly created); `/tmp/rustgrok-opt-original-preflight` 68,620 KiB and `/tmp/rustgrok-opt-oracle-baseline-measure` 69,900 KiB (each contains a Tomcat deployment and copied original index); `/tmp/rustgrok-opt-baseline-measure` 1,340 KiB; each new candidate preflight 128 KiB; `/tmp/rustgrok-profile-test` 108 KiB; `/tmp/rustgrok-profile-20m-v1` 32 KiB; and the preserved old binary `/tmp/rustgrok-search-spike-baseline-ffdaac2` 700 KiB. The pre-existing `/tmp/rustgrok-synthetic-20m-v1` and `/tmp/rustgrok-synthetic-20m-index-v1` used 20,496 KiB and 25,032 KiB respectively; their inputs are needed to repeat the experiment. All sizes are observations, may change, and do not explain the full free-space decline. No data was deleted.
