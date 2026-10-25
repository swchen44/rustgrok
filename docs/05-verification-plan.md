# Rust核心重寫：驗證計畫 v0.6

**文件狀態：proposed／待討論。產品實作與測試狀態：全部 planned，尚未執行。** 日期：2026-10-25（Asia/Taipei）。

使用者已確認 **D01：Rust核心重寫**。原版功能、style、多專案、內網、macOS／Ubuntu24.04 amd64／Windows 64 位元、npm及離線ZIP要求保留。**首輪開發平台已確認Apple M1／Apple Silicon arm64；先在macOS原生arm64完成完整移植、功能、介面及包裝，再於Ubuntu24.04 amd64做平台適配與完整原生重驗；正式部署優先是Ubuntu24.04 amd64。** macOS實際／最低版本、Windows版本／CPU及其餘環境仍由D03確認，不能把Windows 64 位元直接視為已批准x64。Windows最終原生驗收不刪除，不能以Mac PASS或跨編譯成功取代Linux／Windows實測。這次變更不能被解讀為同意少做語言／SCM／API、改搜尋排序或重新設計UI。

本版所有產品測項適用於**標準Rust profile**：不包含或依賴JVM/Tomcat，不執行WAR/JSP，也不呼叫Java完成未移植功能。原版OpenGrok/JVM/Tomcat只在**獨立測試參照環境**執行。使用者已確認 **D19：A，隨包附帶必要helpers**，以及 **D21：搜尋速度優先**。排序、舊index/config與JAR plugins等相容邊界仍為D20 OPEN；搜尋SLO數值、corpus、hardware、request率、think-time、每人請求並行與負載分布仍待確認。未決前不放寬完整同等要求。

使用者選擇**本機local與LAN都支援、約5人同時使用、不用權限管理**。因此帳號、角色、RBAC及project ACL不列為mandatory需求，已登記專案採共同可讀正向驗證。約5人是代表互動負載，不是5 QPS、5個in-flight requests或硬性人數上限。免登入讀者入口只是工程建議，不能寫成使用者已批准，也不新增強制登入／SSO要求。管理操作開放何處、token／CSRF／本機控制面仍依D16／D20確認。

所有主要runtime驗收均使用bundled產物，必要ctags／SCM helpers的精確清單、版本、native依賴及授權由功能inventory鎖定。B「要求使用者外裝工具」與C「不使用helpers」不是本版mandatory交付profile。缺helper、錯版本、損壞或不相容binary仍須故障注入驗證；這些負測不代表正常交付可缺件，也不允許從系統PATH或外網偷偷補件。

本輪**沒有批准任何Java plugin bridge**。只有未來D20明確批准特定插件bridge、另立獨立profile、測項與gates後，才可驗證該額外交付；它不得取代Rust搜尋、索引、Web及host核心，也不得讓本版標準profile的無JVM測項免驗或通過例外。

公開repo **[swchen44/rustgrok](https://github.com/swchen44/rustgrok)** 已回讀確認建立成功，ID **1433282584**、default branch **main**，發行包放 **GitHub Releases**。建立回讀時size=0、內容未提交且沒有產品Release；後續內容上傳需另記實際commit回讀。產品仍未實作，repo建立不代表binary已發布或任何產品測項通過。

## 1. 驗證目標與交付現況

驗證分成互相獨立的兩張結果表：**正確性／功能同等**及**性能／資源收益**。速度較快但查詢語意、索引內容或功能較少，不算完成。性能驗收優先查詢延遲與搜尋使用體驗；記憶體、CPU、索引及其他成本仍完整量測，搜尋優先不代表已同意其他成本無上限。Rust沒有保證更快，也沒有現成倍數或SLO。本階段先建立公平基準與最難相容性原型，讓大型實作前就能看見成本與收益。

機器規格見[`tests/matrix.json`](../tests/matrix.json)。本版沿用原68個ID與既有RST-001～RST-012，共**80項測試規格**，沒有新增或刪除測項。22個FR與16個NFR均有追蹤。所有evidence路徑是未來範本，沒有產品測試結果。

| 狀態 | 意義 |
|---|---|
| planned | 只有規格，未實作／未執行 |
| blocked | 缺決策、平台、資料或前置條件 |
| passed | exact版本與artifact實際執行，結果符合預期且有證據 |
| failed | 實際執行後未滿足預期 |
| skipped | 未執行並記原因，不計passed |
| not_applicable | 有明確批准的scope依據；不能自行刪功能 |

### 平台執行順序與證據

| 工作階段 | 必須取得的證據 | 不可替代的部分 |
|---|---|---|
| macOS先完成 | 在Apple M1原生arm64與核准macOS版本上完成全部移植、功能／API／SCM／query／UI parity及實際npm／ZIP包裝驗證 | 不是只跑垂直切片；Rosetta x86_64不可算arm64 PASS，Mac結果不表示其他平台通過 |
| Ubuntu24.04 amd64後續適配與重驗 | Mac階段完成後處理Linux平台差異，重新跑全部Linux適用的功能、UI、包裝、離線、恢復、安全及效能測項 | 跨編譯只證明產物可建置；必須在Ubuntu24.04 amd64原生執行 |
| Windows最終原生驗收 | 保留全部承諾功能及Windows路徑、鎖、程序、helper、包裝等平台證據 | 不能用Mac／Linux結果、WSL或Docker代替 |

這是平台實作與測試順序，不改既有G0～G6及各Test ID的gate映射。Mac適用測項可先取得結果，但跨平台gate必須等全部承諾平台證據齊全才關閉。每個run獨立記錄OS／CPU、artifact hash、source commit、corpus及結果，禁止把Mac的passed複製成Linux passed。Linux適配若修改共用程式，另回歸受影響的Mac契約。 M1 run須記錄主機CPU、實際macOS版本、Mach-O架構／arm64 slice、Rust核心／npm執行環境／必要helpers的實際程序架構及翻譯狀態；不能只看主機是M1就把Rosetta x86_64執行算原生arm64通過。macOS實際及最低支援版本仍待確認，CPU架構不再重問。

## 2. 先建立獨立原版 oracle

研究基準為OpenGrok `1.14.20`、commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`，其POM使用Lucene `9.12.3`、Java21。使用者現用版仍需確認；研究版不能自行取代最終驗收基準。[S1]

建立REF與RUST兩個獨立部署。REF使用標準原版以及固定JVM／Tomcat／ctags／SCM與配置。RUST使用Rust release build及獨立原生依賴清單。兩者輸入source、history、project選取、邏輯功能配置、locale/timezone與browser一致；執行時與索引檔案格式允許不同，但可觀察行為必須符合批准契約。

黑箱oracle涵蓋HTTP/API、查詢結果、raw、history/diff、xref、頁面及操作。若定位差異需要AST、token、scorer explain，另寫只依賴pinned上游的Java診斷讀取器。它只供測試，不被產品使用，不能依賴Rust parser或engine adapter，避免兩邊共用錯誤而假通過。

Golden先由原版產生，再用簡單手算案例審查。baseline更新必須附上游差異與批准理由，禁止因Rust失敗就改預期。

### 正常化白名單

可經審查正規化host/port、無語意的安裝絕對路徑、查詢耗時及波動識別碼。不可刪除結果總數、順序、line、highlight、revision、來源相對路徑、排序用日期。不可先把全部結果排序再比較。Tie及浮點差異只能用診斷說明；D20未確認前，不設任意容忍值降低排序要求。

## 3. 分層驗證，避免最後才發現底層不相容

| 層 | 對照輸入／輸出 | 必測差異 | 主要測項 |
|---|---|---|---|
| Query入口 | UI各框／REST參數→語意query | UI defs/refs與REST def/symbol、default AND、多框合併、field escape | QRY-001/003、API-001、RST-002 |
| Parser／AST | 原query→相容AST→engine query | Boolean、phrases、wildcard、Lucene regexp、fuzzy、range、boost、非法query | QRY-002～005、RST-002/007 |
| Analyzer | source/query→terms/positions/offsets | 每語言／欄位規則、大小寫、long identifiers、單字元、Unicode | SRC-001、RST-003 |
| Index／scorer | corpus＋選projects→hits/score/rank | field docCount、稀疏欄位、df/norms、刪除、global stats、top-K／ties | QRY-006、PRJ-001、RST-004/009 |
| Snippet／xref | hits＋source→行號／高亮／symbols/scopes | UTF-16/UTF-8座標、CRLF、multiline、HTML escape、歷史xref | UI-002～004、RST-005 |
| Rust SSR／HTTP | 原路由＋資料→DOM／headers／互動 | 原JS依賴的DOM id/class/attributes、URLs、原CSS/JS資產、原版style | UI-001～006、SRV-001、RST-005 |
| SCM／設定／plugins | 原流程→history/annotate/diff／config效果 | 全SCM能力、rename/merge、時序、API tasks、舊config/JAR邊界 | SCM群組、API-004、RST-011 |
| 執行與維運 | npm/ZIP→Rust程序＋持久資料 | 原生平台、依賴閉包、無網、locks、crash、upgrade | PKG/OS/REC群組、RST-006/009 |

### 三個已確認的高風險細節

- OpenGrok `CustomQueryParser`設default AND；`QueryBuilder`逐欄位escape/parse後有自己的Boolean合併規則。不能以「看起來像Lucene」的第三方parser代替。[S2]
- 原版`SearchEngine`跨projects使用`MultiReader`。Rust若逐project獨立算分再合併top-K，不會自然取得同一collection statistics，必須用跨project corpus驗證。[S3]
- 若選Tantivy，它的預設tokenizer會小寫並套long-token filter。研究讀到0.27.0 BM25的預設statistics以index document數為分母；Lucene9.12.3 BM25用field的docCount，對稀疏欄位有差異。同名BM25不是排序相容證明；還要檢查norms、boost、constant-score組合與query rewrite。候選crate尚須正式鎖版。[S4][S5]

Rust parser/analyzer的內部結構可以不同，但必須先對齊語意。當Lucene提供某個功能而候選engine缺少時，應補adapter／自有query，或提報尚未完成；不能默默換成近似功能。

### local／LAN與共同可讀契約

SRV-001、SEC-002驗證兩種運作模式及共同讀取：local模式核對本機listener邊界；LAN模式由多個獨立client讀取同一批已登記projects，涵蓋search、suggest、xref、raw、history、diff、download、直接URL與read API。不建立允許／禁止專案的角色帳戶，不因取消RBAC而把測項skip算passed。不同client的project選取、query與cache結果仍需正確，不可互相污染；來源root外的檔案與管理狀態不因共同可讀而自動開放。

管理面與讀者面分開盤點。工程提案是**主機CLI／本機控制面管理、LAN讀者面只讀**，並非已批准政策。API-001/002/004與SEC-005依D16／D20列出原REST管理端點、verbs、狀態改變及所需控制；token、Host/Origin、CSRF、transport等只在選定政策下驗證。不把「不用權限管理」解讀為已允許所有LAN讀者改配置、reindex或停止服務，也不把管理面待決重新變成登入／RBAC需求。

## 4. Golden corpus與property測試

Synthetic corpus可上GitHub；真實企業source、私有搜尋字串、paths、heap dump及含source畫面只留內網。Corpus固定seed、內容、作者、時間、SCM commits、配置與hash。

| 資料組 | 必備內容 |
|---|---|
| 多project | 同名symbol，不同project文件數／詞頻，空project、無SCM、單project多repo、groups |
| 語言與欄位 | 從pinned indexer --help guru取得完整analyzer inventory；每承諾語言測defs/refs/full/path/hist/type |
| Token邊界 | 超過40長identifier、單字元、underscore、數字、字串／註解、標點、大小寫、宏／typedef／scope |
| Unicode／檔案 | UTF-8/BOM、CRLF/LF、繁中、emoji、combining marks、空白／#/%檔名、long path、binary、巨型行 |
| Query | 原版語法全部正／負／邊界，深巢狀、escape、regex/fuzzy/phrase/slop/range/boost與多欄組合 |
| Ranking | sparse fields、不同長度、rare/common terms、同分、被刪但未merge文件、boost＋constant score、project統計差異 |
| SCM | 固定add/edit/delete/rename/merge/tag/branch、非ASCII message、同時戳、多repo、dirty working tree |
| 故障與路徑 | symlink/junction/cycles、越根目錄、Windows鎖、disk full、corruption、commit中斷、reader交換 |

Token diagnostics定義共同座標系。Java analyzer的UTF-16 offset與Rust byte offset不能直接比整數，需用原始source驗證轉換，再比較同一字元區間及行號。測試要能抓出emoji或CRLF造成的高亮位移。

Property/fuzz用固定seed並縮小失敗案例，輸出成可重跑regression。只有已證明適用原query語法的metamorphic relation可以使用，不任意利用一般Boolean代數重寫語句。Fuzz沒有發現錯誤不等於已覆蓋完整語言；inventory與coverage另列。

## 5. 性能量測：先證明工作量相同，再談收益

### 5.1 公平基準

使用同硬體、OS、磁碟／檔案系統、CPU／電源設定、資料及功能。原版的heap、GC、index threads及相關配置做合理調校，保留參數與依據；Rust以可實際發行的release profile、target及allocator建置，不能拿debug build，也不能以僅本機可用的target-cpu=native結果代表所有下載者。[S6]

不要求兩者相同JVM／Tomcat；標準Rust profile不使用它們。共同控制的是**工作量、功能、資料、硬體及可用資源預算**。D19已選bundled，Rust比較使用實際發行包所附helpers；兩邊需要ctags時固定並記錄版本，避免把工具差異誤當engine收益。SCM必須依原版實際backend核對：OpenGrok的Git history使用JGit，Rust若改用git CLI、git2或gix，即使固定同一Git版本也不能免除rename/merge/blame差異測試；先驗輸出相容再比較並標示依賴差異。[S13]

### 5.2 分開startup、warmup與cache狀態

| 狀態 | 測量內容 | 不可混淆 |
|---|---|---|
| Process cold start | 新程序到配置／索引可用／首頁及第一個query完成 | 啟動快不等於steady-state搜尋快 |
| JVM warmup | 原版JIT／class loading後性能時序；同時記錄第一輪體驗 | 不用cold JVM對已熱Rust宣稱總體加速 |
| Steady state | 已穩定服務下固定query mix與到達模型 | 保留warmup證據，不任意丟棄較慢樣本 |
| Application caches | cold/warm query、xref、history及suggester caches；記錄可觀測的hits/misses及命中率分母 | 應與OS page cache狀態分欄；無法觀測標unknown，不以重複query推定命中率 |
| OS page cache | 實驗性cold／warm檔案資料 | restart不會自動清OS cache；無權清除時用clean VM或標未測 |
| 索引資源競爭 | 同時read/index／多project | 不以只跑孤立query推論內網多人行為 |

Java微基準可用JMH，Rust可用Criterion，但兩者僅輔助定位；產品收益必須以端到端HTTP／index jobs量測。JVM是否進穩態需記錄，不能只假設固定幾秒warmup永遠足夠。[S7][S8]

先量**1位使用者參照**，再量**約5位使用者的互動情境**。各client使用不同project/query組合與獨立瀏覽狀態，包含重複和未重複查詢、公用server cache，以及背景完整／增量index。兩種人數都跑相同功能與可對照資料；local與LAN網路拓撲分別記錄，不混成同一延遲分布。macOS先建立開發量測，完整移植／介面／包裝完成後在Ubuntu24.04 amd64重新量測；正式部署pilot以Ubuntu24.04 amd64為優先。各平台均使用該平台同硬體原版對照，不能把Mac性能或SLO結果當Linux證據；硬體型號、容量與其餘條件仍需核准。

約5位使用者不直接換算為QPS或請求並行數。request arrival model、rate、think-time、每人並行行為、資料量、硬體與SLO仍OPEN；每run另記實際活動client數、request率、in-flight與排隊長度。G1可用明示的研究設定探索，不能冒稱已批准SLO或把5當連線／請求hard cap。

### 5.3 指標與報告格式

| 指標群 | 應報內容 |
|---|---|
| Query（優先） | 各查詢類別、單／跨project、cold/warm與並行索引情境的p50/p95/p99、樣本數、throughput、error/timeout rate與slow-query明細；零／少／大量hits分開 |
| Server計時 | 接收／admission、queue wait、parse/plan/search/snippet/SSR及response；記錄各span邊界與request ID，不以處理時間代替包含排隊的請求延遲 |
| Browser計時 | 提交查詢→首批結果可見／完成互動的端到端時間；另列HTTP/network與render，不把server耗時當瀏覽器體驗 |
| Cache | 各應用cache的hits/misses／命中率、容量與失效條件；同時標註OS page-cache狀態，unknown不能填0或100% |
| Full index | source bytes/file count、時間、CPU、peak RSS、temp、history／xref產出與index bytes |
| Incremental | 固定add/edit/delete/rename變更率、全庫scan成本、update時間、freshness、write amplification、結果對full rebuild |
| 記憶體 | 所有產品process＋bundled ctags/SCM helpers子程序同時採樣總RSS；另記heap/allocator、mmap、峰值與觀測限制 |
| 儲存與I/O | index/xref/history/config各自大小、峰值temp、實際read/write bytes/ops、merge/flush；不只比較tgz大小 |
| Startup | 程序→ready、頁面ready、首query；已索引／未索引與cache狀態分列 |
| 載入與並行 | CPU time/utilization、lock/contention、threads/handles、reindex對tail latency影響 |
| 交付 | npm/ZIP下載與解壓bytes、安裝時間、依賴與runtime closure |

RSS總和會重複計入共享頁面，需交代算法；可在平台支援時補PSS/USS，不把跨OS不同指標直接混比。不能把各process在不同時間的最大RSS相加當同時峰值。Heap小不等於包含mmap/cache與子程序的總記憶體小。

每指標列REF、RUST、差異/比率、重複run分佈或區間、coverage及配置。交錯paired runs控制cache/溫度／背景負載，固定request到達模型並保存timeout，避免只算成功請求掩蓋過載。profile runs另跑，不能把instrumentation overhead混入正式耗時。D21已確認搜尋速度優先，但沒有批准數值權重，不能把startup、RSS、search和index混成單一總分。

查詢至少依全文、defs、refs、path、Boolean、多欄組合、phrase/proximity、wildcard、regex、fuzzy、range、history/type及原版其餘承諾語意分類，再分單project／跨project、選中project組合、命中量、分頁、cold/warm、1人／約5人及背景完整／增量索引狀態。類別可重疊，須保留query ID與分類標籤。查詢集合、各組比例、每人請求並行／到達率、think-time、測量長度與門檻須先記錄，G5前由使用者批准；G1實驗配置只標研究假設，不冒稱實際內網負載。

每組分別報分位數與樣本數，保留逐query的慢查詢、timeouts、錯誤及取消；門檻未定時仍保存完整樣本與最慢案例。成功請求的分位數必須標示為條件分布，timeouts另列數量、比率與deadline，不能丟掉失敗樣本或把timeout截止值當完成延遲。server與browser以request ID關聯、各自單調時鐘計時，不能直接相減不同時鐘的timestamp。並行索引需包含查詢同project、其他project及跨project集合；延遲改善不能靠漏功能、漏命中或未批准的較舊資料。以上細化既有PERF-001/002、PRJ-002與RST-008/010/012，不新增Test ID。

### 5.4 O(n)與allocation的精確邊界

可把單次decode/token scan／query lexical scan／HTML escape與render設計成以input bytes加output量為主的線性處理。用成倍輸入尺寸、bytes/token/line/match/output分開控制，追蹤time、alloc count、allocated bytes、peak live bytes及copy量，尋找重複掃描或每token複製整份檔案。

不能宣稱整個搜尋是O(query length)。Regexp編譯／展開與pattern大小、詞庫／automaton有關，posting遍歷與命中數有關，排序／top-K與候選數有關。Rust regex文件也明列單次搜尋的O(m×n)邊界，某些迭代API最壞情境更高；它並非Lucene regexp的直接語意替代。[S9]

Profile先判CPU、allocation、locks、I/O或外部工具哪個主導，再嘗試buffer reuse、borrow/streaming、reserve、避免熱點clone/format或調整資料佈局。每個優化都要重跑parity與端到端。zero-copy／SmallVec／換allocator都不是自動更快；輸入成長曲線可以揭露問題，但不能取代算法複雜度證明。若優化引入unsafe、FFI或自訂mmap生命週期，補上適用的Miri/sanitizer、並行與平台故障驗證；這些工具的限制必須明列，不用語言名稱代替證據。[S14][S10]

### 5.5 G1收益評估與停損資訊

G1的prototype只可對parity已成立的slice報數據，明列尚未移植範圍，不能將subset速度外推整體。依D21先定位搜尋關鍵路徑與慢查詢；若沒有明顯收益或某些指標退步，照實報告瓶頸與代價，再提出優化取捨；**不自動撤回已確認的Rust方向，也不改回Java產品**。搜尋優先與約5人代表情境已決，具體SLO、corpus/hardware、request率／think-time／每人並行仍OPEN；所有改善倍數在證據前維持unknown，不自行新增數字門檻。

## 6. 既有68測項的轉寫範圍

ID保持穩定，原有需求不因換語言被刪除。下列為內容更新的重點；完整steps與expected見JSON。

| 既有 IDs | v0.6規格重點 |
|---|---|
| REF-001～003 | 固定原版oracle與Rust候選的各自manifest，Java只供參照；新增語意／plugin相容inventory |
| SRC-001～006 | 比邏輯索引與行為；Rust格式不要求Lucene bytes一致；保持來源、完整／增量及一致性測試 |
| QRY-001～007、PRJ-001～005 | Rust相容parser/AST/engine、跨project統計、排序、suggester與自有writer locks |
| UI-001～006 | 原CSS/JS/images/fonts hash＋Rust SSR rendered DOM/URL/互動/visual；不要求WAR/JSP執行 |
| SCM-001～003、API-001～004 | 行為契約完整保留；D19使用bundled helpers並保留缺件／錯版負測；舊config/JAR D20仍未決，不靜默改為新API |
| PKG-001～006、OS-001～005 | Mac先完整驗證，再於Ubuntu24.04 amd64適配並完整原生重驗，Windows承諾保留；npm需要核准Node/npm，native ZIP無Node，產品無JVM／Rust工具鏈 |
| REC-001～004 | Rust engine/schema/metadata/reader交換與故障；OOM/panic行為按實作與平台驗證 |
| SEC-001～005 | OS檔案／root邊界及外連檢查保留；SEC-002改驗local/LAN共同可讀，帳號／角色／RBAC／project ACL不再mandatory；管理控制依D16待決 |
| PERF-001～002 | 搜尋速度優先；按查詢類別／project／cache／並行index分報，公平JVM warmup／Rust release比較，G5批准查詢SLO與內網pilot |
| REL-001～005 | rustgrok的GitHub Releases資產metadata／hash／tag與各平台證據、授權、來源閉包；下載同一發行物後驗離線安裝，不宣稱已有binary |
| WIN-001 | 上游Windows排除領域是oracle證據缺口，另測Rust相同行為；不能直接當Rust缺陷 |
| SRV-001、CLI-001、DAT-001 | 自帶RustHTTP、CLI／程序與持久資料；readiness不能只看port |
| SYN-001、MIG-001、OBS-001 | 原同步／遷移待決保留、Rust診斷；舊Lucene/config相容不得猜測 |

## 7. 測試矩陣

`gate`為最早實驗，`acceptance_gate`為完整驗收；全部`release_gate=G6`。下表全數 **planned**，沒有實測完成率。

| Test ID | 需求 IDs | 測試目的 | 最早／驗收 Gate |
|---|---|---|---|
| REF-001 | FR-012, NFR-009, NFR-014, FR-021, NFR-016 | 取得 pinned upstream、release assets、runtime、ctags | G0 |
| REF-002 | FR-001, FR-009, FR-013, NFR-014, FR-021 | 依原版部署，建立 golden indexes | G1 |
| REF-003 | FR-008, FR-011, FR-012, NFR-014, FR-021, FR-022 | 盤點 webapp routes、UI JS handlers、REST controllers、analyzers、SCM | G0 |
| SRC-001 | FR-009, FR-008, FR-021, NFR-016 | 同 corpus baseline/candidate 全量索引 | G2／G3 |
| SRC-002 | FR-009 | 加／改／刪／rename 檔案後增量索引 | G2／G3 |
| SRC-003 | FR-009 | 無變更重索引 | G2／G3 |
| SRC-004 | FR-009, FR-016, NFR-008 | source root read-only；data root 可寫 | G2／G3 |
| SRC-005 | FR-009, NFR-007 | 索引期間檔案持續修改 | G2／G3 |
| SRC-006 | FR-005, FR-009, NFR-001, NFR-008 | symlinks/junctions/case collisions／path escapes | G2／G3 |
| QRY-001 | FR-001, FR-022 | full/defs/refs/path/hist/type 各單欄 | G3 |
| QRY-002 | FR-001, FR-002, FR-022 | full 大小寫、defs/refs 大小寫、單字元／數字／underscore／標點 | G3 |
| QRY-003 | FR-002, FR-022 | phrases、AND/OR/NOT、+/-、parentheses／field grouping | G3 |
| QRY-004 | FR-002, FR-022 | wildcard/leading wildcard 開關、Lucene regex、fuzzy、proximity、ranges、boost | G3 |
| QRY-005 | FR-002, NFR-008, FR-022 | escapes、錯誤 query、過大 query、`/` vs `\\` | G3 |
| QRY-006 | FR-003, FR-022 | 一筆／零筆／多頁結果、maxresults/start/maxHitsPerFile、sort | G3 |
| QRY-007 | FR-010, FR-021 | suggester 初始／查詢後／重啟後 | G3 |
| PRJ-001 | FR-007, FR-003, FR-022 | 同一 symbol，選 alpha／beta／兩者／all | G2／G3 |
| PRJ-002 | FR-007, FR-009, NFR-005, NFR-006 | A reindex 同時搜尋 B，及 A/B 同時更新 | G2／G5 |
| PRJ-003 | FR-007, FR-009, NFR-007, FR-021 | 同 project 兩次同時 reindex；server duplicate start | G2／G3 |
| PRJ-004 | FR-015, FR-016 | 新增／移除／改顯示名／重新加入 project | G2／G3 |
| PRJ-005 | FR-007, FR-006 | 同 project 含多 SCM repo，groups 巢狀與選取 | G2／G3 |
| UI-001 | FR-012, FR-005, FR-007, FR-021 | 首頁、搜尋表單、project chooser、目錄、breadcrumbs | G3 |
| UI-002 | FR-012, FR-004, FR-005, FR-021 | 搜尋結果、xref、syntax coloring、line anchors、下載、raw views | G3 |
| UI-003 | FR-012, FR-004 | Navigate、Scopes、Intelligence window、1/2–8/n/b/Esc | G3 |
| UI-004 | FR-006, FR-012 | file/directory history、annotate、歷史 xref、unified/side-by-side diff、diff jumper | G3 |
| UI-005 | FR-005, FR-012 | URL 含 Unicode／空白／`#`／`%`；context path；重啟後 bookmark | G3 |
| UI-006 | FR-012, NFR-001, FR-021 | 各支援 browser/OS/viewport | G3 |
| SCM-001 | FR-006 | 固定 Git fixture 的 history/annotate/diff | G3 |
| SCM-002 | FR-006, FR-011, FR-021 | 每個已批准 SCM adapter/client | G3 |
| SCM-003 | FR-006, NFR-011 | 無 repo、SCM command missing／timeout／nonzero | G3 |
| API-001 | FR-011, FR-021, FR-022 | 盤點全部上游 API 的 request/response/status | G3 |
| API-002 | FR-011, FR-017, NFR-008 | 讀者API共同可讀與管理端點邊界（D16待決） | G3 |
| API-003 | FR-011, FR-003, NFR-008, FR-022 | query pagination integer overflow/negative/out-of-range | G3 |
| PKG-001 | FR-013, NFR-001, NFR-002, FR-021 | clean supported OS；從 packed tgz 安裝並啟動 | G1／G4 |
| PKG-002 | FR-018, NFR-002, NFR-015, FR-021 | npm install scripts 禁用；外網出口阻斷、只開內網 registry | G1／G4 |
| PKG-003 | FR-018, NFR-003, NFR-015, FR-021 | 完全斷網／空 npm cache，從完整 offline ZIP/tgz bundle 安裝 | G1／G4 |
| PKG-004 | FR-013, NFR-004, FR-021 | 無JVM/Tomcat/Rust工具鏈、非管理員環境的成品啟動 | G1／G4 |
| PKG-005 | FR-016, NFR-001, NFR-002 | npm global/local install、空白／中文／長 install path | G1／G4 |
| PKG-006 | NFR-001, NFR-009, FR-021 | platform/CPU mismatch、corrupt payload/hash mismatch | G1／G4 |
| OS-001 | NFR-001, FR-004, FR-006, FR-010, FR-013, FR-021 | Windows 64 位元原生，不藉 WSL／Docker | G1／G4 |
| OS-002 | FR-014, NFR-001, NFR-007, FR-021 | Ctrl+C、正常 stop、crash、parent exit、port conflict | G1／G4 |
| OS-003 | NFR-001, NFR-004 | Apple M1 macOS原生arm64先完整驗證 | G1／G4 |
| OS-004 | NFR-001, NFR-004 | Ubuntu24.04 amd64適配與完整原生重驗 | G1／G4 |
| OS-005 | NFR-001, FR-009, NFR-012 | Windows file locking/junction/UNC candidate／case-insensitive paths | G1／G4 |
| REC-001 | FR-009, FR-016, NFR-007, FR-021 | full/incremental indexing 的各 commit 階段中斷 | G1／G4 |
| REC-002 | FR-016, NFR-007, NFR-011, FR-021 | disk full、權限錯誤、分析器hang與Rust記憶體／panic故障 | G3／G4 |
| REC-003 | FR-020, NFR-012, FR-021 | app upgrade compatible/incompatible index schema | G4 |
| REC-004 | FR-016, FR-020, NFR-007, NFR-012 | config migration fail、backup restore、跨版本 downgrade | G4 |
| SEC-001 | FR-016, NFR-008 | source/data/config的OS檔案保護（管理secret依D16） | G3／G4 |
| SEC-002 | FR-017, NFR-008 | local／LAN共同可讀，無角色與project ACL | G3／G4 |
| SEC-003 | FR-005, FR-009, NFR-008 | traversal/double decoding/symlink escapes／HTML/shell payload | G3／G4 |
| SEC-004 | FR-018, NFR-008 | runtime outbound monitoring、proxy env、telemetry | G3／G4 |
| SEC-005 | FR-017, NFR-008 | local／LAN讀者與管理控制面邊界（D16待決） | G3／G4 |
| PERF-001 | NFR-005, NFR-006, FR-021, NFR-016 | 搜尋優先：同硬體原OpenGrok與Rust公平分組量測 | G1／G5 |
| PERF-002 | FR-007, FR-009, NFR-005, NFR-006, FR-021, NFR-016 | 使用者內網pilot：各類查詢／並行reindex與批准搜尋SLO | G5 |
| REL-001 | NFR-002, NFR-003, NFR-009, NFR-010 | 發行候選TGZ／ZIP、metadata／checksum與原生安裝 | G4 |
| REL-002 | NFR-002, NFR-003, NFR-012, NFR-014, NFR-015 | run documented quickstart/offline/upgrade steps verbatim | G4 |
| REL-003 | NFR-013, NFR-014 | requirement → design → WBS → test → evidence | G6 |
| WIN-001 | NFR-001, FR-004, FR-006, FR-015, FR-009, FR-021 | 原版Windows證據缺口對應行為的Rust補驗 | G1／G4 |
| SRV-001 | FR-013, FR-011, FR-012, NFR-001, FR-021 | Rust自帶HTTP server與原版行為／UI契約整合 | G1／G4 |
| CLI-001 | FR-014, FR-015, NFR-011 | CLI 契約與錯誤處理 | G2 |
| DAT-001 | FR-016, NFR-002, NFR-012 | 持久資料與安裝目錄分離 | G2 |
| SYN-001 | FR-019, FR-009, NFR-008 | 來源同步能力（需求待決） | G3 |
| MIG-001 | FR-020, FR-016, NFR-012, FR-021 | 既有 OpenGrok 資料遷移（需求待決） | G4 |
| OBS-001 | NFR-011, NFR-008, NFR-014 | 記錄、診斷與證據完整性 | G3 |
| REL-004 | NFR-010, NFR-009, FR-021 | 授權、再散布與來源對應 | G4 |
| REL-005 | NFR-013, NFR-014, NFR-009 | rustgrok GitHub Releases資產與交接可追溯 | G6 |
| API-004 | FR-011, FR-009, FR-015, NFR-007, FR-021 | 非同步配置／專案 API 完成與實際生效 | G2／G3 |
| RST-001 | FR-021, FR-022, NFR-014 | 獨立oracle與分層診斷擷取 | G1／G3 |
| RST-002 | FR-001, FR-002, FR-022 | 相容parser→AST→engine adapter差異 | G1／G3 |
| RST-003 | FR-001, FR-008, FR-022 | 各語言／欄位analyzer、token與Unicode座標 | G1／G3 |
| RST-004 | FR-003, FR-007, FR-022 | scorer／跨project統計與ranking同等 | G1／G3 |
| RST-005 | FR-003, FR-004, FR-005, FR-012, FR-021 | snippet/xref/Rust SSR完整輸出管線 | G1／G3 |
| RST-006 | FR-013, FR-021, NFR-001, NFR-002, NFR-003, NFR-015 | 純產品執行路徑與原生Rust發行物 | G1／G4 |
| RST-007 | FR-002, FR-008, FR-022, NFR-008 | query／analyzer／路徑的property與differential fuzz | G2／G3 |
| RST-008 | FR-021, FR-022, NFR-005, NFR-006, NFR-016 | 搜尋關鍵路徑的輸入規模、allocation與熱點複雜度 | G1／G5 |
| RST-009 | FR-009, FR-016, FR-021, NFR-007, NFR-012 | Rust索引格式與完整／增量等價 | G2／G4 |
| RST-010 | NFR-005, NFR-006, NFR-016 | 搜尋分組與cold/warm cache的公平跨語言benchmark | G1／G5 |
| RST-011 | FR-011, FR-020, FR-021, NFR-012, NFR-014 | 舊config/index/JAR與Rust相容邊界 | G0／G3 |
| RST-012 | FR-021, NFR-005, NFR-006, NFR-016 | 搜尋收益、慢查詢與整體成本的宣稱證據審查 | G1／G5 |

## 8. Rust v0.6 Gates

| Gate | 與舊封裝方案的差異 | 必要證據 |
|---|---|---|
| G0 需求 | Rust、bundled helpers、搜尋優先、local＋LAN、約5人與無權限管理已確認；Mac先開發驗證，Ubuntu24.04 amd64部署優先；repo名rustgrok | D03 macOS實際／最低版本、Windows與剩餘環境；已建公開repo的內容提交／回讀證據、D16管理、D20相容及具體SLO待決 |
| G1 最難語意與效能可行性 | 不再以「WAR啟動成功」當核心可行；提前驗parser/analyzer/scorer/xref以及公平收益 | 獨立oracle、難query／sparse fields／跨project／Unicode片段、原生Rust slice、startup/index/query/RSS等基準與profile；沒有倍數承諾 |
| G2 Rust垂直流程 | install→init/project→Rust index→RustHTTP→原UIsearch/xref/history→stop/restart | exact npm prototype、持久資料／writer控制、Rust核心執行證據；僅里程碑不是完整驗收 |
| G3 全parity | 本版確認範圍內的語言/SCM/plugin/config/API與UI逐項移植／相容證據 | 原68及RST語意測項完整，含共同可讀正向契約；不把已取消mandatory的RBAC留作永遠阻擋；D16/D20差異結案 |
| G4 打包可靠性 | 標準Rust bundled profile含必要helpers，不帶Java；Mac先驗後Linux完整原生重驗，Windows最終驗收保留 | 各平台npm/ZIP空cache離線、安全、crash/upgrade、授權/SBOM與來源；native ZIP無Node；各自artifact hash與證據，不共用PASS |
| G5 user SLO pilot | 搜尋優先；Ubuntu24.04 amd64部署pilot對照1人與約5人、共用cache及背景index | 具體搜尋SLO與負載先批准；同平台公平原版比較、全process成本與user acceptance；Mac結果不代替Linux |
| G6 交接 | rustgrok實際repo確認後，GitHub Releases最終包需與Rust commit/Cargo lock/build target/evidence一致 | 回讀Release資產metadata／bytes／checksum，核對各平台證據與離線閉包、SBOM/notices/source；npm交由使用者上傳 |

G1發現語意缺口時先修相容層或提出需要決策的成本，不把近似功能當成同等。G4只表示發行可靠性，沒有取代G5使用者驗收。G6若artifact改變，重跑受影響項及必要整合驗證；不可挪用舊版本報告。

## 9. 證據、目錄與發布條件

測試目錄與[Monorepo規劃](07-monorepo-and-github.md)一致：`tests/reference/`（獨立原版部署/診斷）、`tests/fixtures/golden/`、`tests/fixtures/queries/`、`tests/contract/`、`tests/parity/`、`tests/visual/`、`tests/install/`、`tests/offline/`、`tests/platform/`、`tests/recovery/`、`tests/performance/`、`tests/fuzz/`、`tests/evidence/`。Rust microbench放對應crate的`benches/`，跨產品HTTP／job量測放`tests/performance/`，兩者報告分開。`tests/evidence/`保存schema與證據索引；matrix中的`evidence/<release>/<run>/...`是run artifact內的相對路徑範本，大型輸出不放進Git。

npm安裝情境事先提供核准Node/npm及內網registry或完整離線TGZ閉包；若入口是JS launcher，執行時亦依該launcher前提驗證。native ZIP直接啟動Rust binary，必須另外在未安裝Node/npm的乾淨環境完成指定流程。兩者都必須附帶功能所需helpers，不得借用使用者預裝ctags/SCM、Java、Rust工具鏈或外網補件；系統ABI前提與bundled依賴閉包分別列清。

每run manifest記錄：git commit、原版tag/SHA、Rust/Cargo及crate lock、release profile/CPU target/allocator、REF JVM/Tomcat/ctags/SCM參數、bundled helpers清單/版本/hash、各artifact/hash、精確OS版本/CPU/RAM/磁碟/檔案系統、平台執行階段（Mac完成／Linux重驗／Windows原生）、local或LAN拓撲、1人或約5人情境、實際client數/request率/in-flight/think-time、power/cache/warmup條件、corpus/config hash、query ID/類別/project範圍/到達模型、共用cache hits/misses、index並行情境、server/browser計時邊界、timeouts/慢查詢、test IDs、結果、raw evidence、批准例外及時間。

Diagnostics至少包含parser AST/plan、field tokens/positions/offset座標、index statistics/explain、snippet spans、xref/SSR差異與CPU/alloc/I/O profiles；只限合法fixture或使用者批准的匿名結果。大量trace/profile可用GitHub artifacts/Release資產，summary/checksums/traceability入git，企業資料留內網。

發行包指定放GitHub Releases，不能只留在有期限的CI artifacts。REL-001/002於G4驗發布候選的完整檔案與操作；REL-005於G6回讀rustgrok實際Release的tag／commit、asset名稱／ID／大小／類型、OS/arch、版本及checksum，下載後與候選bytes核對，必要時重跑PKG-001/003的同一離線安裝流程。每個npm TGZ／portable ZIP都對應其native平台、helpers、SBOM／licenses與必要source；不能把GitHub自動source archive當可執行包。repo已確認public且建立完成；正式Release發布後驗公開下載，現在產品未實作，不能宣稱binary已發布。

Release-ready需mandatory測項有對應實測結果，不能有未說明planned/blocked/failed/skipped；依已確認local＋LAN／共同可讀、D19 bundled及D21搜尋優先驗收。macOS實際／最低版本、Windows／硬體等剩餘環境、D16管理邊界、D20及具體SLO／負載等必要決策先確認；已建公開repo另補齊內容提交與正式Release回讀證據，不重問owner或visibility。移除mandatory RBAC後以共同可讀測試取得證據，不用skip算passed。80項規格有追蹤不等於80項已通過。本版只檢查文件結構，沒有執行Rust產品或性能驗收。

## 10. 官方來源與已讀原始碼

- [S1] [OpenGrok pinned POM](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/pom.xml)。
- [S2] [CustomQueryParser](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/CustomQueryParser.java)、[QueryBuilder](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/QueryBuilder.java)、[Lucene9.12.3 QueryParser](https://lucene.apache.org/core/9_12_3/queryparser/org/apache/lucene/queryparser/classic/package-summary.html)。
- [S3] [OpenGrok SearchEngine](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/SearchEngine.java)。
- [S4] [Tantivy0.27.0 tokenizer manager](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/tokenizer/tokenizer_manager.rs)、[BM25](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/bm25.rs)。這是研究候選，非已批准採用。
- [S5] [Lucene9.12.3 BM25Similarity](https://lucene.apache.org/core/9_12_3/core/org/apache/lucene/search/similarities/BM25Similarity.html)。
- [S6] [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)。
- [S7] [OpenJDK JMH](https://openjdk.org/projects/code-tools/jmh/)、[JVM穩態量測研究](https://arxiv.org/abs/2209.15369)。
- [S8] [Criterion輸入規模量測](https://bheisler.github.io/criterion.rs/book/user_guide/benchmarking_with_inputs.html)。
- [S9] [Rust regex時間複雜度](https://docs.rs/regex/latest/regex/struct.Regex.html)。
- [S10] [Rust profiling](https://nnethercote.github.io/perf-book/profiling.html)、[Heap allocations](https://nnethercote.github.io/perf-book/heap-allocations.html)。
- [S11] [OpenGrok UI](https://github.com/oracle/opengrok/wiki/User-Interface)、[Supported Languages](https://github.com/oracle/opengrok/wiki/Supported-Languages-and-Formats)、[SCM能力](https://github.com/oracle/opengrok/wiki/Supported-Revision-Control-Systems)。移動Wiki只作盤點來源，最終以pinned source與實跑確認。
- [S12] [Playwright visual comparisons](https://playwright.dev/docs/test-snapshots)。
- [S13] [OpenGrok GitRepository／JGit](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java)。

- [S14] [Rust Miri](https://github.com/rust-lang/miri)、[Rust sanitizer文件與支援範圍](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html)。
