# Bounded Unicode path-analyzer observations

The independent Java wrappers use the pinned OpenGrok 1.14.20 `CompatibleAnalyser` and its release classpath. `oracle.json` contains 24 public synthetic inputs and the complete original token streams, including position, UTF-16 offsets, base64 UTF-8 terms and original final stream offsets. The last input is one token containing every scalar from U+4E00 through U+9FFF, in order; the original returned the same 20,992 scalars, with token offset 0–20,992. This is one exhaustive-sequence observation, not a proof of every possible surrounding context. Java binary, wrapper and class hashes, all classpath jar hashes and input/output hashes are recorded.

The other original inputs cover Traditional Chinese, `🦀` and `😀`, `İ` lowercase expansion, `Σ` at word boundaries, repeated delimiters and dot-separated tokens. Twenty of the 24 inputs are currently supported and compared completely. Four sigma tokens mixing punctuation, CJK or emoji are deliberately rejected even though their original outputs are saved; the rejected cases do not count as matching.

`sigma-contexts.stdout` is an independent original capture of every combination of zero or one cased character on either side of a single `Σ`: empty, ASCII A–Z/a–z, `İ`, `Α`, `α`, `Ο`, `ο`, `σ`, `ς`. All 3,600 rows include base64 input, position, original UTF-16 start/end and base64 output. The Rust example test compares every row; the receipt records the wrapper, executable, stdout and jar hashes. Longer sigma tokens, multiple uppercase sigmas and other context classes are explicitly unsupported. These finite observations do not establish general Java Locale.ROOT or all-Unicode parity.

Original UTF-16 offsets are retained even when leading or repeated slashes or lowercase expansion make them differ from actual source boundaries. Rust also returns separate UTF-8 byte spans; tests verify the exact original substrings, including non-BMP and expanded terms. Controls, unverified characters and unverified sigma contexts return an error, not partial tokens.

Reproduction with the previously verified JDK and original release libraries (`$jdk`, `$lib`, and an empty `$classes` directory):

```sh
"$jdk/bin/javac" -cp "$lib/*" -d "$classes" tests/evidence/path-analyzer-2026-11-03/PathTokenOracle.java tests/evidence/unicode-path-2026-11-03/SigmaContextOracle.java
"$jdk/bin/java" -cp "$classes:$lib/*" SigmaContextOracle
cargo test --manifest-path crates/analysis/Cargo.toml --locked --offline
python3 -m unittest discover -s crates/analysis/tests -p test_unicode_path_oracle.py -v
python3 -m unittest discover -s tests/contract -p test_unicode_path_evidence.py -v
```

Only analyzer correctness is addressed. This does not establish complete search, indexing, ranking, generation, HTTP, browser behavior, performance or platform acceptance.
