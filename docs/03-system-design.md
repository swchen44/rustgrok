# 系統設計書 SDD：Rust核心重寫

版本：v0.6；2026-10-25。**Rust、bundled helpers、搜尋優先、本機＋LAN約5人且無權限管理、Ubuntu24.04 amd64主要部署、macOS先開發驗證及GitHub rustgrok/Releases已確認；M1／arm64與Public已確認，macOS版本與具體選型仍待決。**

## 1. 產品邊界

搜尋、索引、query interpretation、Web與host管理用Rust實作。Java/OpenGrok放在獨立reference環境；不以Rust外殼啟動Java backend作重寫交付。

npm只作分發與啟動。若採通用package，薄JS launcher選平台、校驗payload並轉交argv/stdin/stdout/exit；server及index業務在native binary。ZIP可直接執行同一binary而不用Node。

原CSS/JS/圖示與可見操作盡量沿用；JSP包含Java動態邏輯，需移植view models與Rust templates。模板引擎未選，不自行改成SPA、換主題或增加首頁。

## 2. 元件圖

~~~mermaid
flowchart TB
  B["瀏覽器：原版style與操作"] --> H["Rust HTTP與原版路由"]
  C["Rust CLI"] --> M["專案與索引工作管理"]
  H --> V["Rust模板與原版靜態資產"]
  H --> Q["OpenGrok查詢相容層"]
  Q --> E["搜尋引擎adapter"]
  M --> A["Rust分析與xref pipeline"]
  A --> D["版本化索引與metadata"]
  E --> D
  A --> S["來源與SCM adapters"]
  H --> S
~~~

HTTP候選為Axum/Tokio、search候選為Tantivy。選型須經G1證據，不能從這張圖推論已有可執行產品。

## 3. 模組與契約

| 模組 | 契約 | 責任 |
|---|---|---|
| core | ProjectId、DocumentId、SourceRevision、IndexGeneration、QueryContext | 穩定domain types，不依賴engine document ID |
| query-compat | 原query/fields/projects → typed AST或相容錯誤 | Boolean、escaping、range/fuzzy/regex、boost、error contract |
| analysis | source/language/config → tokens/positions/offsets/symbols | 逐欄正規化、大小寫與語言行為 |
| search | AST/scoring scope → hits/count/order/snippets | adapter、scorer、project scope、分頁 |
| indexer | source manifest/changes → generation receipt | 掃描、增量、queue、writer鎖、取消、publish |
| xref | source/symbol spans → 安全HTML及navigation model | 行號、連結、CSS classes、escaping、offset轉換 |
| history | repo/revision/path → log/annotate/diff | 對照原版SCM，不自行pull/reset |
| web | 原route/request → HTML/API/errors | SSR、assets、原headers/JSON與auth邊界 |
| platform | OS/path/process/locks | native程序、檔案、共享庫與升級 |
| storage | config/projects/jobs/generations | transactional metadata、schema、crash恢復，實際KV/SQLite候選待定 |
| grok-host | CLI/serve | 組裝服務；常駐方式依D15 |

先凍結上述interfaces，再允許各模組並行開發。不得暴露Tantivy內部doc ID作長期URL，移動workspace不能造成public file identity不穩定。

## 4. Query、分析、排名

### 4.1 Query pipeline

原始HTTP query → 各原版欄位escaping → Rust相容parser → typed AST → analyzer選擇 → engine lowering → execution → scoring/sort → 原版snippet/highlight/pagination。

UI的defs/refs與REST的def/symbol等接口差異要由adapter保留。不能直接把Tantivy內建parser當成OpenGrok相容parser。AST至少明確表達fielded term、phrase/slop、Boolean must/should/must-not、prefix/wildcard/regex、range、fuzzy、boost、all/empty；每種語意都要從reference驗證，列型別不代表完成。

### 4.2 Token與offset

逐欄定義full、defs、refs、path、hist、type、project及其他metadata。token契約包含term、position、position_length、source byte spans與normalized spans。Java字串offset與Rust UTF-8 byte offset要明確轉換，不能混用UTF-16 code unit、Unicode scalar與byte。

測underscore、dot、slash、長identifier、Unicode/CJK、CRLF/BOM、註解/字串、數字與大小寫。Tree-sitter是syntax parser候選，不會自動提供原版Ctags/analyzer/refs語意；D19已允許隨包Ctags；優先驗證與原版一致的Ctags能力／參數及Rust adapter，具體build在G1固定。

### 4.3 排名與跨project

相同BM25名稱不代表相同score/order：norm量化、稀疏欄位doc count、IDF、boost、tf、deleted docs、ties與selected-project統計都要比對。每project各用局部分數取Top K再合併，不能直接視為原版跨project搜尋。

先定義ScoringScope：selected projects、可見文件集合、generation、field statistics與tie rule。候選比較「工作區單index＋project filter」及「per-project indexes＋共同統計／scorer」；單index的filter不會自動改變引擎的全域IDF範圍。Tantivy不能滿足contract時，先調adapter/scorer或重評engine，不能偷偷放寬排序。

D20未批准任何新差異。若reference同分順序確有非決定性，先有證據才建立tie-group比較規則。

### 4.4 Index與配置相容

Tantivy候選使用自己的versioned index/schema，不宣稱直接讀Lucene索引。建議從sources重建；是否接受首次重建屬D10/D20。prototype只能在合成corpus重建，不動使用者原索引。Java XML配置以白名單讀取與schema匯入，不執行XMLDecoder物件。

詳細query/scorer來源與差異見12-rust-compatibility.md。

## 5. Sources、incremental與一致性

每檔候選metadata包含DocumentId/ProjectId、相對path、來源引用、language、size/hash、source revision、analysis version、xref generation、history狀態與delete marker。不能僅看mtime判斷不變；測rename、branch checkout、dirty/untracked與SCM更新。

D05仍決定來源布局、跨磁碟、symlink/junction、NAS/UNC；改Rust不自動保證任意network filesystem。source與index可放的位置分開定義。

Rust自己提供實體writer鎖與持久queue，不繼承上游Lucene lock。每個index只能一writer，global registry/config發佈亦有受控序列；不同projects並行受CPU/RAM/IO預算控制。

候選generation序列：

1. 取source manifest與config revision、建立job。
2. 分析變更，寫pending index/xref/history。
3. 檢查來源是否期間變動；依D12重試、標stale或使用snapshot。
4. 完成engine commit與metadata，產生generation receipt。
5. 通知reader reload，驗證新generation可查。
6. 標記job成功，再依policy清舊資料。

Tantivy commit不等於index/xref/history/config全體原子提交。停機切換、shadow、snapshot或可恢復journal是待驗方案。Windows reader handles與mmap可能妨礙rename/delete。只保留舊index仍不能保證live raw source是舊內容，強一致性需相應source策略。

## 6. 排程、記憶體與取消

HTTP/短I/O協調由Axum/Tokio候選處理；CPU密集分析、壓縮與長時間index工作放有上限的worker pool。候選dedicated threads／Rayon／隔離worker process，按profiling選，不在async handler直接跑整repo同步分析。

使用有界channels、query及index資源預算，避免每檔無上限spawn、整repo讀入RAM、無上限cache與每query重建昂貴狀態。共享唯讀generation，避免全域Mutex包住整個query或writer。mmap、zero-copy、buffer reuse需有安全生命週期與實測，不只為口號加入。

Tokio官方說明已開始的spawn_blocking不能以abort取消；CPU並行也應受限。取消設計須在檔案／批次／stage合作式檢查；external ctags/SCM有timeout與child lifetime。必須硬取消的工作可隔離成worker process，但增加IPC成本，要驗證。[spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)

## 7. UI與xref移植

先盤點home/search/xref/directory/history/annotate/diff/help/error等頁面。Rustview model輸出保留原route、DOM/classes/assets、shortcut/hover/back。CSS/JS要修改時列patch原因及互動差異，不直接換套介面。

source不能直接插入HTML；以typed safe fragments處理可信xref片段，保留escaping，防query/url雙重編碼。snippet與xref的offset以相同原始內容版本計算。原CSS相同不是UI已相同，需測rendered DOM與同OS/browser screenshot。

## 8. SCM、plugins與API

D19已選A：必要SCM CLI可隨包，由Rust adapter管理；精確工具／build、library替代或FFI選型仍需G1證據。原版Git history用JGit的路徑必須對照，Git CLI/git2/gix的rename/follow/merge/blame不因同為Git就自動等價。其他原版SCM不可默默取消。

Java JAR與Rust ABI不同；既有plugins、Python CLI、Java XML及API consumers列D16/D20。可以提出Rust功能移植、adapter或明確獨立bridge，但不能默加JVM／默刪功能。目前標準Rust profile不依賴Java，全部現有產品測項以此為前提。特定舊JAR plugins的bridge只是D20未批准候選；若未來使用者明確需要，須獨立profile、依賴台帳與完整驗收，不得取代Rust核心或成為隱性fallback。

## 9. Lifecycle與持久資料

Rust native binary持有服務生命週期。候選命令有doctor/init/project/index/serve/status/stop/logs/backup/upgrade-check；前景、背景或service由D15選。npm wrapper只負責入口。

start檢查platform/config/依賴/lock/port，再初始化storage/reader/worker/HTTP。ready要含schema、project與generation可查。stop先拒新jobs，再依policy drain或取消、完成或回滾commit、關HTTP/child、釋放鎖與保存狀態。Windows和POSIX分開測。

安裝binary/assets不可變；workspace分config/metadata/index/xref/history/jobs/run/logs/backup。升級保留舊binary及相容generation manifest，Windows執行中EXE不就地覆寫；npm upgrade/uninstall不刪source/data。Rollback依schema與資料版本，不是只降binary。

## 10. 使用模式與安全

D02已確認local＋LAN兩種模式、約5人共用、不做使用者／角色／project ACL管理。同一Rust binary用配置選擇loopback或明確LAN介面；read routes共同存取服務內已註冊projects。免新增登入頁／帳號為工程提案，不列必需SSO或auth插件。資料根目錄、URL、檔案與process邊界仍是程式契約。

D16尚待確認管理入口。建議主機CLI／本機控制面處理project/index/config/stop，LAN提供讀者路由；這是提案，不把目前回覆當已批准此入口或允許所有人管理。需要既有REST管理相容時另盤點D16/D20，相關token/Origin等依實際控制面決策。企業source/index/secrets不進GitHub/npm，runtime不自動CDN/telemetry/download。

## 11. 效能契約

同corpus/query mix/硬體/資源預算/功能比較。Rust release記compiler/flags/allocator/CPU/features；reference用相容JDK、合理heap/cache、分開cold與warm。兩邊計整個process tree，Rust包括workers、external tools及npm launcher若常駐。

量啟動ready、全量、指定比例增量、query p50/p95/p99、並行吞吐/error、RSS/peak/mmap/page cache、CPU/IO、index與備份大小。不能用cold Java對預熱Rust，或缺history/排序功能的Rust比較完整版OpenGrok宣稱速度。非等價工作負載可研究但不得算達成性能需求。

## 12. 可交接的資料與服務契約草案

以下是 WP-B08／WP-C01 前需要凍結的契約，不是已實作的 API 或使用者已批准的命令格式。欄位名稱可在 ADR 中調整；下列語意必須能以 tests/contract 驗證。公開 HTTP/REST 的格式以原版契約優先，不能把內部 Rust types 直接序列化成新 API。

### 12.1 核心識別與持久資料

| 契約 | 必需內容 | 不變條件與驗證 |
|---|---|---|
| ProjectRecord | stable ProjectId、display_name、source mapping、enabled state、config revision、來源限制 | ID不以顯示名稱或絕對路徑直接替代；改名/移機不破壞public identity |
| SourceDocument | ProjectId、原始relative path bytes／編碼資訊、可安全表示的URL path、content hash、size、language、line map、SCM state | 不把Windows separator直接存成跨平台URL；同名不同project獨立 |
| SourceRevision | 明確snapshot／manifest identity、dirty state、採樣hash與可用SCM revision | Git commit不能代表所有dirty/untracked內容；xref/raw和index使用相容來源策略 |
| IndexGeneration | opaque generation ID、schema/analysis/engine versions、source manifest、published_at、各component receipts | generation成功需全部必要component可用；非單看一個index commit |
| GenerationManifest | parent generation、project/scoring scope、index/xref/history metadata、必要checksums、pending/active/retired狀態 | reader取得一致manifest；舊generation保留至沒有reader且符合rollback policy |
| JobRecord | job id、request id、project/config revision、mode、state、progress、取消請求、generation、error code、開始/完成時間 | 重啟可識別未完成job；失敗不能宣稱新的索引ready |
| FeatureRecord | 原版feature、Req ID、source/reference case、Rust owner、Test IDs、相容狀態與批准差異 | 未實作／blocked／上游不可驗不等於passed或功能不需要 |

來源路徑需要區分「使用者配置的原始路徑」、「存取檢查後的實際路徑」、「對外project相對URL」。follow symlink/junction、大小寫、Unicode normalization與UNC依平台契約處理；canonicalize一次不能消除後續來源替換的所有風險。索引與讀檔操作保留root邊界檢查，不靠字串prefix判斷安全。

### 12.2 Query 與結果

| 契約 | 必需輸入／輸出 | 驗證焦點 |
|---|---|---|
| QueryInput | 原HTTP/CLI欄位、原字串、selected ProjectIds、sort/page、共同可讀scope與generation selection | URL decode與query escape各一次；保留原UI多框合併規則 |
| CompatibleAst | typed nodes、field、term/phrase、Boolean、range bounds、fuzzy/slop/boost與source spans | parser與engine lowering分開；不支援的節點不可悄悄忽略 |
| AnalysisOutput | normalized term、position、position length、source byte span、必要原版char span、analyzer identity | 不切開UTF-8字元；phrase位置、長token與大小寫契約 |
| ScoringContext | selected projects、fields、有效文件集合、statistics scope、engine/searcher generation、sort contract | filter與statistics不同層；跨project結果需要原版等價視野 |
| SearchResult | total hits與精確度、ordered hits、page資訊、query diagnostics | total hit threshold／approximation不能默改；不丟掉相同score的結果 |
| SearchHit | stable DocumentId、project/path、score若原介面可見、match spans／lines、sort values、generation | score/排序分開驗證；不把engine doc ID放永久URL |
| XrefViewModel | source revision、行號、safe token fragments、definition/ref links、CSS classes與navigation | 同內容生成snippet/xref；source文字與可信markup分開 |
| CompatibilityError | 原操作類型、相容錯誤種類、HTTP/status映射、可見message與內部diagnostic ID | 不洩露source/token；必要原版錯誤行為需對照，不能一律500 |

分頁策略依原版結果契約實作。若採 search-after／快照，需要保留或等價實現使用者看見的offset/page行為；索引更新時的重複／遺漏語意納入D12，不能以私有游標默改原URL。

### 12.3 Index job 狀態與提交

候選持久狀態如下，最終命名在ADR固定：

~~~mermaid
stateDiagram-v2
  [*] --> Queued
  Queued --> Running: 取得writer與資源
  Running --> Prepared: 完成pending資料
  Prepared --> Publishing: 驗證後切換
  Publishing --> Succeeded: 新generation可讀
  Queued --> Cancelled: 取消
  Running --> Cancelling: 收到取消
  Cancelling --> Cancelled: 清理完成
  Running --> Failed: 錯誤
  Prepared --> Failed: 驗證失敗
  Publishing --> Recovering: 不完整提交或重啟
  Recovering --> Succeeded: 證明已完整生效
  Recovering --> Failed: 保留最後可用generation
~~~

- **進入Running前**：寫下config/source版本、獲得對應實體writer鎖，經全域資源預算核准。queue持久化與worker權限是不同層，不能只鎖記憶體中的Map。
- **Prepared前後**：source manifest與index/xref/history receipts可核對；來源變動依D12重試／stale／snapshot，不偷偷標成功。
- **Publishing**：事前寫recoverable intent，記錄哪些component已commit。跨index與metadata不能宣稱天然原子；crash後靠journal/manifest驗證或回復最後完整generation。
- **取消**：有明確可取消邊界；進入不可回滾的publish階段後，先完成一致性處置，再回傳實際結果。不能收到cancel即刪除仍被reader使用的檔案。
- **Succeeded**：reader確實取得可讀generation後才寫成功。只有「子程序exit 0」或「engine commit完成」不足。
- **Failed/Cancelled**：保留可診斷錯誤與必要pending資料，按policy回收；不得刪使用者source。restart能識別並清理殘留工作。
- **重複請求**：request/job identity避免網路重試重開互相競爭的writer；是否合併重複full/incremental jobs由queue契約明確定義。

這個狀態機是工程提案，不代替D12對服務可用性與資料新鮮度的選擇。

### 12.4 服務邊界與控制面

| 操作族 | 內部服務責任 | 外部映射 |
|---|---|---|
| ProjectService | validate/register/list/disable/remove registration | CLI與批准管理API共用同一實作；remove不刪source |
| IndexService | plan/enqueue/cancel/status/recover | 管理API若非同步，回job id與可查狀態；原版REST契約另做adapter |
| SearchService | parse/analyze/execute/collect/render snippets | HTTP搜尋與原版API使用相同查詢語意，避免兩份邏輯 |
| ContentService | tree/source/raw/xref/revision | 先檢查project註冊/root/revision，再產生共同可讀內容；不加入逐使用者ACL |
| HistoryService | repositories/log/blame/diff | 封裝SCM差異、timeout、cache與錯誤 |
| HostService | readiness/stop/diagnostics/version | 程序控制須驗證instance identity；不要僅憑PID檔就kill |

前景／背景／service模式依D15決定。若需要本機control channel，可比較POSIX socket、Windows named pipe或受保護loopback端點；未選定之前不固定傳輸。但端點只控制正確instance、無admin暗示、身份權限、timeout與shutdown語意必須先定。

log至少能以request/job/project/generation關聯階段和錯誤；performance counters和使用者可見訊息分開。普通log不記原始source與credential；診斷匯出需讓使用者知道內容。

### 12.5 契約的實作與驗證順序

1. WP-A05/A08確認原版可觀察契約與差異比較方法。
2. WP-B02/B03/B06用少量公開語料驗證token/scoring/view model/generation設計。
3. WP-B08將interfaces、序列化版本、能力旗標與錯誤映射記入ADR。
4. WP-C01建立有可運行contract tests的共用types；不先為全語言建立空crate。
5. 各Rust模組與npm封裝只依賴已定契約；更動需指出受影響FR/NFR、WP與test IDs。
6. 原版兼容層與Rust內部schema分別版本化；資料格式升級需G4 migration/rollback證據。

## 13. 搜尋優先的工程提案

D21已確認搜尋速度第一優先，D19已確認必要工具隨包。詳細實驗見[14搜尋效能設計](14-search-performance-plan.md)。

- 一般全文／defs／refs／path查詢以已發布索引與必要metadata為主要讀取路徑；Ctags在分析／索引階段使用。snippet若需source內容，依既有一致性契約讀取，不能為了避免I/O而回傳錯誤版本。
- reader／searcher可重用；每次查詢取得一致generation與scoring scope，避免重新開索引、重建全域狀態或持有全域writer mutex。
- CPU密集查詢與index工作有各自受控佇列／並行預算。先以profile比較同時索引下的尾端延遲與資料新鮮度，不自行給index無限低優先而延後更新。
- cache採有界容量及單獨命中／miss／eviction統計。完整鍵含query語意、projects、sort/page、config/analyzer/index generation與內容版本；本版共同可讀，不加入不存在的使用者／ACL cache分區，仍不可跨project／舊generation誤用。
- 先量query parsing／analysis、排隊、postings、統計／ranking、result collection、snippet/source I/O、模板輸出與client render，再選最有收益的優化。
- top-K／early termination／預計算／skip策略需符合total hits、排名、分頁、boost與跨project統計。未證等價的approximation不進相容產品。
- history／annotate／diff可能需要SCM；按各類操作測量，不把一般搜尋不重跑Ctags推廣成所有功能絕不呼叫工具。
- primary benchmark為相同結果下搜尋延遲；RSS、磁碟、索引時間、錯誤率與freshness都是記錄項。資源與新鮮度取捨需D09/D12/D21確認。

## 14. 本機＋LAN、macOS開發與Ubuntu amd64部署

目前約5人規模採單主機服務／共用索引的候選設計。單人local profile與LAN profile共用業務邏輯、source mapping、索引格式與原版UI，不為5人另建分散式搜尋／帳號平台。實際CPU／RAM與資料規模仍待提供。

使用者確認先在macOS完成開發與第一輪完整驗證，再於Ubuntu24.04 amd64做平台適配與完整原生重驗；Ubuntu仍是主要部署目標。候選設計共用Rust業務核心與test contracts，平台層處理paths、檔案系統、程序／signals、locks、bundled helpers與打包。Mac已確認M1／arm64；macOS實際／最低版本待決。原生arm64與Rosetta執行分開記錄，預編譯Mac payload採確認的arm64架構。Windows支援保留。Mac、Linux各自的oracle/candidate/artifact evidence分開，跨編譯不算native PASS。

負載使用1人baseline及約5人的獨立互動client；測不同projects/query、共用熱門cache、cache miss、慢query與並行index。client只是load generator，非產品帳號。人數不是固定request concurrency或QPS，query mix／間隔／SLO另定。更多契約見[15部署與平台順序](15-deployment-and-linux-priority.md)。

## 研究來源

- [Tantivy](https://github.com/quickwit-oss/tantivy)、[QueryParser](https://docs.rs/tantivy/latest/tantivy/query/struct.QueryParser.html)
- [Axum](https://docs.rs/axum/latest/axum/)、[Tokio blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
- [Tree-sitter定位](https://tree-sitter.github.io/tree-sitter/)
- [OpenGrok固定版本](https://github.com/oracle/opengrok/tree/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3)，詳細檔案定位與查詢差異見01及12。
