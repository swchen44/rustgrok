# G1 搜尋候選：可執行但尚不相容

這是 Issue #6 的 Mac arm64 實驗，不是 Rustgrok 產品或已選定引擎。它建立一次性的 JSON 搜尋索引，使用指定的本機 Universal Ctags，並提供 CLI 與只綁 loopback 的 `/probe/search` HTTP 入口。資料由公開 corpus 建立；不用 Java 執行 Rust 候選。原版 OpenGrok／Java 只在獨立 oracle 與白箱診斷使用。

## 可重跑

先依 `tools/README.md` 取得並核實原版工具，再在 repo root 執行。`$probe_dir` 必須是新建的外部目錄；勿放企業 source。

```sh
probe_dir="$(mktemp -d -t rustgrok-g1)"
python3 tools/build_public_corpus.py --output "$probe_dir/corpus"
cargo build --release --offline --manifest-path experiments/search-spike/Cargo.toml
experiments/search-spike/target/release/rustgrok-search-spike index \
  --source "$probe_dir/corpus/source" --index "$probe_dir/index.json" \
  --ctags /opt/homebrew/bin/ctags
experiments/search-spike/target/release/rustgrok-search-spike search \
  --index "$probe_dir/index.json" --field full --query main --project alpha
python3 experiments/search-spike/differential.py \
  --binary experiments/search-spike/target/release/rustgrok-search-spike \
  --index "$probe_dir/index.json" \
  --oracle tests/evidence/search-spike-2026-11-01/oracle-inputs \
  --output "$probe_dir/diff"
```

測試 loopback 入口可啟動 `serve --index "$probe_dir/index.json" --listen 127.0.0.1:18080`，再以 GET `/probe/search?field=full&query=main&project=alpha` 查詢。這是實驗用介面，**不是**原版 `/api/v1/search` 的相容聲明；無管理 API、原版 UI 或正式 process lifecycle。

`differential.py` 也可在上述 server 啟動時用 `--http-base http://127.0.0.1:18080` 取代 `--binary`／`--index`，以 loopback HTTP 實際比對七筆；它不使用 proxy，也不允許將測試資料送到非 loopback 位址。原版與候選的 bytes 另存，並且不重排結果。僅將原版無行號的空字串投影為空行號清單，並比較 resultCount、有序路徑、各路徑行號；time、snippet、tag 和其他 HTTP 欄位不在此比較，原始回應保留。任一差異或候選錯誤使程式 exit 1，不代表全部原版能力已對照。

## 目前證據與阻擋

[七案例原始資料與比較](../../tests/evidence/search-spike-2026-11-01/README.md) 中，固定公開小 corpus 的七筆單詞查詢涵蓋 C／Python／JavaScript 各自 defs／refs；CLI 與實際 loopback HTTP 兩種比對均為 7/7 count 和逐路徑行號相同、3/7 有序路徑相同、4/7 不同。這是這次執行的測量，不是全功能、全部資料、分詞、HTTP、排名或效能 PASS。

實際限制：

- 僅支援單一 identifier 或有限 path；Boolean、phrase、regex、wildcard、fuzzy、range、escaping、多欄位組合、分頁與原版錯誤契約尚未實作。
- 全文／refs 以簡單字元切分，defs 用 Ctags JSON 與實驗選項，均不是 OpenGrok analyzer／Ctags protocol 的相容實作。除了本資料的七筆，其他語言和 SCM 的原版能力沒有被取消，尚待實作與驗證。
- 此 JSON 索引保存 source 全文，查詢時重新讀取與掃描；不是 inverted index，也不是正式效能設計。它只用於搜尋實驗，不提供閱讀用 source snapshot；raw/xref等閱讀規則仍依 Q9 未實作。
- 候選沒有 BM25 scorer、原版 field statistics、selected-project MultiReader scoring 或 docID tie 模型。原版 reference-beta 的 Python 命中分數高於 C，而候選按資料讀入次序輸出 C 在前；無法把該差異歸為 tie。不得硬編測試路徑、重排結果或把 tie 差異視作使用者同意。
- 此次 `full_alpha` 與另外三筆原版白箱 score/explain 診斷的 source、原始輸出、classpath hashes 與索引身分邊界見 [score evidence](../../tests/evidence/search-spike-2026-11-01/README.md)。helper 使用 `IndexSearcher(reader)` 而非原版帶 executor 的完整 HTTP 路徑；只描述實際案例，不外推全部 query。
- 本地 Ctags 並非已驗證的正式 bundled helper；沒有確認 clean 安裝、LAN、Windows、Ubuntu、效能、source 更新或 generation 行為。

Rust `src/main.rs` 為獨立新增實驗，MIT 條款依 [RUST-LICENSE-MIT.txt](RUST-LICENSE-MIT.txt) 僅作用於此 Rust 檔案；使用者於 2026-11-01 確認權利人名稱為 `swchen44`。依賴及其他 Python／Java 診斷程式不因此改授權。使用的原版 fixtures、JAR／WAR 仍按各自來源與條款記錄。

### Second candidate acquisition attempt

Tantivy 0.27.0 was investigated as a separate G1 candidate, not selected for the
product. `cargo info tantivy@0.27.0` reported Rust 1.88 minimum and MIT for that
crate; this machine has rustc 1.94.0. A two-job Cargo build with HTTP
multiplexing disabled downloaded several dependencies but did not finish after
more than eight minutes and was stopped. A subsequent offline build failed with
`failed to download libc v0.2.191`; a separate resumed download timed out, so
its incomplete bytes were not installed into the Cargo cache. This is an
acquisition result, not evidence that Tantivy is incompatible. No Tantivy build,
query, ranking, performance or dependency-license review passed. The unfinished
local prototype is outside the repository and is not a deliverable.

The final formatted scan source was rebuilt in release mode and compared again;
`candidate-final-cli` and `candidate-final-http` retain the resulting raw
outputs. Both compare seven cases: 7/7 equal counts and per-path line numbers,
3/7 equal path order, and exit 1. The index was reused from the earlier run;
its identity and the final source and binary hashes are in `experiment.json`.
