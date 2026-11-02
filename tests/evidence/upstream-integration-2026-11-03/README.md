# Original path tokenizer test scenario

The fixed upstream `PathTokenizerTest.testIncrementToken` uses `alpha/beta/gamma/delta.ext` and stops checking on the dot. The independent `PathTokenOracle.java` from the earlier path-analyzer evidence was compiled against the verified original 1.14.20 classpath; `path-tokenizer.stdout` and `.stderr` are unmodified process outputs. The complete stream contains `alpha`, `beta`, `gamma`, `delta`, `.`, and `ext`; its final `END` offset is retained but the current product API does not expose it. The input, original raw stream, wrapper class/source, Java binary and classpath hashes are in `path-tokenizer-receipt.json`.

Reproduce with a verified JDK and release libraries:

```sh
"$jdk/bin/javac" -cp "$lib/*" -d "$classes" tests/evidence/path-analyzer-2026-11-03/PathTokenOracle.java
"$jdk/bin/java" -cp "$classes:$lib/*" PathTokenOracle YWxwaGEvYmV0YS9nYW1tYS9kZWx0YS5leHQ=
cargo test --manifest-path crates/analysis/Cargo.toml --locked --offline --test upstream_path_integration
python3 -m unittest discover -s tests/contract -p test_upstream_integration_evidence.py -v
```

`source-provenance.json` records seven pinned upstream test sources and the independently authored path fixture. The Rust integration test is CDDL-scoped as an adaptation of the original test; see [analysis license notes](../../../crates/analysis/README.md). This is analyzer correctness evidence only, not product HTTP, browser e2e, performance or overall compatibility acceptance.
