# Restricted no-hit path query: strict live experiment

**Index completeness not verified:** this historical runner deleted the fresh original capture without retaining its actual `index.stderr.txt` and `index.stdout.txt`; the similarly named `indexer.*.txt` files contain the capture subprocess output instead. Original exit 0 alone does not prove all source files were indexed. No complete path inventory was captured. These latencies and matching responses are recorded observations, but equal complete indexed data and therefore a fair whole-data comparison were not established. The original index byte count difference from the single-hit run has not been explained. The missing original index logs cannot be recovered from this record.

On 2026-11-01 the Mac M1 ran the original OpenGrok 1.14.20 JDK/Tomcat and Rust release scan probe with fresh indexes of the same verified 3,969-byte public corpus. The query was `path=absentfile.c&projects=alpha&start=0&maxresults=10`. Original `allowMostPopular=false` is an explicit departure from its default for the deferred suggestion capability. Candidate and original had zero hits. The client forbade redirects and used per-client sessions; startup used an explicit loopback-only Tomcat connector and disabled its shutdown port. Preflight and each warmup/sample compared HTTP status, JSON values and their types recursively, result path order, and relevant headers and cookie behavior, excluding only top-level time and the listed transport headers. All retained 400 timed response bodies were independently rechecked against the original preflight using the strict gate; no mismatches or recorded errors occurred.

After 50 warmup requests per endpoint per load, 100 timed requests per endpoint per load gave the following nearest-rank latency observations:

| Clients | Endpoint | p50 ms | p95 ms | p99 ms | Errors |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | OpenGrok | 2.049 | 2.740 | 2.903 | 0 |
| 1 | Rust probe | 0.469 | 0.537 | 0.814 | 0 |
| 5 | OpenGrok | 3.695 | 5.844 | 9.372 | 0 |
| 5 | Rust probe | 1.122 | 1.511 | 1.729 | 0 |

Endpoints were measured sequentially; each request used a new connection. `resources.json` contains five `ps` snapshots taken during the combined run: original RSS 350,464–357,952 KiB and reported CPU 130.3–221.2%; Rust RSS 1,664–2,160 KiB and reported CPU 0.0–1.5%. These are sparse process snapshots, not peaks, a per-query CPU cost, or a full process-tree/resource comparison. CPU samples may include startup or activity outside the timed requests. Index byte counts were 447,125 for original and 5,414 for the Rust JSON; these cover different capabilities and cannot be compared as equal-function storage. Raw samples, sanitized headers, configuration, hashes, JVM flags, environment metadata and capture integrity are retained. The index bytes were deleted by the runner after hashes and sizes were recorded; exact document IDs from this run cannot be reconstructed.

This is one no-hit query on a tiny fixture, with uncontrolled background load, power state and OS caches. It is not a representative 500 MB workload, full functional or session equivalence, SLO, or general performance comparison. The nearby `live-path-cross-project-blocked-2026-11-01` evidence shows the fairness gate rejecting a nonmatching query before timing.

Reproduce with verified public corpus and original capture:

```sh
python3 tools/run_live_path_experiment.py --oracle-capture CAPTURE --corpus CORPUS --output NEW_EXTERNAL_DIR --case no-hit --warmup 50 --samples 100
```
