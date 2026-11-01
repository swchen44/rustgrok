# G1 搜尋候選：原始對照與排序診斷

日期：2026-11-01。Issue #6；WP-A08／WP-B01／WP-B02。**此為有限實驗，包含可重現差異；不是產品、效能或完整相容驗收。**

- [experiment.json](experiment.json) 記錄 Mac arm64、Rust/Cargo、候選 release binary、index、Ctags 和固定公開 corpus 的實際 hashes。候選 binary、暫存 index 與 corpus 的 Git 工作樹不進 repo；corpus 可用已版控的固定 manifest 重建。
- [candidate-first-five](candidate-first-five/comparison.json) 保存首次 release 對照，原始結果與 exit 1 的後續七案例對照保持分開；其五組 raw bytes 與後續同名案例一致。
- [candidate-seven](candidate-seven/comparison.json) 保存原版與 Rust CLI 原始 JSON 及每筆比對；[candidate-http-seven](candidate-http-seven/comparison.json) 保存同一 index、實際 loopback HTTP 的七筆對照，均 HTTP 200，harness exit 1。兩者同為：7/7 命中數、逐路徑行號相同；3/7 路徑次序相同，4/7 不同。只對原版 path-only 的空行號做明示投影；原始回應全部保留，沒有重排序。未比較 time、snippet、tag、完整原版 HTTP 契約或未列 query。
- [oracle-inputs.json](oracle-inputs.json) 是七筆原版輸入的逐筆來源／hash；前五筆來自 #5 的 `run-verified-1`，後兩筆來自本輪 [extra-oracle](extra-oracle/result.json)，同一固定 corpus manifest，原版用完整 JDK 啟動。兩筆補測分別為 Python definition 與 JavaScript reference；原始 stdout/stderr、HTTP 回應、config 與工具環境均保留。C/Python/JavaScript 各一個 defs／refs 單詞案例有覆蓋，並不代表這些語言完整相容。
- [full-alpha score](score-full-alpha/diagnostic.json) 為先前兩個保留 index 的只讀 Lucene 診斷：兩個 JS 文件在每輪的 score 相同、docID 次序與相應原始 HTTP 次序一致。索引 file hashes 是診斷當時取得，原始擷取未留下 capture-time index hashes；不能宣稱 index bytes 在兩時點經驗證相同。
- [其他四個 score](score-other-cases/diagnostic.json) 對 `run-verified-1` 的 full-alpha、defs-alpha、refs-beta、full-both 分別保存 query、原始 score/explain、docID、HTTP 與候選原始順序。其中 refs-beta 的 Python score `0x1.ad91a8p-4` 高於 C 的 `0x1.84549p-4`，原版 Python 在前、scan 候選 C 在前；候選未實作 scorer，不能將此差異當作 tie。full-both 同樣出現不同 relevance scores。不得對所有排序漂移推論同一原因。

兩個 Java 白箱診斷使用原版 `QueryBuilder`、指定 project 的 Lucene `MultiReader` 與 `TopScoreDocCollectorManager`；helper 採 `new IndexSearcher(reader)`，沒有原版 `RuntimeEnvironment` search executor，也不走完整 HTTP `SearchEngine`。本 helper 與原始 HTTP 的路徑順序在列出的案例相同，但這不證明其他 query、配置或索引 generations 同樣一致。helper source 與命令、原版 JAR／classpath hashes、JDK 身分與完整 stdout/stderr 皆在 `score-full-alpha/` 和 `score-other-cases/`；獨立新增的 Java wrapper 未因新 Rust/JS MIT 政策而被重新授權。`.class` 與原版二進位 index 不存 repo。

[實驗使用方式及缺項](../../../experiments/search-spike/README.md)另列單詞語法、JSON source 索引、無 BM25、原版 Ctags／tokenizer／UI 差異與平台限制。80 項完整產品測試維持 planned；不得把單一 corpus 或這些 probe 當成測試整項 PASS。
