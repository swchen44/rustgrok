# 原版資產與 Rust templates 規劃

依使用者指定的 OpenGrok 基線建立 assets/upstream，逐檔保留原 CSS／JS／images／fonts、hash、來源與 notices。研究基線不直接代替正式D04。

templates 保存 JSP 動態邏輯的 Rust 移植，保留原版DOM、URL、JS globals與操作契約。此處尚未複製正式產品資產，也沒有UI實作；詳見 [Rust相容性研究](../docs/12-rust-compatibility.md)。
