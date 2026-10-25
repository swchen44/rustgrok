# 搜尋效能規劃

D21已確認搜尋速度優先，D19已確認必要工具隨包。此目錄目前只有[benchmark manifest模板](search-benchmark-manifest.template.json)，沒有產品benchmark程式或量測結果。

執行方式、查詢分組、效能分層與公平比較契約見[14搜尋效能設計](../../docs/14-search-performance-plan.md)。對應既有PERF-001、PERF-002、RST-008、RST-010、RST-012；模板中的null不代表0，也不代表可以略過。

每個run使用實際artifact、corpus、query inventory、原版oracle與環境hash，保留所有raw samples和失敗。私有source不加入GitHub。

D02已確認local＋LAN、約5人、無角色／project ACL；主要部署Ubuntu24.04 amd64已確認；先Mac開發及完整驗證，再Ubuntu原生重驗。首輪Mac已定M1／arm64，macOS版本仍待D03確認；原生arm64與Rosetta結果分列。5人是互動情境，不是固定QPS或HTTP並行上限。
