# No-hit path query with retained indexer logs and source-path checks

On 2026-11-01, the Mac M1 ran original OpenGrok 1.14.20 JDK/Tomcat and the Rust release probe with fresh indexes of the same verified 3,969-byte public corpus. Before timing, the retained original indexer stdout/stderr showed no known `ERROR addFile()` or nonzero parallel-indexing failure indicator and contained both completion markers. The independently running original API returned all five expected source paths across three basename searches; `coverage-*.body` and `original-path-coverage.json` preserve those observations. The Rust JSON index contained the same five paths and exact source strings, verified in `candidate-source-coverage.json`. Path presence alone does not prove all original token contents, analyzers or features are equal.

For `path=absentfile.c&projects=alpha&start=0&maxresults=10`, the endpoints returned zero hits. After 50 warmups per endpoint and load, 100 requests per endpoint per load produced 400 measured HTTP responses with no recorded errors. The redirect-disabled strict gate compared status, JSON types and values, path order, relevant headers and session behavior, excluding top-level time and documented transport headers. A separate recheck found all 400 saved response bodies matched the original preflight under that same comparison.

| Clients | Endpoint | p50 ms | p95 ms | p99 ms |
| --- | --- | ---: | ---: | ---: |
| 1 | OpenGrok | 2.155 | 2.949 | 3.137 |
| 1 | Rust probe | 0.491 | 0.597 | 0.884 |
| 5 | OpenGrok | 3.478 | 4.987 | 5.988 |
| 5 | Rust probe | 1.132 | 1.556 | 1.908 |

Five process snapshots show original RSS 351,296–358,768 KiB and reported CPU 110.6–221.6%, Rust RSS 1,536–2,048 KiB and CPU 0.0–2.1%. They are sparse combined-run observations, not peaks, per-query CPU cost or full process-tree accounting, and may include startup. Original and candidate index sizes were 447,038 and 5,414 bytes respectively; their feature coverage differs, so these are not equal-function storage comparisons. Each request used a new connection, endpoint windows ran sequentially, and background load, power and OS caches were uncontrolled. Original `allowMostPopular=false` differs from its default. The fixture and case do not represent the approximately 500 MB workload, full product behavior, session semantics or a product SLO. The original index bytes were removed after recording hashes; original exact docIDs cannot be reconstructed.

The runner also deleted the candidate index. `candidate-index.json` was recreated afterwards from the verified public corpus and the same Rust probe, and its SHA-256 matches the hash recorded at measurement time: `51f7f42c70eb4c2184c5e19ac1e9f329023ea3be865fb3accb8644c4597315d5`. The saved bytes contain only the five public source fixtures and their derived definitions; `tests/contract/test_indexer_evidence.py` independently checks the recorded hash and each source string. This is a hash-matched reconstruction, not a claim that the original file was retained. Future runs retain their candidate index directly.

`original-indexer-log-check.json` describes the particular checked patterns and hashes of both raw streams; absence of these patterns by itself is not proof of complete indexing. Reproduce with `tools/run_live_path_experiment.py` and `--case no-hit --warmup 50 --samples 100`, supplying a verified corpus, verified original capture and fresh external output path.
