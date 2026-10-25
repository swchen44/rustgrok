# 詳細實作計畫：Rust重寫

版本：v0.6；Rust、A、搜尋優先、local＋LAN約5人且無權限管理、Ubuntu24.04 amd64部署、macOS先開發驗證、rustgrok與GitHub Releases已確認；M1／arm64及Public已確認；macOS版本、精確依賴／其餘邊界／SLO待決。

## 1. 執行方法

先做最難的query/analyzer/ranking/UI與native分發實驗，再鋪完整語言與SCM。Rust已選定，不再把Java包裝當預設退路；如果Tantivy不符合原版契約，重評Rust搜尋adapter／engine方案或提出有證據的需求決策，不能悄悄放寬功能。

44個工作包保留WP-A01至WP-E10的結構，內容已改成Rust工作；舊Java計畫只保留歷史記錄。每包需有source、interfaces、實作、test IDs、artifact/benchmark證據及PR。prototype可以先用小corpus與功能子集，但不得以此交付宣稱完全同等，完整範圍仍依原要求。

## 2. Gates

| Gate | 主要產出 | 退出條件 |
|---|---|---|
| G0 | Rust決策、相容inventory、平台／source／工具／性能需求、固定oracle | 關鍵未知有owner；Rust/A/搜尋優先不重問，D19具體build與D20等剩餘邊界先處理 |
| G1 | AST/token/score/跨project/UI移植、原生binary與offline、早期公平benchmark | 最昂貴差異有證據與可行解；數據不足不得大量展開 |
| G2 | macOS先完成Rust安裝→project→index→HTTP搜尋→xref→stop垂直流程 | 真發行包、兩project、持久資料、Rust核心、原style基本頁 |
| G3 | 全query/UI/xref/history/analyzer/SCM/API與適用plugin相容（無RBAC需求） | mandatory功能逐項有證據，不偷偷縮減 |
| G4 | macOS完整階段後，Ubuntu24.04 amd64適配及完整native重驗；Windows承諾tuple驗收 | 每tuple的功能／UI／包裝／離線／故障／升級證據完整，依賴可重現；Mac階段完成不代替全域G4 |
| G5 | 使用者內網pilot與批准SLO | 正確性相同下查詢/索引/RSS等達成核准目標；使用者驗收 |
| G6 | rustgrok GitHub tag/Release、npm/tgz/ZIP、source/SBOM/checksum與交接 | 每項對上source與test結果；由使用者發布npm |

G1 benchmark只回答可行性及瓶頸，不等於全功能已較快。正式效能門檻由D09/D21決定。整體工期須在G1查清相容成本與規模後估算，不先寫看似精確的人日。

## 3. 工作包

現行順序是macOS先開發及完整驗證，再Ubuntu24.04 amd64平台實作／適配與完整原生重驗。Ubuntu是主要部署優先，不再等同第一個開發平台。工作包可依平台執行同一契約：先完成Mac的全feature、UI、bundled工具與真包測試，再進Linux完整階段；Linux差異若修改共用核心，要補Mac回歸。全域Gate仍以其全部承諾範圍為準，Mac完成不讓未驗Linux／Windows提前PASS。

### A：需求、原版與測試契約

| WP | 工作 | 產出與完成條件 | 依賴 |
|---|---|---|---|
| WP-A01 | 記錄Rust/A/搜尋/雙模式無ACL/Mac先驗/Ubuntu amd64/rustgrok；確認剩餘決策 | 更新SRS/決策，保留未決；不把Rust等同全純Rust或已接受排名差異 | 無 |
| WP-A02 | 確認OS/CPU/browser、最低版本、registry與權限 | Ubuntu24.04 amd64已定；首輪Mac已確認M1／arm64，確認macOS實際／最低版本與硬體，其後Linux原生環境及Windows矩陣 | WP-A01 |
| WP-A03 | 收source布局/語言/SCM/規模/性能痛點與更新模式 | corpus dimensions、D05–D09/D12及benchmark工作負載草案 | WP-A01 |
| WP-A04 | 鎖定OpenGrok source/assets與Rust候選庫研究版本 | 來源hash/授權台帳、oracle版本；候選庫不可浮動latest當release | WP-A01 |
| WP-A05 | 盤點routes、queries、analyzers、SCM、plugins與工具 | 每feature有Req/Test ID、原行為與待決相容層次 | WP-A04 |
| WP-A06 | 建立獨立原Java/OpenGrok參照服務 | 不共用Rust parser；同corpus能取得tokens/AST/results/UI與合理JVM設定 | WP-A02/WP-A04 |
| WP-A07 | 建立deterministic synthetic corpus/history | 同名跨project、稀疏fields、Unicode、rename/merge、錯誤與長token可重現 | WP-A05 |
| WP-A08 | 建分層差異harness與normalization白名單 | raw evidence、tokens/AST/order/DOM各層可比，不靠排序或遮罩掩蓋錯誤 | WP-A06/WP-A07 |

### B：高風險Rust可行性

| WP | 工作 | 產出與完成條件 | 依賴 |
|---|---|---|---|
| WP-B01 | 最小Rust CLI/HTTP/search native縱切試驗 | release binary可以完整小corpus建index/查詢/停止；不是只有Hello World | WP-A06/WP-A07 |
| WP-B02 | 比較query/analyzer/BM25與selected-project統計 | 預設AND、case、long token、offset、norm/IDF/ties的差異與修正可行證據 | WP-B01/WP-A08 |
| WP-B03 | 原版search/xref頁移植Rust模板試驗 | 原CSS/JS、DOM、Unicode行號/連結與互動比較；JSP無需產品端執行 | WP-B01/WP-A08 |
| WP-B04 | macOS先做native/npm/ZIP，準備Ubuntu amd64及Windows適配契約 | Mac先驗無JRE/Tomcat的Rust真包、native linkage、空cache與path/stop；Linux／Windows能力留待各自native證據 | WP-B01/WP-A02 |
| WP-B05 | source映射與SCM/ctags候選adapter | D19批准profile下比對JGit/原SCM，不假設Git CLI等價；跨磁碟/UNC條件 | WP-B01/WP-A03 |
| WP-B06 | bounded pipeline、writer locks、commit/reload/cancel實驗 | reader一致性、索引取消、crash與Windows檔案鎖可恢復；不假設abort取消blocking | WP-B01 |
| WP-B07 | 早期搜尋延遲與資源tradeoff量測 | 同硬體/等價subset、Rust release、JVMwarmup；先在Mac的1人/約5人互動、query類別/跨project/cold-warm/並行index延遲與queue，分報RSS/IO成本 | WP-B02/WP-B03/WP-B06 |
| WP-B08 | 整合G1與選Rust技術棧/範圍基線 | engine/renderer/helpers/storage方案、已知差異與估算；有owner的阻擋，不能默換功能 | WP-B02/WP-B03/WP-B04/WP-B05/WP-B06/WP-B07 |

### C：Rust正式模組與垂直流程

| WP | 工作 | 產出與完成條件 | 依賴 |
|---|---|---|---|
| WP-C01 | 建立Cargo workspace/lock/toolchain及npm封裝目錄 | apps/crates/xtask/tests/contract，cargo fmt/clippy/test可實跑；無空tests造綠燈 | WP-B08 |
| WP-C02 | config/metadata schema、import與migration契約 | 嚴格validation、revision、secrets、Java XML僅白名單匯入；後端選型有證據 | WP-C01 |
| WP-C03 | native平台路徑、workspace、locks與payload cache | 大小寫/Unicode/OS檔案ACL/同instance/partial解壓、binary/data分離 | WP-C02/WP-B04 |
| WP-C04 | runtime/helper resolver與doctor | Rust binary identity、隨包必要工具的版本/能力/shared libs；缺件明確拒絕，無隱含下載 | WP-C03/WP-B05 |
| WP-C05 | Rust serve/start/status/stop與程序控制 | readiness查generation/schema、結構化lifecycle logs、graceful shutdown、取消/timeout、沒有orphan | WP-C04/WP-B06 |
| WP-C06 | 多project registry與source映射 | 穩定ID、來源限制、add/remove、watch/sync授權邊界；不刪原始碼 | WP-C02/WP-C03 |
| WP-C07 | Rust掃描/增量/分析工作佇列與generation publisher | hash/mtime/rename策略、有界pipeline、writer互斥、source變動偵測、job/stage診斷 | WP-C05/WP-C06 |
| WP-C08 | Rust query-compat與analyzers實作 | typed AST、欄位/語言tokens/positions/offsets、API/UI參數區分與錯誤契約 | WP-B02/WP-C01 |
| WP-C09 | search engine adapter/scoring/snippet與xref基本實作 | 正確selected-project stats、排名/分頁、escaping與offset轉換，對照分層evidence | WP-C07/WP-C08 |
| WP-C10 | 組裝原style Rust SSR與完整真包垂直流程 | 兩project、index→search→xref→stop/restart；G2報告不冒充完整同等 | WP-C04/WP-C05/WP-C06/WP-C07/WP-C08/WP-C09 |

### D：完整同等與穩健性

| WP | 工作 | 產出與完成條件 | 依賴 |
|---|---|---|---|
| WP-D01 | 完整query/fields/ranking/highlight/error矩陣 | 原版differential含Boolean/wildcard/regex/fuzzy/range/boost與跨project；差異結案 | WP-C10 |
| WP-D02 | 全頁UI/xref/keyboard/raw/permalink/history views | 原DOM/classes/assets與各OS/browser screenshot；不改版、不藏差異 | WP-C10/WP-B03 |
| WP-D03 | 全部承諾language/SCM/history/annotate/diff | 優先語言深測＋完整inventory；JGit/其他SCM語意與dirty/rename/merge證據 | WP-C10/WP-B05 |
| WP-D04 | suggester/REST/配置/plugins與工具契約 | D16/D20決策逐項實作，不默刪Java插件能力也不默加JVM | WP-C10/WP-A05 |
| WP-D05 | 本機＋LAN共用讀者路由與D16控制面 | 無RBAC／project ACL；共同可讀契約、listen選擇與約5人使用；管理暴露依D16明確決策，style保留 | WP-C10 |
| WP-D06 | parser/renderer/FFI/path/process安全負測 | 不執行source、不shell injection、bounded regex、escaping、memory safety與fuzz regression | WP-C10 |
| WP-D07 | 索引中讀取、source變動、取消/crash/recovery | generation/index/xref/history/config契約、diskfull/kill/reader lock；故障不標ready | WP-C10/WP-B06 |
| WP-D08 | G3 feature與parity審查 | 全inventory有結果；upstream quirks與產品regression分清，未決不算PASS | WP-D01/WP-D02/WP-D03/WP-D04/WP-D05/WP-D06/WP-D07 |

### E：發行、效能與交接

| WP | 工作 | 產出與完成條件 | 依賴 |
|---|---|---|---|
| WP-E01 | 產生npm薄launcher與平台native packages | 精確版本與files allowlist、shim/argv/exit、missing optional payload診斷 | WP-C10/WP-B04 |
| WP-E02 | 完整offline tgz依賴閉包 | 空cache/無外網/ignore-scripts安裝，包含批准的helpers/shared-library前提 | WP-E01 |
| WP-E03 | 直接native ZIP與平台啟動器 | 不用Node/JRE/Tomcat仍可指定流程；native下載/quarantine/permissions/paths | WP-E01 |
| WP-E04 | 新schema/generation升級、備份與rollback | 二進位/資料成對相容，live source邊界，WindowsEXE/讀者鎖與卸載保留 | WP-D07/WP-E01 |
| WP-E05 | Cargo/npm/native/移植來源SBOM/license/checksums | 來源逐檔追蹤，換Rust不自動改CDDL授權；可重建metadata與對應source | WP-E01/WP-E02/WP-E03 |
| WP-E06 | Mac完整階段後Ubuntu amd64及全部承諾tuple原生重驗 | 真正packed artifacts，D19 bundled主profile；先Mac全部適用功能/UI/包裝，再Ubuntu24.04 amd64完整重驗並完成Windows矩陣；G5性能/pilot及G6交接另驗 | WP-D08/WP-E02/WP-E03 |
| WP-E07 | 完整公平性能/資源/並行量測與優化 | 相同功能下優先搜尋p50/p95/p99與各query組、timeout/error/queue；記RSS/CPU/IO/indexsize/freshness，SLO由D09/D21批准 | WP-D08/WP-E06 |
| WP-E08 | 使用/安裝/升級/故障與診斷手冊 | 可由未參與者照release包操作；敏感source/token不入一般logs | WP-E04/WP-E06 |
| WP-E09 | 使用者內網代表資料pilot與soak | 不外傳企業source，只交批准統計；達SLO與使用者驗收 | WP-E07/WP-E08 |
| WP-E10 | GitHub tag/Release與npm交接 | rustgrok Release的artifact/source/hash/tests對得上；可見性已定Public，下載回核對checksum；使用者自己發布npm | WP-E05/WP-E09 |

## 4. 失敗後的行動

- Query/ranking不可相容：保留raw/token/AST/stats差異，評估custom adapter/scorer或另一Rust引擎；向使用者提出具體tradeoff，不預設接受不同結果。
- 必要Ctags/SCM某平台無法隨包：先查build／依賴／授權與adapter替代，提出具體阻擋；不能無聲改為外裝要求、刪功能或連外下載。
- Windows不通：定位Rust/FFI/helper/path層，小修並回歸；不改成WSL驗收。
- Rust未改善性能：以profiling拆分解析/IO/SCM/index/query/HTTP，按D21目標優化；不能用語言選擇宣稱通過。
- 舊index/JAR/XML不相容：D10/D16/D20提供明確遷移／移植／bridge方案與成本，禁止silent fallback。

搜尋優先的實驗步驟與benchmark manifest見[14搜尋效能設計](14-search-performance-plan.md)；該文件細化既有工作包，不新增另一套WP或產品測試完成率。

## 5. 分工與變更控制

可並行做上游行為盤點、公開corpus、native包裝與license；token/AST/scoring/generation/view-model interfaces凍結後才大幅平行寫crates。每PR一個可驗收契約，附Req/WP/Test IDs、原版對照、性能影響、來源授權及migration。

變更engine、tokenizer、SCM adapter、模板、compiler/features、index schema都要找對應parity與benchmark受影響項。既有Rust方向不因普通技術失敗而自動改回Java包裝。

prototype階段允許小範圍實驗但必須明確標示；產品全功能完成與性能驗收才在G3/G5簽核。沒有先批准的精確時程或速度倍數。
