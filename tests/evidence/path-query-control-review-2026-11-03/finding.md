## P2: accepted form-feed syntax changes Boolean query semantics

Feature: 89f9ae6a21d51d848977852ab5c6705dd3724900. Integration remains 6436e6f34316f1553eff65a2237982269226d8bb. No merge or source edit.

`crates/path-index/src/query.rs:52` skips U+000C via `is_ascii_whitespace`, then classifies AND/OR/NOT as operators. Pinned OpenGrok 1.14.20 QueryBuilder treats the operator followed immediately by U+000C as an analyzed literal term. On the same five-file synthetic corpus:

| Query as JSON | Original HTTP | Rust CLI |
|---|---|---|
| `"alpha AND\u000cbeta"` | 200, 0 hits | exit 0, 1 hit |
| `"alpha OR\u000cbeta"` | 200, 0 hits | exit 0, 4 hits |
| `"alpha NOT\u000cbeta"` | 200, 0 hits | exit 0, 2 hits |

Plain-space `alpha AND beta` and `alpha OR beta` agree. See `comparison.json`, raw `original-http/parse.stdout.txt`, `original-http/http-00.body` through `http-04.body`, `rust-query-*.stdout`, and `receipt.json` for commands, exact inputs, binary identity, pinned tool hashes and source-byte equality. Original Tomcat terminated and was reaped before Rust subprocess execution.

Conservative correction: reject form feed within the current supported subset, or independently establish the exact pinned lexical boundary before accepting it. Add API/subprocess regressions for all three reserved words followed by U+000C; normalizing it to ordinary whitespace produces false successes. Shared parser also feeds host expressions, but no extra host runtime test was run after this material direct-CLI defect.

Feature/integration/issue7 files and Git states are unchanged; pre-existing cached binary bytes are unchanged. New reviewer build is retained under user cache by SHA. Full suites, fmt/clippy, evidence contracts and package checks were not run after the confirmed difference. No benchmark or ranking claim.
