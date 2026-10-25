# 測試語料尚待建立：Rust v0.6

依 [驗證計畫](../../docs/05-verification-plan.md) 建立可公開、可重建的 synthetic corpus。[corpus-manifest.template.json](corpus-manifest.template.json) 是規格模板，尚無實際 source/history 或 PASS 結果。

除了多專案、完整語言／SCM inventory，Rust 重寫需涵蓋長 identifiers、defs/refs 大小寫、稀疏欄位、跨 project statistics、phrase positions、Unicode UTF-8／UTF-16 spans、xref 行號、merge/rename/blame 與無效輸入。預期答案取自獨立固定版本原版；不能由 Rust parser 同時產生 input 和 expected output。

不同 OS 不允許的檔名與大小寫衝突要有各自預期，不得拿無法建立的 fixture 當所有平台同一輸入。企業 source、索引、tokens 與憑證不加入此目錄。合成 fixtures 及引用內容也要保存來源授權。
