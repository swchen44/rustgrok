# 目前沒有產品測試證據

規劃結構檢查與產品驗收分開。本目錄只定義未來 evidence 的存放位置；**80 個測試規格仍為 planned**。

每次執行記錄 artifact/commit/platform/corpus/config hashes、steps、expected/actual、status、logs/screenshots/checksums。Rust執行需記Cargo.lock、rustc、release profile、CPU target、helpers與shared libraries；原版參照記JDK/OpenGrok/servlet container、heap、JIT warmup、Ctags/SCM與cache狀態。兩邊程序樹成本分列。

性能證據含raw samples、cold/warm條件、並行與query arrival model、正確性檢查及離群/失敗樣本。大型binary、profiles和screenshots放CI artifacts／Release並留下manifest，不能把企業source或secrets放入GitHub。
