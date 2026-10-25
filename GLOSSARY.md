# OpenGrok 相容搜尋產品

本詞彙表記錄本產品用於描述 OpenGrok 相容性的領域用語。

## Language

**Project（專案）**：
使用者在服務中登記並作為搜尋選取範圍的來源集合；一個 project 可以包含多個或巢狀的版本控制 repository。
_Avoid_: 單一 Git repo、repository 的同義詞

**Repository（版本控制儲存庫）**：
有自身版本歷史及版本控制邊界的來源儲存庫；project 的範圍與 repository 的邊界各自描述。
_Avoid_: project 的同義詞

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

**本機管理網頁**：
只供服務主機本機瀏覽器使用的管理介面；LAN 使用者的搜尋／瀏覽頁面屬讀者介面。
_Avoid_: LAN 管理網頁、一般讀者設定頁
