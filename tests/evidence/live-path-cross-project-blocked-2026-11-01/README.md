# Cross-project path query blocked before timing

The original indexer's actual stdout/stderr were not retained before the fresh capture was deleted; `indexer.*.txt` here are capture subprocess output. Original index completeness is unverified and the missing logs cannot be recovered. The retained search responses are still the observed reason this query was blocked.

This independent 2026-11-01 run used the same public 3,969-byte corpus and original/Rust endpoints as the adjacent strict no-hit experiment. The query `path=main.c&projects=alpha&projects=beta&start=0&maxresults=10` returned two paths from each endpoint, but the original ordered alpha before beta and the Rust scan probe ordered beta before alpha. Both raw preflight bodies and relevant headers are retained in `run/`.

The strict fairness gate returned `BLOCKED_OUTPUT_DIFFERENCE`; no warmup or timed samples were recorded. This is one index generation and query, not a general statement about every cross-project query. `resources.json` is not a performance result for this blocked case. The original index was removed after its hash was recorded, so its exact document IDs cannot be reconstructed from this evidence alone.

Reproduction uses the command in the no-hit evidence with `--case cross-project` and a fresh external output directory. It rebuilds indexes, and original same-score path order may vary between index generations; retain each actual response without reordering.
