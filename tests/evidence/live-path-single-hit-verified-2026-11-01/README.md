# Single-hit path query with retained indexer logs and source-path checks

On 2026-11-01, the Mac M1 ran original OpenGrok 1.14.20 JDK/Tomcat and the Rust release probe with fresh indexes of the same verified 3,969-byte public corpus. Before timing, the retained original indexer stdout/stderr showed no known `ERROR addFile()` or nonzero parallel-indexing failure indicator and contained both completion markers. The independently running original API returned all five expected source paths across three basename searches; `coverage-*.body` and `original-path-coverage.json` preserve those observations. The Rust JSON index contained the same five paths and exact source strings, verified in `candidate-source-coverage.json`. Path presence alone does not prove all original token contents, analyzers or features are equal.

For `path=main.c&projects=alpha&start=0&maxresults=10`, the endpoints returned the same one hit. After 50 warmups per endpoint and load, 100 requests per endpoint per load produced 400 measured HTTP responses with no recorded errors. The redirect-disabled strict gate compared status, JSON types and values, path order, relevant headers and session behavior, excluding top-level time and documented transport headers. A separate recheck found all 400 saved response bodies matched the original preflight under that same comparison.

| Clients | Endpoint | p50 ms | p95 ms | p99 ms |
| --- | --- | ---: | ---: | ---: |
| 1 | OpenGrok | 2.547 | 3.673 | 5.746 |
| 1 | Rust probe | 0.482 | 0.563 | 0.820 |
| 5 | OpenGrok | 3.701 | 4.946 | 5.264 |
| 5 | Rust probe | 1.142 | 1.711 | 1.956 |

Five process snapshots show original RSS 346,128–354,800 KiB and reported CPU 146.4–224.8%, Rust RSS 1,552–2,400 KiB and CPU 0.0–1.9%. They are sparse combined-run observations, not peaks, per-query CPU cost or full process-tree accounting, and may include startup. Original and candidate index sizes were 439,911 and 5,414 bytes respectively; their feature coverage differs, so these are not equal-function storage comparisons. Each request used a new connection, endpoint windows ran sequentially, and background load, power and OS caches were uncontrolled. Original `allowMostPopular=false` differs from its default. The fixture and case do not represent the approximately 500 MB workload, full product behavior, session semantics or a product SLO. Index bytes were removed after recording hashes; original exact docIDs cannot be reconstructed.

`original-indexer-log-check.json` describes the particular checked patterns and hashes of both raw streams; absence of these patterns by itself is not proof of complete indexing. Reproduce with `tools/run_live_path_experiment.py` and `--case single-hit --warmup 50 --samples 100`, supplying a verified corpus, verified original capture and fresh external output path.
