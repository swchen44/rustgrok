# Restricted single-hit path query: strict live experiment

**Index completeness not verified:** this historical runner deleted the fresh original capture without retaining its actual `index.stderr.txt` and `index.stdout.txt`; the similarly named `indexer.*.txt` files contain the capture subprocess output instead. Original exit 0 alone does not prove all source files were indexed. No complete path inventory was captured. These latencies and matching responses are recorded observations, but equal complete indexed data and therefore a fair whole-data comparison were not established. The original index byte count difference from the no-hit run has not been explained. The missing original index logs cannot be recovered from this record.

On 2026-11-01 the Mac M1 ran original OpenGrok 1.14.20 JDK/Tomcat and the Rust release scan probe with fresh indexes of the same verified 3,969-byte public corpus. The query was `path=main.c&projects=alpha&start=0&maxresults=10`; each endpoint returned the same one result. Original `allowMostPopular=false` is an explicit departure from its default for the deferred suggestion capability. The client forbade redirects and used per-client sessions; startup explicitly bound the Tomcat connector to loopback and disabled its shutdown port. Preflight and each warmup/sample compared HTTP status, JSON values and their types recursively, result path order, and relevant headers and cookie behavior, excluding only top-level time and the listed transport headers. All retained 400 timed response bodies were independently rechecked against the original preflight using the strict gate; no mismatches or recorded errors occurred.

After 50 warmup requests per endpoint per load, 100 timed requests per endpoint per load gave these nearest-rank latency observations:

| Clients | Endpoint | p50 ms | p95 ms | p99 ms | Errors |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | OpenGrok | 2.588 | 3.430 | 5.926 | 0 |
| 1 | Rust probe | 0.500 | 0.566 | 0.856 | 0 |
| 5 | OpenGrok | 3.782 | 5.071 | 5.273 | 0 |
| 5 | Rust probe | 1.125 | 1.566 | 1.908 | 0 |

Endpoints were measured sequentially; each request used a new connection. Five `ps` snapshots during the combined run show original RSS 363,120–370,080 KiB and reported CPU 148.9–206.5%; Rust RSS 2,048–2,336 KiB and reported CPU 0.0–1.3%. These sparse snapshots are not peaks, a per-query CPU cost, or a full process-tree/resource comparison; CPU readings may include startup or activity outside the timed requests. Index byte counts were 45,327 original and 5,414 Rust JSON, with different capabilities; these are not equal-function storage measurements. Raw samples, sanitized headers, configuration, hashes, JVM flags, platform metadata and capture integrity are retained. The indexes were deleted after hashes and sizes were recorded, so exact document IDs for this run cannot be reconstructed.

This is one single-result query on a tiny fixture, with uncontrolled background load, power state and OS caches. It does not establish full functional or session equivalence, representative 500 MB performance, SLO, or a general speed result. The cross-project case in `../live-path-cross-project-blocked-2026-11-01` was rejected by the gate before timing. Repeat using `tools/run_live_path_experiment.py` with `--case single-hit --warmup 50 --samples 100` and verified corpus/capture paths as in the adjacent no-hit README.
