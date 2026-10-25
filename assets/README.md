# 原版資產與 Rust templates 規劃

依 D04 已確認的 OpenGrok 1.14.20／commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3 建立 assets/upstream，逐檔保留原 CSS／JS／images／fonts、hash、來源與 notices。基線選定不代表已複製或驗證資產。

templates 保存 JSP 動態邏輯的 Rust 移植，保留原版DOM、URL、JS globals與操作契約。此處尚未複製正式產品資產，也沒有UI實作；詳見 [Rust相容性研究](../docs/12-rust-compatibility.md)。
