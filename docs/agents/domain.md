# Domain Docs

## 文件配置

採 single-context：
- GLOSSARY.md：全 repo 共用的領域詞彙。
- docs/adr/：架構決策紀錄。

## 閱讀規則

探索某個領域前，先讀 GLOSSARY.md 與相關 ADR。
仍須遵守 AGENTS.md 指定的 README、需求與交接文件閱讀順序。

詞彙表或相關 ADR 尚不存在時，繼續工作；
由 domain-modeling 在詞義或決策確定時建立，不預建空文件。

## 使用規則

Issue、設計、程式碼與測試使用詞彙表中的領域名稱。
詞彙表只記領域詞義，不放實作細節或待辦事項。

遇到與既有 ADR 不同的方向，明確指出衝突，
依使用者新答覆記錄變更並保留歷史。

docs/08-decision-register.md 記錄產品決策與未決事項。
ADR 記錄已形成、難以逆轉且具有實際取捨的架構決策。
候選技術與未驗證行為保留原有狀態，不寫成已採用或已通過。
