# Sequential 20 MiB synthetic search experiment

On the Mac M1 with 8 GiB RAM, this experiment generated 10 projects with 8 C/Python/JavaScript files each, exactly 20,971,520 source bytes. All text is deterministically generated repeated comments; there are no SCM repositories or histories. The manifest and retained candidate JSON let the recorded 80 inputs and candidate content be independently checked. This synthetic data is not representative of real projects or the user's approximately 500 MB source workload.

The user's later instruction was to avoid running workloads simultaneously. Source generation, original indexing, Rust indexing, original HTTP path checks, Rust preflight, original timing, and Rust timing therefore ran in sequence. The two product servers did not run together. Before a later server started, the previous server and its measurement process groups were confirmed stopped; the receipts retain group IDs and results. Five simulated clients were used within the relevant single-server measurement. The old concurrent-server benchmark mode is disabled in the current tool.

The earlier 6 GiB sampled free-space threshold was an engineering choice. After the sequential-work instruction, the observed free space before source creation was 5,817,298,944 bytes; this stage used a 4.5 GiB floor of 4,831,838,208 bytes, leaving 985,460,736 bytes of initial sampled headroom. The generator checked free space before each file; index and server stages sampled approximately every 0.25 seconds, with additional sampling overhead. The lowest retained sample across the recorded stages was 5,532,700,672 bytes. These are sampled checks, not an OS quota: they do not establish a hard upper bound between samples or a safe budget for a 500 MB run. No existing user data was deleted.

## Input and correctness checks

Both index processes exited zero and their process groups stopped before the next stage. Original raw indexer stdout/stderr and receipt are retained. The selected known per-file failure markers were absent and both completion markers appeared; this log check alone cannot prove complete indexing. The original API returned all 80 expected paths, and its input index tree hash was unchanged while the isolated copy was queried. The retained Rust candidate index contains the same 80 paths and exact generated content; its SHA-256 is in `probe/report.json`. This path/content check does not establish complete original token indexing, all language behavior or product parity.

Eight cross-project path queries each returned 10 paths on each side, but all eight had different path order in the original and Rust raw bodies. They are retained in `candidate-preflight/report.json` and were not timed. Two further queries used a single selected project: one fixed hit and one absent path. Original and Rust were started separately, and saved raw first/follow-up bodies were SHA-verified before timing. Their preflight compared status, JSON values and types, result order, relevant headers and the observed session-cookie creation/follow-up, excluding only top-level `time`, `Date`, `Content-Length` and `Connection`. The original had `allowMostPopular=false`, an explicit deviation from upstream default for the deferred suggestion feature. The observations do not establish all session behavior or complete HTTP compatibility.

## Restricted path timing

Only the two queries whose full preflight matched were timed. For each role and query, each load had 20 warmups and 50 timed requests. There were 1- and 5-client loads, with a distinct in-memory cookie per simulated client; cookies were redacted from retained reports. Original timing finished and its process groups stopped before Rust timing began. Each timed response and relevant headers were checked against the saved original follow-up; all 400 retained timed responses were HTTP 200 and had no recorded error. Each request opened a new connection. The endpoints ran in sequential windows, with uncontrolled background load, power state and OS caches; these are not paired or representative-production measurements. Percentiles use nearest rank.

The historical `candidate-measure/report.json` has a misleading top-level status `BODY_ONLY_DIAGNOSTIC_NOT_TIMED`: that label describes its eight cross-project body checks, while its `measurements` array and separate `measure-candidate-*` reports record the two timed single-project cases. The raw receipt is retained unchanged; the runner now distinguishes those situations in its top-level status.

| Query | Clients | Endpoint | p50 ms | p95 ms | p99 ms |
| --- | --- | --- | ---: | ---: | ---: |
| Single hit | 1 | OpenGrok | 2.657 | 3.297 | 5.356 |
| Single hit | 1 | Rust spike | 12.894 | 14.052 | 14.258 |
| Single hit | 5 | OpenGrok | 3.788 | 5.262 | 6.179 |
| Single hit | 5 | Rust spike | 37.917 | 63.368 | 63.580 |
| No hit | 1 | OpenGrok | 2.120 | 3.017 | 3.839 |
| No hit | 1 | Rust spike | 12.868 | 13.581 | 13.946 |
| No hit | 5 | OpenGrok | 3.437 | 4.554 | 4.909 |
| No hit | 5 | Rust spike | 37.688 | 63.973 | 64.983 |

The Rust spike was slower for both of these restricted cases in this run. This does not establish a product speed result or SLO. `tests/contract/test_sequential_20m_evidence.py` independently rechecks saved input hashes, raw preflight and sample bodies, headers, counts, path ordering and percentile arithmetic. Cookie values are intentionally absent, so their distinctness is a recorded in-memory check rather than independently reproducible from the saved values.

## Resources and source review

The original index/data occupied 3,838,874 bytes and the Rust JSON index 21,330,431 bytes. They implement different features, so these sizes are not equal-function storage costs. During sequential HTTP stages, sampled original-server RSS ranged from 880 to 348,928 KiB and sampled `ps` CPU from 0.0% to 261.0%; sampled Rust-server RSS ranged from 32 to 110,224 KiB and CPU from 0.0% to 96.3%. These include startup and are snapshots of server process groups, not measured peaks or full resource accounting. `iostat` before/after outputs are cumulative device-wide observations including unrelated activity; they do not establish per-process physical read/write bytes. The indexing `time -l` streams retain elapsed/CPU/RSS and block-operation counters; zero block operations do not prove zero I/O.

Static review of the pinned spike source found that every path request calls `search`, which reads and deserializes the entire 21,330,431-byte JSON index before filtering its 80 documents by project. For each selected document, `path_matches` tokenizes its path and the query again. Its server handles incoming connections in a synchronous loop. These are observable source-code paths and plausible contributors to the single- and five-client measurements. No runtime profiler or per-function attribution was performed, so the measurements do not quantify their individual costs or prove physical disk reads or exact causality. `source-analysis.json` records the source and candidate binary hashes; it is not an independent reproducible-build attestation.

The exact original Lucene index generation is retained only in the local temporary pilot directory, not in this repository. The candidate JSON and all compared raw responses are retained here. The 32 MiB pilot input cap and 4.5 GiB sampled reserve are not a decision that a larger run is safe; capacity and resource growth must be assessed before moving closer to approximately 500 MB. Full product, SCM, Ubuntu and Windows acceptance remains unperformed.

A subsequent planning-package build ran alone under the same sampled 4.5 GiB guard and passed its structure and archive-integrity checks; it did not run product tests. The 80 product test specifications remain planned.
