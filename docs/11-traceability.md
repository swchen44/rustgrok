# Rust v0.6：需求、設計、工作包與驗證對照

版本：**v0.6，proposed**。目前 **38項需求、44個Rust工作包、80個測試規格**已建立關聯。所有執行狀態仍為 **planned／not_run**；這是規劃結構完整性檢查，不是產品相容性或性能驗收。

需求與決策狀態見[需求書](02-requirements.md)；元件及流程見[設計書](03-system-design.md)；工作包見[實作計畫](04-implementation-plan.md)；測試方法見[驗證計畫](05-verification-plan.md)，具體steps與expected見[測試矩陣](../tests/matrix.json)。機器可讀完整關聯見[traceability.json](../tests/traceability.json)。

## 1. 映射原則與現行修正

- 保留原需求與測試ID；新增FR-021、FR-022、NFR-016及RST-001～012，Rust重寫不代表縮減原版功能或style。
- 原版Java/OpenGrok只作獨立oracle。工作包映射已改為Rust的parser/analyzer/engine/SSR/index/native封裝內容。
- `test_ids`從matrix的`req_ids`反向產生；工作包按04目前內容映射，所有被引用WP均實際存在。
- `planned`描述實作／驗證執行狀態，不取代SRS中的CONFIRMED、PROPOSED、OPEN或CONDITIONAL。
- 下表是需求層級關聯；每張Issue／PR仍須列其直接實作步驟、採用測項及exact artifact證據，不能把某需求的全部測試都當單一WP完成條件。

| 需特別避免的舊映射 | v0.6正確關聯 |
|---|---|
| FR-021 Rust核心 | WP-C01、WP-C05、WP-C07、WP-C08、WP-C09 |
| FR-022 相容parser／AST／engine adapter | WP-B02、WP-C08、WP-C09、WP-D01 |
| NFR-016 公平效能與收益證據 | WP-B07、WP-E07、WP-E09 |
| NFR-011 logs／diagnostics | WP-C05、WP-C07、WP-E08；C09現為搜尋／scoring／snippet，不能沿用舊logs對應 |
| 原生平台／離線spike | 以WP-B04為主；B02／B03現為query與SSR相容性 |

## 2. 需求 → 工作包 → 測試

| 需求 | 工作包 | 測試ID | 執行狀態 |
|---|---|---|---|
| FR-001 | WP-B02, WP-C08, WP-C09, WP-D01 | REF-002, QRY-001, QRY-002, RST-002, RST-003 | planned；未執行 |
| FR-002 | WP-B02, WP-C08, WP-D01 | QRY-002, QRY-003, QRY-004, QRY-005, RST-002, RST-007 | planned；未執行 |
| FR-003 | WP-B02, WP-C09, WP-D01 | QRY-006, PRJ-001, API-003, RST-004, RST-005 | planned；未執行 |
| FR-004 | WP-B03, WP-C09, WP-D02 | UI-002, UI-003, OS-001, WIN-001, RST-005 | planned；未執行 |
| FR-005 | WP-B03, WP-C03, WP-C10, WP-D02, WP-D06 | SRC-006, UI-001, UI-002, UI-005, SEC-003, RST-005 | planned；未執行 |
| FR-006 | WP-A05, WP-B05, WP-D03 | PRJ-005, UI-004, SCM-001, SCM-002, SCM-003, OS-001, WIN-001 | planned；未執行 |
| FR-007 | WP-A03, WP-B02, WP-C06, WP-C09, WP-D01 | PRJ-001, PRJ-002, PRJ-003, PRJ-005, UI-001, PERF-002, RST-004 | planned；未執行 |
| FR-008 | WP-A05, WP-B02, WP-C07, WP-C08, WP-D03 | REF-003, SRC-001, RST-003, RST-007 | planned；未執行 |
| FR-009 | WP-B06, WP-C07, WP-D07 | REF-002, SRC-001, SRC-002, SRC-003, SRC-004, SRC-005, SRC-006, PRJ-002, PRJ-003, OS-005, REC-001, SEC-003, PERF-002, WIN-001, SYN-001, API-004, RST-009 | planned；未執行 |
| FR-010 | WP-A05, WP-D04 | QRY-007, OS-001 | planned；未執行 |
| FR-011 | WP-A05, WP-C02, WP-C05, WP-D04 | REF-003, SCM-002, API-001, API-002, API-003, SRV-001, API-004, RST-011 | planned；未執行 |
| FR-012 | WP-A06, WP-B03, WP-C10, WP-D02 | REF-001, REF-003, UI-001, UI-002, UI-003, UI-004, UI-005, UI-006, SRV-001, RST-005 | planned；未執行 |
| FR-013 | WP-B01, WP-C05, WP-C10 | REF-002, PKG-001, PKG-004, OS-001, SRV-001, RST-006 | planned；未執行 |
| FR-014 | WP-C04, WP-C05, WP-E08 | OS-002, CLI-001 | planned；未執行 |
| FR-015 | WP-C06, WP-C07, WP-C10 | PRJ-004, WIN-001, CLI-001, API-004 | planned；未執行 |
| FR-016 | WP-C02, WP-C03, WP-E04 | SRC-004, PRJ-004, PKG-005, REC-001, REC-002, REC-004, SEC-001, DAT-001, MIG-001, RST-009 | planned；未執行 |
| FR-017 | WP-A01, WP-C05, WP-D05, WP-D06 | API-002, SEC-002, SEC-005 | planned；未執行 |
| FR-018 | WP-B04, WP-C04, WP-E02, WP-E03, WP-E06 | PKG-002, PKG-003, SEC-004 | planned；未執行 |
| FR-019 | WP-A03, WP-C06, WP-C07 | SYN-001 | planned；未執行 |
| FR-020 | WP-A01, WP-C02, WP-D04, WP-E04 | REC-003, REC-004, MIG-001, RST-011 | planned；未執行 |
| FR-021 | WP-C01, WP-C05, WP-C07, WP-C08, WP-C09 | REF-001, REF-002, REF-003, SRC-001, QRY-007, PRJ-003, UI-001, UI-002, UI-006, SCM-002, API-001, PKG-001, PKG-002, PKG-003, PKG-004, PKG-006, OS-001, OS-002, REC-001, REC-002, REC-003, PERF-001, PERF-002, WIN-001, SRV-001, MIG-001, REL-004, API-004, RST-001, RST-005, RST-006, RST-008, RST-009, RST-011, RST-012 | planned；未執行 |
| FR-022 | WP-B02, WP-C08, WP-C09, WP-D01 | REF-003, QRY-001, QRY-002, QRY-003, QRY-004, QRY-005, QRY-006, PRJ-001, API-001, API-003, RST-001, RST-002, RST-003, RST-004, RST-007, RST-008 | planned；未執行 |
| NFR-001 | WP-A02, WP-B04, WP-C03, WP-E06 | SRC-006, UI-006, PKG-001, PKG-005, PKG-006, OS-001, OS-002, OS-003, OS-004, OS-005, WIN-001, SRV-001, RST-006 | planned；未執行 |
| NFR-002 | WP-B04, WP-E01, WP-E06, WP-E10 | PKG-001, PKG-002, PKG-005, REL-001, REL-002, DAT-001, RST-006 | planned；未執行 |
| NFR-003 | WP-B04, WP-E03, WP-E06 | PKG-003, REL-001, REL-002, RST-006 | planned；未執行 |
| NFR-004 | WP-A02, WP-B04, WP-C03, WP-C04, WP-E06 | PKG-004, OS-003, OS-004 | planned；未執行 |
| NFR-005 | WP-A03, WP-B07, WP-E07, WP-E09 | PRJ-002, PERF-001, PERF-002, RST-008, RST-010, RST-012 | planned；未執行 |
| NFR-006 | WP-A03, WP-B07, WP-E07, WP-E09 | PRJ-002, PERF-001, PERF-002, RST-008, RST-010, RST-012 | planned；未執行 |
| NFR-007 | WP-B06, WP-C05, WP-C07, WP-D07, WP-E04 | SRC-005, PRJ-003, OS-002, REC-001, REC-002, REC-004, API-004, RST-009 | planned；未執行 |
| NFR-008 | WP-C03, WP-D05, WP-D06 | SRC-004, SRC-006, QRY-005, API-002, API-003, SEC-001, SEC-002, SEC-003, SEC-004, SEC-005, SYN-001, OBS-001, RST-007 | planned；未執行 |
| NFR-009 | WP-A04, WP-C01, WP-C04, WP-E05, WP-E10 | REF-001, PKG-006, REL-001, REL-004, REL-005 | planned；未執行 |
| NFR-010 | WP-A04, WP-E05 | REL-001, REL-004 | planned；未執行 |
| NFR-011 | WP-C05, WP-C07, WP-E08 | SCM-003, REC-002, CLI-001, OBS-001 | planned；未執行 |
| NFR-012 | WP-C02, WP-C03, WP-D07, WP-E04 | OS-005, REC-003, REC-004, REL-002, DAT-001, MIG-001, RST-009, RST-011 | planned；未執行 |
| NFR-013 | WP-C01, WP-E10 | REL-003, REL-005 | planned；未執行 |
| NFR-014 | WP-A01, WP-A05, WP-A06, WP-A07, WP-A08, WP-B08, WP-D08, WP-E08, WP-E10 | REF-001, REF-002, REF-003, REL-002, REL-003, OBS-001, REL-005, RST-001, RST-011 | planned；未執行 |
| NFR-015 | WP-B04, WP-E02, WP-E03, WP-E06 | PKG-002, PKG-003, REL-002, RST-006 | planned；未執行 |
| NFR-016 | WP-B07, WP-E07, WP-E09 | REF-001, SRC-001, PERF-001, PERF-002, RST-008, RST-010, RST-012 | planned；未執行 |

## 3. 44個工作包的需求反向對照

這張表用來找孤立工作包及變更影響；不是實作完成表。測試關聯由上一節需求與matrix追蹤，不以名稱相似推定功能已完成。

| 工作包 | 目前工作內容 | 相關需求 |
|---|---|---|
| WP-A01 | 記錄Rust/A/搜尋/雙模式無ACL/Mac先驗/Ubuntu amd64/rustgrok；確認剩餘決策 | FR-017, FR-020, NFR-014 |
| WP-A02 | 確認OS/CPU/browser、最低版本、registry與權限 | NFR-001, NFR-004 |
| WP-A03 | 收source布局/語言/SCM/規模/性能痛點與更新模式 | FR-007, FR-019, NFR-005, NFR-006 |
| WP-A04 | 鎖定OpenGrok source/assets與Rust候選庫研究版本 | NFR-009, NFR-010 |
| WP-A05 | 盤點routes、queries、analyzers、SCM、plugins與工具 | FR-006, FR-008, FR-010, FR-011, NFR-014 |
| WP-A06 | 建立獨立原Java/OpenGrok參照服務 | FR-012, NFR-014 |
| WP-A07 | 建立deterministic synthetic corpus/history | NFR-014 |
| WP-A08 | 建分層差異harness與normalization白名單 | NFR-014 |
| WP-B01 | 最小Rust CLI/HTTP/search native縱切試驗 | FR-013 |
| WP-B02 | 比較query/analyzer/BM25與selected-project統計 | FR-001, FR-002, FR-003, FR-007, FR-008, FR-022 |
| WP-B03 | 原版search/xref頁移植Rust模板試驗 | FR-004, FR-005, FR-012 |
| WP-B04 | macOS先做native/npm/ZIP，準備Ubuntu amd64及Windows適配契約 | FR-018, NFR-001, NFR-002, NFR-003, NFR-004, NFR-015 |
| WP-B05 | source映射與SCM/ctags候選adapter | FR-006 |
| WP-B06 | bounded pipeline、writer locks、commit/reload/cancel實驗 | FR-009, NFR-007 |
| WP-B07 | 早期搜尋延遲與資源tradeoff量測 | NFR-005, NFR-006, NFR-016 |
| WP-B08 | 整合G1與選Rust技術棧/範圍基線 | NFR-014 |
| WP-C01 | 建立Cargo workspace/lock/toolchain及npm封裝目錄 | FR-021, NFR-009, NFR-013 |
| WP-C02 | config/metadata schema、import與migration契約 | FR-011, FR-016, FR-020, NFR-012 |
| WP-C03 | native平台路徑、workspace、locks與payload cache | FR-005, FR-016, NFR-001, NFR-004, NFR-008, NFR-012 |
| WP-C04 | runtime/helper resolver與doctor | FR-014, FR-018, NFR-004, NFR-009 |
| WP-C05 | Rust serve/start/status/stop與程序控制 | FR-011, FR-013, FR-014, FR-017, FR-021, NFR-007, NFR-011 |
| WP-C06 | 多project registry與source映射 | FR-007, FR-015, FR-019 |
| WP-C07 | Rust掃描/增量/分析工作佇列與generation publisher | FR-008, FR-009, FR-015, FR-019, FR-021, NFR-007, NFR-011 |
| WP-C08 | Rust query-compat與analyzers實作 | FR-001, FR-002, FR-008, FR-021, FR-022 |
| WP-C09 | search engine adapter/scoring/snippet與xref基本實作 | FR-001, FR-003, FR-004, FR-007, FR-021, FR-022 |
| WP-C10 | 組裝原style Rust SSR與完整真包垂直流程 | FR-005, FR-012, FR-013, FR-015 |
| WP-D01 | 完整query/fields/ranking/highlight/error矩陣 | FR-001, FR-002, FR-003, FR-007, FR-022 |
| WP-D02 | 全頁UI/xref/keyboard/raw/permalink/history views | FR-004, FR-005, FR-012 |
| WP-D03 | 全部承諾language/SCM/history/annotate/diff | FR-006, FR-008 |
| WP-D04 | suggester/REST/配置/plugins與工具契約 | FR-010, FR-011, FR-020 |
| WP-D05 | 本機＋LAN共用讀者路由與D16控制面 | FR-017, NFR-008 |
| WP-D06 | parser/renderer/FFI/path/process安全負測 | FR-005, FR-017, NFR-008 |
| WP-D07 | 索引中讀取、source變動、取消/crash/recovery | FR-009, NFR-007, NFR-012 |
| WP-D08 | G3 feature與parity審查 | NFR-014 |
| WP-E01 | 產生npm薄launcher與平台native packages | NFR-002 |
| WP-E02 | 完整offline tgz依賴閉包 | FR-018, NFR-015 |
| WP-E03 | 直接native ZIP與平台啟動器 | FR-018, NFR-003, NFR-015 |
| WP-E04 | 新schema/generation升級、備份與rollback | FR-016, FR-020, NFR-007, NFR-012 |
| WP-E05 | Cargo/npm/native/移植來源SBOM/license/checksums | NFR-009, NFR-010 |
| WP-E06 | Mac完整階段後Ubuntu amd64及全部承諾tuple原生重驗 | FR-018, NFR-001, NFR-002, NFR-003, NFR-004, NFR-015 |
| WP-E07 | 完整公平性能/資源/並行量測與優化 | NFR-005, NFR-006, NFR-016 |
| WP-E08 | 使用/安裝/升級/故障與診斷手冊 | FR-014, NFR-011, NFR-014 |
| WP-E09 | 使用者內網代表資料pilot與soak | NFR-005, NFR-006, NFR-016 |
| WP-E10 | GitHub tag/Release與npm交接 | NFR-002, NFR-009, NFR-013, NFR-014 |

## 4. 結構檢查結果與實測界線

| 檢查 | 本版結果 |
|---|---|
| SRS需求數 | 38：FR-001～022、NFR-001～016 |
| 每項需求有WP與test | 是；未決需求仍保留 |
| 工作包存在且無孤立 | 44／44具需求關聯，引用ID均存在於04 |
| 測試規格有需求關聯 | 80／80，含原68與新增12 Rust測項 |
| 設計文件路徑 | 關聯中的文件路徑已檢查存在 |
| 產品實作／驗收結果 | **未執行**；沒有相容性／性能PASS證據 |

D19必要工具隨包、D21搜尋優先已確認；D19具體build、D20相容範圍、D09/D21數值與其他OPEN／CONDITIONAL需求仍待決。不能因測試環境缺工具、原版Windows跳過測試、Rust引擎能力差異或性能不理想而自動排除。

需求或工作包調整時，先改SRS／決策／設計，再更新04、matrix、此對照與JSON。新增或刪除test也要重建反向關聯。G4發行可靠性不等於G5使用者pilot驗收；G6交接時須把這些規劃路徑連到實際commit、artifact hashes、測試報告及批准的例外。

搜尋優先的設計與量測契約見[14-search-performance-plan.md](14-search-performance-plan.md)；它細化既有PERF-001、PERF-002、RST-008、RST-010、RST-012，不增加測項數量。

D02已確認雙模式約5人、不用權限管理。FR-017及相關SEC/API測項改驗共同讀取與已決管理邊界，不將不需要的RBAC列成永遠blocked。Ubuntu24.04 amd64主要部署與Mac先開發驗證已定，Mac／Windows細節仍待D03；M1／arm64與Public swchen44/rustgrok已確認，repo已建立，D11已決。實作與產品測試仍未執行。
