# Rust 模組規劃

正式責任與目錄見 [系統設計](../docs/03-system-design.md) 及 [Monorepo](../docs/07-monorepo-and-github.md)。目前只有規劃，沒有可編譯 crate 或 Cargo.lock。

候選模組是 core、query-compat、analysis、search、indexer、xref、history、web、platform、storage；由 apps/grok-host 組裝。contracts 凍結後再建立有實際用途的 crates，避免只有空目錄或空測試。Rust 核心方向已定，第三方依賴與拆分粒度依 G1 證據。
