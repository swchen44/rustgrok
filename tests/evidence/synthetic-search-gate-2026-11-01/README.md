# Synthetic corpus path-search gate

This limited 2026-11-01 Mac M1 experiment reused the independently generated 10-project, 40-file, 2,621,440-byte synthetic corpus in [`../synthetic-capacity-pilot-2026-11-01`](../synthetic-capacity-pilot-2026-11-01/README.md). It contains repeated comments and no SCM repositories or history; it is not representative of the user's approximately 500 MB workload. The original and Rust indexes were copied into isolated experiment output. The original index tree hash matched the prior 40-path proof before and after this run. The original indexer's saved log streams have the previously recorded hashes and no detected known per-file failure markers; the original API returned all 40 expected paths during this run. The saved Rust JSON index matches the prior index hash and all 40 generated source contents. Path presence does not prove that all original tokens or all product functions were indexed.

For each case, both endpoints received identical path, project, pagination and query parameters. The original used `allowMostPopular=false`, differing from the upstream default because suggestions were deferred. Tomcat bound its connector to `127.0.0.1` and its shutdown port was disabled; the Rust probe also listened on loopback. The benchmark disabled redirects. Its gate compared HTTP status, complete ordered JSON values and types, relevant headers, and the observed session-cookie setup and follow-up; only top-level `time`, `Date`, `Content-Length` and `Connection` were excluded. Each simulated client retained its own cookie in memory. Redaction means this evidence does not independently establish distinct cookie values or full session equivalence.

After 20 warmups per endpoint and load, each endpoint supplied 50 timed samples at one client and 50 at five concurrent clients. Each timed request opened a new connection; the endpoints ran sequentially. All 400 retained timed bodies across the following two cases were independently rechecked against their original preflight, and all had HTTP 200 and zero recorded errors. Percentiles use nearest rank.

| Query | Clients | Endpoint | p50 ms | p95 ms | p99 ms |
| --- | --- | --- | ---: | ---: | ---: |
| Single hit | 1 | OpenGrok | 2.885 | 3.491 | 4.079 |
| Single hit | 1 | Rust probe | 2.249 | 2.589 | 3.288 |
| Single hit | 5 | OpenGrok | 5.196 | 7.470 | 7.761 |
| Single hit | 5 | Rust probe | 5.693 | 8.741 | 9.315 |
| No hit | 1 | OpenGrok | 2.491 | 3.077 | 4.259 |
| No hit | 1 | Rust probe | 2.103 | 2.376 | 2.451 |
| No hit | 5 | OpenGrok | 3.302 | 4.632 | 4.655 |
| No hit | 5 | Rust probe | 4.899 | 6.996 | 7.245 |

For the multi-project query, both returned 10 paths but their original order differed. `benchmark-multiple-hit/report.json` records `BLOCKED_OUTPUT_DIFFERENCE` and zero timed samples; the two raw preflight bodies retain the discrepancy. No sorting or score-tie exception was applied. These results are only for the selected endpoint behavior and this synthetic corpus; the five-client Rust p50 was slower in both measured cases, and no general speed claim or SLO follows.

`resources.json` retains sampled process-group RSS/CPU rows, but this run did not save the mapping from process IDs to original and candidate roles. Their CPU/RSS cannot be assigned reliably to either endpoint. `iostat-before.txt` and `iostat-after.txt` are device-wide cumulative observations that include other activity and cannot attribute I/O to a particular process, query or byte direction. Original and candidate index sizes were 616,774 and 2,668,891 bytes in the prior indexing pilot; the indexes have different functionality and are not an equal-function storage comparison. Background load, power state and OS caches were uncontrolled.

The disk guard sampled every approximately 0.25 seconds during the run; the lowest recorded free-space sample was 7,331,897,344 bytes, above the 6 GiB reserve. A subsequent attempted rerun stopped before output creation with 5,749,108,736 bytes free, below the 6,442,450,944-byte reserve. Its returned error values are transcribed into `v2-guard-blocked.json`; no v2 raw stdout/stderr file was captured and no v2 timing occurred. Sampling cannot bound writes between checks and is not a filesystem quota. No existing user data was deleted.

`tests/contract/test_synthetic_search_evidence.py` independently verifies the recorded corpus and index hashes, all original path responses, 400 timed bodies, percentile arithmetic and the blocked multi-hit order. The original index generation remains only in the local pilot directory and cannot be reconstructed exactly from this repository evidence alone. Reproduction needs sufficient free space, the retained index/corpus and the original coverage proof; it remains capped at 16 MiB of synthetic source by the pilot tools.
