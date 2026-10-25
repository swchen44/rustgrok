# OpenGrok 相容搜尋產品

本詞彙表記錄本產品用於描述 OpenGrok 相容性的領域用語。

## Language

**OpenGrok 驗收基線**：
使用者選定、作為本產品功能、style 與排序比較標準的固定 OpenGrok 版本。
_Avoid_: 最新上游、未經確認的研究版本

**獨立參照 oracle**：
與 Rust 產品分開執行、提供驗收基線實際行為的原版 OpenGrok 參照系統。
_Avoid_: 產品 Java backend、Rust 核心替代品

**讀者操作**：
搜尋與瀏覽已登記專案內容的操作。
_Avoid_: 專案管理、設定變更

**管理操作**：
改變服務、專案登記、索引工作或設定狀態的操作，例如新增專案、重建索引及停機。
_Avoid_: 搜尋、瀏覽
