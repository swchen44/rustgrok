# 目前沒有產品測試證據

規劃結構檢查與產品驗收分開。本目錄保留開發環境觀測、oracle 取得檢核及未來產品驗收證據；**80 個完整測試規格仍為 planned**。

2026-11-01 的 [oracle 取得紀錄](oracle-acquisition-2026-11-01.json) 只證明列明的 source／archive 檢核範圍。下載未完成、部分步驟或靜態 inventory 均不代表 REF-001／REF-002／REF-003 整項 passed。

每次執行記錄 artifact/commit/platform/corpus/config hashes、steps、expected/actual、status、logs/screenshots/checksums。Rust執行需記Cargo.lock、rustc、release profile、CPU target、helpers與shared libraries；原版參照記JDK/OpenGrok/servlet container、heap、JIT warmup、Ctags/SCM與cache狀態。兩邊程序樹成本分列。

性能證據含raw samples、cold/warm條件、並行與query arrival model、正確性檢查及離群/失敗樣本。大型binary、profiles和screenshots放CI artifacts／Release並留下manifest，不能把企業source或secrets放入GitHub。
