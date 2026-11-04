# 測試語料尚待建立：Rust v0.6

依 [驗證計畫](../../docs/05-verification-plan.md) 建立可公開、可重建的 synthetic corpus。[corpus-manifest.template.json](corpus-manifest.template.json) 仍是完整規格模板；下方已建立有限公開搜尋資料與 Git history，但不代表完整規格或產品測試通過。

除了多專案、完整語言／SCM inventory，Rust 重寫需涵蓋長 identifiers、defs/refs 大小寫、稀疏欄位、跨 project statistics、phrase positions、Unicode UTF-8／UTF-16 spans、xref 行號、merge/rename/blame 與無效輸入。預期答案取自獨立固定版本原版；不能由 Rust parser 同時產生 input 和 expected output。

不同 OS 不允許的檔名與大小寫衝突要有各自預期，不得拿無法建立的 fixture 當所有平台同一輸入。企業 source、索引、tokens 與憑證不加入此目錄。合成 fixtures 及引用內容也要保存來源授權。

## 公開搜尋 corpus 的第一個有限切片

[公開來源與授權](public-source/README.md)提供C、Python、JavaScript三份固定上游輸入。`python3 tools/build_public_corpus.py --output /tmp/rustgrok-public-corpus` 建立兩個project、五個Git repositories（含一個普通巢狀Git）及固定history；既有目錄會被拒絕。這是小型搜尋corpus，不代表完整語言／SCM或容量驗收。原版兩輪搜尋輸出及重跑方式見[原始證據](../evidence/search-baseline-2026-11-01/README.md)。
