# FULL sigma admission review fix

The prior [FULL evidence](../full-analysis-2026-11-04/README.md) remains the original capture and test record for commit `9541fcd`; its Rust test receipt does not describe this fix. Its finite original 81-context oracle remains unchanged and is reused by the current integration test.

The review identified a contract gap: `AAΣ` and `ΣAA` were accepted despite the documented at-most-one-character bound on each side of uppercase sigma. The original and Rust outputs for the reviewer's finite sample agreed; this finding is an admission-boundary violation, not a demonstrated term mismatch. `red.stdout`, `red.stderr`, and `red-receipt.json` record the first public API test failing on `AAΣ` before the fix. The hash-named test binary is retained locally at the receipt path.

`rust-test-receipt.json` binds the subsequent full analysis correctness suite to the final source and test hashes, raw `analysis.stdout.txt` / `analysis.stderr.txt`, the 2.5 GiB sampled reserve, and the retained actual test binary. It covers both asymmetric boundary examples, multiple sigma, an unsupported context, existing 81 original oracle cases, and the independent query-normalization contract. No additional oracle or performance run was performed. The CDDL/MIT provenance and FULL reader/indexing limitations remain as described in the prior evidence.
