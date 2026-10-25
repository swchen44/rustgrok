# 搜尋速度優先：設計、實驗與驗收計畫

版本：v0.6；2026-10-25（Asia/Taipei）。使用者決策原文：「A+搜尋速度」。此文件細化既有需求與工作包，所有產品實作與測試仍為 planned／not_run。

## 1. 已確認與未確認的界線

| 項目 | 已確認 | 尚未確認 |
|---|---|---|
| D01 | 核心Rust重寫 | 具體engine/framework與版本 |
| D19 | A方案：必要Ctags／Git／其他SCM工具隨包；bundled為主要產品profile | 精確工具清單、build、能力、各平台依賴、再散布條件 |
| D21 | 搜尋速度第一優先 | 代表query mix、資料量、硬體、到達率、數值SLO與可接受資源代價 |
| D02／D09 | local＋LAN、約5同時使用者、無角色／專案ACL | 實際request concurrency／QPS、使用間隔與管理入口 |
| D03 | macOS先開發及完整驗證，Ubuntu24.04 amd64為主要部署並完整原生重驗 | Ubuntu tuple與Mac M1／arm64已確認，Mac OS版本／Windows細節未定 |
| 功能與style | 原要求完整保留 | D20對舊index/config/JAR等相容細節仍待盤點；未允許命中或排序降級 |

選A後，企業預裝工具profile（B）與不啟動helpers（C）不再是必須同時交付的產品方案。必要工具缺件、版本不符、崩潰等仍是負向測試。若某平台無法完整附帶必要工具，要提出具體阻擋，不能默改為請使用者自行安裝或連外下載。

搜尋優先不等於停用其他功能，也不等於允許增加任意RAM、磁碟、索引延遲或資料陳舊時間。這些代價必須量測並討論。

## 2. 把「搜尋速度」拆成可定位的時間

以下量測方式為工程提案，數值目標保留空值：

| 層次 | 定義 | 報告用途 |
|---|---|---|
| Browser可見結果 | 使用者送出查詢到結果列表、行號／片段及必要互動可用 | 接近使用者實際等待；沿用原UI而不修改style |
| HTTP端到端 | client送出到收到完整該頁HTML／JSON body | 包含傳輸與服務端成本；與browser rendering分報 |
| Server排隊 | 接收請求到取得實際查詢執行資源 | 看background index／並行負載是否阻塞搜尋 |
| Query準備 | field escape、parser、AST、analyzer與engine lowering | 定位重複解析、分配與相容層成本 |
| Search／ranking | postings／term expansion、scoring statistics、排序、total count／Top K | 保持原版命中、count、排名與分頁契約 |
| Result／snippet | 取得path／line／source、highlight、片段與必要metadata | 防止只量引擎而漏掉使用者等待的大部分工作 |
| Response render | view model、原DOM模板或API序列化、輸出 | 驗證Rust SSR不以縮減內容換取時間 |

每層用其程序的單調時鐘；不要把不同主機或瀏覽器的wall clock直接相減。並行stage可能重疊，報critical path與各stage spans，不能將重疊時間相加冒充總延遲。

建議核心報表為各分組p50/p95/p99、樣本數、timeout／error／cancel數、吞吐與排隊。p95/p99的數值門檻、樣本量／重複run與可接受回歸範圍需在代表環境中定案；沒有數值時只報測量，不判PASS。

## 3. 查詢工作負載矩陣

不要只重複一個最容易命中cache的query。每組都要有獨立原版expected與同等Rust結果：

| 維度 | 至少覆蓋情境 |
|---|---|
| Query類型 | 全文、defs、refs、path、history欄位；Boolean／phrase／wildcard／regex／fuzzy／range／boost依原版inventory |
| 選擇性 | 無命中、少量、高命中、常見term、稀疏defs；長token與Unicode |
| Project範圍 | 單project、選定多project、全部適用projects；同名symbol與稀疏欄位統計 |
| 排序／頁面 | 原版各排序、首頁與後續頁；count／score ties／分頁正確性 |
| 快取狀態 | process啟動後首查、application cache冷／暖、OS page cache已知狀態；hit／miss／eviction分報 |
| 資料更新 | 無索引工作、增量index中、full index或merge中、generation切換後 |
| 負載 | 1人參照與約5人互動client；到達率／request concurrency待定，慢／一般query混合、取消與timeout |
| 輸入與環境 | D06語言／SCM、D09規模、D03原生OS/CPU與磁碟；不要默認C-only或小型repo |

正式混合比例從使用者代表查詢或經其確認的公開合成樣本建立；目前query權重為null。G1可用公開小subset定位成本，但不能把subset的收益宣稱完整產品加速。

若無法可靠控制OS page cache，標示實際觀察條件，不把清除application cache或重啟程式等同完整cold。兩邊相同原則，操作不得依賴未批准的管理權限。

## 4. 搜尋路徑的候選優化與限制

### 4.1 重用reader與generation

一般全文／defs／refs／path搜尋使用已發布索引及必要metadata，不每次重新建立索引、掃描全部source或啟動Ctags。reader／searcher採穩定可重用設計，每個查詢引用一致generation和scoring scope。

來源片段、raw、xref與歷史需要的實際讀取依既有契約處理。不能因要減少I/O而讓snippet顯示錯誤版本。history／blame／diff可能需要SCM呼叫，另設cache與timeout，不把一般搜尋路徑的設計推廣成所有操作禁止外部程序。

### 4.2 有界cache

分別量測parser／analysis cache、查詢結果、snippet／xref／history cache是否值得保留；不預設全部建置。

共通key需要涵蓋必要語意：原query的正確canonical form、field、selected projects、sort/page、generation、analyzer/config版本與來源revision；D02已不需逐使用者／project ACL，不建立多租戶權限cache分區。未證等價的query canonicalization不得把不同查詢合併。

generation/config/已註冊project集合變更時，舊entry不能用於新狀態；容量、生命週期、回收與命中率必須可觀測。cache不足時回到正確計算，不回傳近似／過期結果。容量值經profiling與D09資源預算決定。

### 4.3 搜尋與索引資源控制

限制index workers、Ctags/SCM子程序、merge、I/O與記憶體的並行。CPU密集工作不直接長時間占用HTTP async handler，也不讓長query持有整個服務的全域鎖。

比較「只有讀取」、「增量更新」、「full/merge」的分組延遲，同時記錄source變更到新generation可查的時間。搜尋優先的排程應避免互相餓死；具體配額、降速或暫停策略須看數據，不先接受index無限延後。

### 4.4 不改語意的引擎優化

profile之後再比較allocation/copy減少、批次metadata讀取、預計算可穩定統計、reader重用、有效Top K與skip策略。每個優化都附parity regression。

跨project statistics、exact total hits、排序、ties、分頁、phrase／fuzzy／regex等原版契約仍是限制。單一index＋project filter與獨立index Top K合併都須先證明相同統計／排名；不能因較快就選擇不相容方案。背景詳見[12相容性研究](12-rust-compatibility.md)。

## 5. G1可執行實驗順序

| 順序／既有WP | 輸入與動作 | 必須留下的產物 |
|---|---|---|
| WP-A03/A05/A07 | 定query inventory與公開corpus；分清使用者規模未知與prototype限制 | query/corpus manifest、expected來源、未覆蓋清單 |
| WP-A06/A08 | 獨立固定OpenGrok、合理JVM設定、原HTTP/UI與分層診斷 | oracle manifest、raw結果與normalization白名單 |
| WP-B01/B02 | Rust release執行同corpus，先對query/tokens/hits/order/count | 每組相容報告；不相容group不得算同等速度收益 |
| WP-B03 | 原CSS/JS加Rust SSR，測完整搜尋頁而非只有engine | DOM／互動結果、browser/HTTP分層量測 |
| WP-B06 | reader／queue／generation與並行index實驗 | spans、queue、index進度／freshness及cancel/recovery證據 |
| WP-B07 | 成對重複run與profile；變更一個主要策略後回歸 | raw samples、按組延遲／timeout、RSS/IO、瓶頸與成本 |
| WP-B08 | 依上述證據選engine／cache／排程／helper adapter並更新估算 | ADR、未決tradeoff及進入G2的條件 |

正式全功能／原生發行包的驗收由WP-E07／E09完成，不能把G1小subset通過搬成G5結果。Ctags/Git等bundled產物另以WP-B04/B05與G4平台／離線測試驗證。

## 6. 比較規則與對外結論

1. 同一corpus/config/project選擇/query/result/page；Rust與原版feature coverage相同才可計算同等速度比率。
2. 固定硬體、資源預算、電源／背景負載與cache條件；原版合理調校並記warmup，Rust是可發行release build。交錯或配對run，避免單次偶然狀態。
3. 記實際送出時間、排隊與完成時間；在批准的到達模型下量測。失敗、timeout、取消與未完成請求都保留分母與狀態，不只統計成功快查詢。
4. 不混合不同query組的p95來計算「平均p95」；按組報，再用已確認mix另做整體run。
5. 區分server加速、HTTP加速、browser體感加速；若snippet或render抵銷engine改善，如實呈現。
6. 全程序樹的RSS、CPU、IO、index/cache/temp/backup大小，以及新鮮度都記錄。尚無批准取捨，不把資源變大當自動可接受。
7. 更新後第一次query與cache miss也要報；熱門query暖cache的數字不可代表全部。
8. exact功能／排序先過；性能未定門檻只標MEASURED_NOT_ACCEPTED或NOT_RUN，不以更換語言直接PASS。

## 7. 驗收值與證據模板

| 欄位 | v0.6狀態 |
|---|---|
| 優先指標 | 搜尋速度，USER_CONFIRMED |
| primary helper profile | bundled_required_helpers，USER_CONFIRMED |
| 查詢分組／比例 | 待D06/D09代表資料 |
| client／server延遲門檻 | null，待批准 |
| 使用模式／人數 | local＋LAN、約5人已確認，1人為比較baseline |
| cold／warm、request並行／吞吐目標 | null；5人不換算成5QPS或5個同時requests |
| RSS／CPU／磁碟／freshness上限 | null，待D09/D12/D21 |
| 開發／產品測試結果 | not_run |
| 功能與排序差異容忍 | 沒有新增批准；沿用原同等要求 |

機器可讀模板在[search-benchmark-manifest.template.json](../tests/performance/search-benchmark-manifest.template.json)。它不是實測結果，也不是正式runtime設定。對應既有PERF-001、PERF-002、RST-008、RST-010、RST-012，需求為FR-003、FR-007、NFR-005、NFR-006、NFR-016。38需求、44工作包、80測試總數不變。

## 8. 決策歷史與v0.6下一輪問題

v0.3已確認A與搜尋優先；v0.4加入雙模式約5人、無權限管理。v0.5確認Ubuntu24.04 amd64為主要部署，但開發及完整首輪驗證改在macOS，其後Ubuntu完整原生重驗。兩平台的原版/Rust公平比較各在同一硬體進行，不以Mac結果推算Linux改善倍數。

v0.6已確認M1／arm64與Public，repo已建立；接著補D03的macOS實際／最低版本。Ubuntu24.04 amd64、Mac先驗、架構、rustgrok名稱／可見性及GitHub Releases不再重問。之後收D09資料量／硬體、D04原版及D16管理入口。約5人已知，不要求重答人數，也不換成假QPS。
