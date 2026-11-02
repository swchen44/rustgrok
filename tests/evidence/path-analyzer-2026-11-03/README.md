# Path tokenizer correctness observations, 2026-11-03

The independent `PathTokenOracle.java` invokes the pinned OpenGrok 1.14.20 `CompatibleAnalyser` with field `path`; the product crate does not use Java. The source checkout is commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`. `oracle.json` contains all 14 public synthetic inputs, each original stdout/stderr and exit code, the source and wrapper hashes, compiled wrapper hash, Java binary hash and SHA-256 for each release classpath jar. Each stdout line is position, UTF-16 start, UTF-16 end and base64-encoded UTF-8 token; `END` records the original final stream offsets. The Rust API does not yet expose the end-of-stream offset.

The retained observations include empty input, leading/repeated slashes, leading/trailing/repeated dots, spaces, a 100-character token, an astral character and lowercasing expansion. Upstream offsets are not always source byte spans. The crate deliberately rejects the two Unicode inputs and all other non-printable-ASCII inputs; they are not counted as matching tokenizer output. Twelve printable-ASCII inputs match original term, position and recorded offsets in `test_path_oracle.py`. These finite cases do not establish complete analyzer, search, ranking or product compatibility.

To reproduce on the previously verified Mac oracle, set `jdk` to the verified JDK 21 home, `lib` to `opengrok-1.14.20/lib`, and compile the wrapper into an isolated empty directory:

```sh
"$jdk/bin/javac" -cp "$lib/*" -d "$classes" tests/evidence/path-analyzer-2026-11-03/PathTokenOracle.java
"$jdk/bin/java" -cp "$classes:$lib/*" PathTokenOracle L0EvQi5j
python3 -m unittest discover -s crates/analysis/tests -p test_path_oracle.py -v
```

The example argument decodes to `/A/B.c`; the JSON retains all captured inputs and streams. Only correctness tests were run. No new performance measurement, G1/G2 acceptance, production indexing or full product test is implied.
