# 更新期間維持搜尋，完整就緒後整批發布

狀態：accepted；2026-10-26；決策 D12，使用者回覆「Q8 Yes」。

使用者選擇索引更新期間持續提供上一個成功版本的搜尋，等新索引與相關資料完整就緒後再整批切換，接受期間搜尋上一版內容及保留新舊資料的磁碟成本。這保留更新期間的搜尋可用性，必要component未就緒或更新失敗時不得把部分新資料發布為成功。

## 邊界與後果

需量測source變更至發布的延遲，以及新舊資料保留的磁碟峰值；新鮮度、磁碟、RTO/RPO與其他數值門檻仍未決。Q9之後的閱讀規則已確認沿原版；引擎、儲存／journal、建置用source snapshot及跨project發布scope仍未凍結，首次無成功generation與異常回復需有明確狀態。原版功能與排序要求保留，不以延後資料更新製造未批准的效能改善。

[固定原版來源閱讀研究](../research/2026-10-26-opengrok-source-view-consistency.md)顯示命中、snippet、xref及raw有不同來源分支。後續Q9使用者回覆「1」，選沿固定原版live source／SCM規則處理snippet／xref／raw／download，不把全部閱讀綁定搜尋版本快照。索引建置期間來源變動與內部機制仍須驗證，靜態source不代表runtime通過。

追溯：FR-009、NFR-005、NFR-007、NFR-012、NFR-016；既有 SRC-002、PRJ-002、REC-001、REC-002、RST-009。實作與所有產品驗證仍 NOT_RUN；完整對照見[決策表](../08-decision-register.md)、[系統設計](../03-system-design.md)與[測試規格](../../tests/matrix.json)。
