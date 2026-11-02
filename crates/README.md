# Rust 模組

`analysis` 是第一個正式 Rust library 的有限 path tokenizer 切片，支援經原版核對的有限 ASCII／Unicode 子集，其他輸入明確回報 unsupported；詳細 API、證據與授權見 [analysis/README.md](analysis/README.md)。這不代表搜尋產品、完整 analyzer 或全部原版相容已完成。

已新增 [path-index](path-index/README.md) 的單 project 記憶體 postings 與程序內 generation 切換；來源掃描、持久化、排序及完整索引尚未實作。其餘規劃中的 core、query-compat、search、indexer、xref、history、web、platform 與 storage 模組及 apps/grok-host 尚未建立。模組責任和仍待凍結的契約見[系統設計](../docs/03-system-design.md)及 [Monorepo](../docs/07-monorepo-and-github.md)；搜尋引擎、HTTP、儲存及正式版本均未選定。
