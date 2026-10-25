# 軟體需求規格書 SRS

版本：v0.6，2026-10-25。**Rust、bundled helpers、搜尋優先、本機＋LAN約5人且不用權限管理、Ubuntu24.04 amd64部署、macOS先開發驗證、rustgrok及GitHub Releases已確認；M1／arm64及Public repo已確認；macOS版本、技術選型與數值門檻待決。**

新增使用者原文：「我還是想要改寫rust 因為效能比較好」。這確認實作語言與效能動機，沒有批准任何特定速度倍數、Tantivy/Tree-sitter選型、功能裁減或排序差異。v0.1的Java/Tomcat產品包裝方向已取代；原版Java只作參照系統。

新增決策原文：「A+搜尋速度」。D19選bundled必要helpers；D21搜尋速度第一優先。此答覆沒有選定具體版本、授權自動sync，或批准以不完整結果／近似排名／無限資源換取速度。

D02已確認本機與LAN雙模式、約5人代表負載、不做角色／project ACL。D03最新確認Ubuntu24.04 amd64為主要部署目標，先在macOS開發及完整驗證，再Ubuntu平台實作／適配及完整原生重驗。D11確認Public repo rustgrok，已建立於swchen44帳號，release packages放GitHub Releases。首輪Mac另確認M1／arm64；macOS實際／最低版本仍待提供。

## 1. 狀態與來源

- CONFIRMED：本次使用者明確說明的需求。
- DERIVED：把使用者的「同原版」或交付目標拆成可驗證項，詳細範圍待基線確認。
- PROPOSED：工程建議，使用者尚未批准。
- CONDITIONAL：只在某項決策選定後適用。
- OPEN：缺少會改變設計的資料，禁止默認填入。

「未定」不等於不做。尤其不能因使用者尚未列出全部 SCM／語言，就默默把原版功能縮成 Git-only 或 C-only。

## 2. 使用者原始目標對照

| ID | 原始目標 | 狀態 | 展開需求 |
|---|---|---|---|
| U01 | OpenGrok 功能一模一樣 | CONFIRMED，版本待定 | FR-001–FR-011 |
| U02 | 所有風格與 style 照原版 | CONFIRMED | FR-012 |
| U03 | host版本、自帶Web Server；本機＋LAN均支援 | CONFIRMED，D02 | FR-013、FR-014、FR-017 |
| U04 | 同時使用多個專案 | CONFIRMED，並行含義待定 | FR-007、FR-009、FR-015 |
| U05 | 內網可用，最後 npm 打包 | CONFIRMED | FR-018、NFR-002、NFR-015 |
| U06 | macOS／Ubuntu24.04 amd64／Windows64；Ubuntu為主要部署目標 | CONFIRMED平台與Ubuntu CPU；Mac OS版本／Windows細節OPEN | NFR-001 |
| U07 | 可提供離線 ZIP | 候選交付，細節待定 | NFR-003 |
| U08 | 新GitHub Monorepo rustgrok含docs/tests，發行包放GitHub Releases | CONFIRMED名稱、Public與交付平台；repo已建立 | NFR-013、NFR-014 |
| U09 | 詳細實作與驗證計畫；決策先討論 | CONFIRMED | 全部文件、D01–D21 |
| U10 | 最後由使用者上傳 npm | CONFIRMED | 發佈包可交付；不自行公開 npm 發佈 |
| U11 | 使用Rust重寫，目的是改善效能 | CONFIRMED方向；效能提升尚未量測 | FR-021、FR-022、NFR-016 |
| U12 | 選A：Rust核心附必要Ctags／SCM工具 | CONFIRMED策略；具體build與能力待驗 | FR-008、FR-018、NFR-002、NFR-003、NFR-009、NFR-010、NFR-015 |
| U13 | 效能優先搜尋速度 | CONFIRMED優先順序；數值SLO仍OPEN | FR-003、FR-007、NFR-005、NFR-006、NFR-016 |
| U14 | 兩種模式、約5人、不用權限管理 | CONFIRMED；約5人是代表互動負載 | FR-017、NFR-005、NFR-008 |
| U15 | 先macOS開發及完整驗證，再Ubuntu24.04 amd64適配與完整原生重驗 | CONFIRMED；M1／arm64已確認，macOS版本待決 | NFR-001 |

## 3. 使用情境

### UC01 首次安裝與啟動

使用者取得核准平台的 npm 套件或 ZIP，執行診斷、建立 workspace、加入 source projects、初次索引、開啟原版網頁。安裝不應假裝成功卻等到啟動才隱性連外補檔。必要Ctags／SCM工具按D19已確認的A方案隨包；一般使用者權限與企業native執行政策仍依D07核對，完整build／依賴按平台驗證。Rust可執行檔在建置端預編譯，使用者不需要安裝Rust toolchain；npm入口形式仍待分發實驗。

### UC02 搜尋與閱讀

選一或多個 project，執行原版查詢，開啟結果與定義／參照，閱讀檔案、raw、history、annotate、diff。查詢與 UI 的 observable behavior 以確認後的原版 instance 為 oracle。

### UC03 索引更新

使用者修改檔案或外部工具更新 checkout 後，執行增量或完整索引。更新頻率、手動／排程／watch 與讀取一致性由 D08/D12 決定；「多專案同時可查」不能推論成「每個專案無上限平行重建」。

### UC04 內網共用

D02已確認本機與LAN共用，約5人同時使用，不做使用者／角色／project ACL管理。讀者共同存取服務內已註冊專案，保留原版搜尋與閱讀介面。免登入閱讀為工程提案，不新增必需帳號／SSO。D16另決定專案、索引、設定及stop等管理操作是否開LAN；主機CLI／本機控制面為建議方案，尚未批准。

### UC05 更新、移機與復原

安裝新包或 ZIP，保留設定與資料，檢查索引格式相容性，必要時重建；失敗時有可實際執行的回復路徑。直接 npm downgrade 不等同索引可回復。

## 4. 功能需求

| ID | 需求 | 狀態／決策 | 驗收要點 |
|---|---|---|---|
| FR-001 | 保留全文、定義、參照／symbol、path、history 等原版欄位 | DERIVED；D04 | 同一 corpus/query/project 選擇，命中集合與欄位語意符合原版 |
| FR-002 | 保留 Lucene/OpenGrok 查詢語法與分析差異 | DERIVED；D04 | Boolean、phrase、wildcard、regex、fuzzy、range、escaping、大小寫及限制逐項對照；無效 query 同等處理 |
| FR-003 | 搜尋結果、排序、分頁、matching lines、highlight、snippet | DERIVED | 比較 total hits、路徑、行號與次序；不以任意排序掩蓋 ranking regression |
| FR-004 | 原版 cross reference、definitions/references、symbols 快捷跳轉 | DERIVED | 同名 symbols、不同 scope／project、macro 等公開 fixture；不承諾超越原版的語意分析 |
| FR-005 | 目錄、檔案、行號、raw/download、permalink、navigation | DERIVED | URL 編碼、行號 anchor、中文／空白檔名與返回行為 |
| FR-006 | 原版支援的 history、annotate/blame、file/directory history、各種 diff | DERIVED；D06 | 在對應 SCM 可用時對照；無 SCM 需明確能力狀態，不能當已達全功能 |
| FR-007 | 一個服務保有多 projects；可依原版選擇跨 project 查詢 | CONFIRMED/DERIVED；D05 | A/B 同名函式、A-only/B-only/A+B；project identity 不因顯示名稱或路徑格式混淆 |
| FR-008 | 以Rust重建原版analyzer／language行為；D19已允許隨包Ctags，具體adapter與build待驗 | DERIVED；D06 | 建立完整 inventory；使用者語言加強測，其他原版能力不得無聲刪除 |
| FR-009 | 初次、增量、全量、單 project 索引與失敗狀態 | DERIVED；D08/D12 | 新增／修改／刪除／rename 後正確；相同 index 禁止競爭 writer；read/update 行為須符合選定政策 |
| FR-010 | 原版 suggester／autocomplete 行為 | DERIVED；D04 | 固定字典、查詢歷史與 popularity，保留能力或明確阻擋發佈；不偷偷停用 native-dependent 分支 |
| FR-011 | 原版REST、配置與適用插件相容邊界 | DERIVED；D02/D06/D16/D20 | 使用者權限管理／authorization plugins不列mandatory；其餘端點、格式、錯誤與非權限plugins保留盤點。管理API可達性及控制依D16，不默刪既有automation |
| FR-012 | 全部可見 style 與操作沿用確認後的原版 | CONFIRMED；D04 | 原CSS/JS等可沿用資產hash，加上Rust模板輸出的DOM、互動、同平台screenshot；JSP需移植動態邏輯，不能要求直接執行WAR；不私自改版 |
| FR-013 | 自帶可啟動的 Web Server，不要求另行架 Apache HTTP Server | CONFIRMED；D01採Rust | Rust內建HTTP server於乾淨目標環境啟動；原路由、靜態資源、動態頁面、404/error與API行為完整 |
| FR-014 | 提供啟動、停止、狀態、診斷等 host lifecycle 入口 | PROPOSED | 可重複 start/stop、不殺錯 process、不遺留孩子程序；命令名待 CLI 設計凍結 |
| FR-015 | 專案加入、列出、移除、索引操作 | DERIVED/PROPOSED；D05 | 設定與 runtime 同步；移除產品註冊與刪除使用者 source 是不同動作；不默默刪 source |
| FR-016 | 設定、索引、cache/log 與 npm 安裝目錄分離並持久保存 | PROPOSED | npm upgrade/uninstall 不刪 workspace；原始碼路徑與索引可清楚識別 |
| FR-017 | 同一產品支援本機與LAN共用，無使用者／角色／project ACL管理 | CONFIRMED；D02，管理入口D16 OPEN | loopback與明確LAN listen都驗；共同讀取已註冊projects的search/raw/xref/history/suggester/read APIs；不把不用權限管理解讀成任意管理操作皆對LAN開放 |
| FR-018 | 內網執行時不需要外網服務 | CONFIRMED/DERIVED；D07 | 阻斷外網後完整指定流程；不自動下載CDN、fonts、native binary、ctags或送telemetry |
| FR-019 | 自動 sync、scheduled indexing、filesystem watch | OPEN；D08 | 三者分別決策；索引不代表授權 git pull、reset 或修改 checkout |
| FR-020 | 匯入既有 OpenGrok 設定／索引／書籤的相容或遷移 | OPEN；D10 | 決定是否要保留舊URL、配置與index；Rust/Tantivy新索引不可宣稱直接讀Lucene，重建／匯入需求需D10/D20確認 |
| FR-021 | 搜尋、索引、Web Server、host管理主體以Rust重寫 | CONFIRMED；D01 | 產品核心為原生Rust實作；不能僅用Rust外殼啟動原Java backend。Java/Tomcat可留在獨立驗證環境 |
| FR-022 | 明確的OpenGrok查詢相容parser／typed AST／搜尋引擎adapter | PROPOSED，服務既有同等要求 | 原始query、欄位分析、positions/offsets、排序與錯誤逐層對照；不將Tantivy預設parser直接視為相容完成 |

## 5. 非功能需求

| ID | 需求 | 狀態 | 驗收方向 |
|---|---|---|---|
| NFR-001 | macOS先開發驗證；主要部署Ubuntu24.04 amd64；保留Windows64 | CONFIRMED順序與Ubuntu tuple；Mac OS版本／Windows細節OPEN | Mac全部適用功能／介面／包裝驗證後，Ubuntu以真包完整原生重驗；各平台證據分開，Windows不用WSL替代 |
| NFR-002 | npm 可安裝的最終產品 | CONFIRMED；scope/name/npm version OPEN | 測實際 npm pack 的 .tgz；安裝版，不只 source checkout |
| NFR-003 | 離線 ZIP 交付 | PROPOSED，使用者允許 | native ZIP可直接執行而不依賴Node；helpers、啟動方式、架構分包、executable bits與Gatekeeper依D13/D19驗收 |
| NFR-004 | 一般使用者權限安裝及運作 | PROPOSED；D07 | 不能用admin裝好的Git/ctags/shared libraries、symlink權限或系統PATH遮掉依賴；產品不需JRE |
| NFR-005 | 搜尋速度第一優先；首頁、xref與索引性能同時記錄 | CONFIRMED優先；數值OPEN，D09/D21 | 1人參照＋約5人互動負載；按query/project測server／browser latency、cold/warm、p50/p95/p99、queue、timeouts與並行索引；人數非QPS／請求硬上限，門檻待批 |
| NFR-006 | 記憶體、磁碟、CPU、套件大小限制 | OPEN；D09 | 分別記Rust server/index workers、npm launcher（如常駐）、ctags/SCM、總RSS及index/temp/backup peak bytes；原版Java成本獨立記錄；不可虛構上限 |
| NFR-007 | 崩潰、取消與重啟可診斷可復原 | PROPOSED | ctags hang、Rust程序panic/abort/OOM、disk full、kill/reboot、殘留 PID／lock、最後成功索引狀態 |
| NFR-008 | 本機／LAN資料根目錄、程序與管理邊界 | DERIVED/PROPOSED；D02已定，D16待決 | 路徑traversal/symlink、source escaping、process identity及低權限仍驗；不要求RBAC/專案ACL。控制面token／Origin等僅依D16實際介面決策驗證 |
| NFR-009 | 可追溯、固定版本、完整依賴的供應鏈 | PROPOSED | hashes、SBOM、build來源與編譯參數；不得用 latest 或未核實下載器發佈 |
| NFR-010 | 遵守原版與各 runtime 的再散布授權 | 必要工程／發佈條件 | notices、CDDL modifications、必要 source/build instructions、Rust crates、來源移植、ctags/Git與native libraries各自條件 |
| NFR-011 | 可定位故障的 log 與診斷，避免暴露 source/secrets | PROPOSED | job id、階段、exit、原生binary與工具版本、明確支援包；token 與企業程式碼不入一般 log |
| NFR-012 | 可控升級、保留資料與已驗證 rollback | PROPOSED；D12 | 相容矩陣、冷備份／shadow策略、Windows file locks、損毀及重建情境 |
| NFR-013 | 新GitHub Monorepo rustgrok管理產品/docs/tests/規劃與歷史，發行包放GitHub Releases | CONFIRMED名稱、Public及Release目的地；D11已決 | 建立後固定repo URL與commit；tag、assets、hash、source／tests相符；核實Releases可見性與可下載性 |
| NFR-014 | 每項需求可追溯至設計、工作包、test、驗收證據 | CONFIRMED/DERIVED | 每項在 tests/matrix.json 有測試或明確 pending 決策；未測不算 PASS |
| NFR-015 | 離線安裝不依賴外網或先前 cache | DERIVED/PROPOSED；D07 | 乾淨 cache、網路阻斷、ignore-scripts、完整 transitive deps；build 離線另外決策 |
| NFR-016 | 以公平、可重現的工作負載證明Rust搜尋速度改善並揭露資源代價 | CONFIRMED搜尋優先／PROPOSED方法；D09/D21 | 同功能／排序／結果下對照合理調校的原版與Rust release；查詢類別分報、記所有timeout及RSS/IO/freshness成本，未量測不得宣稱提升倍數 |

## 6. 平台支持矩陣待補欄位

| OS | 已知 | 還需要使用者資訊 | 不能擅自替代 |
|---|---|---|---|
| macOS | 首輪開發／完整驗證為M1 Apple Silicon arm64 | macOS實際／最低版本；是否另交Intel x64尚未指定 | 原生arm64執行，Rosetta x86_64結果分開記錄 |
| Ubuntu | Ubuntu24.04 amd64主要部署目標；接續Mac完整驗證 | desktop/server、CPU型號／cores、RAM與磁碟；實測記錄24.04.x映像 | amd64已確認；Mac PASS或ubuntu-latest名稱不能替代Ubuntu實際執行證據 |
| Windows | 64 位元，本計畫候選為 x64 | Windows 10／11／Server 版本；PowerShell/cmd 限制；管理員權限 | WSL／Docker／Git Bash 不代替原生產品支持 |
| Browser | 沿用 OpenGrok | Chrome／Edge／Firefox／Safari，版本與企業政策 | Linux Chromium 圖片不代替所有 OS/browser |

## 7. 性能與容量驗收契約（空值保留）

| 項目 | 使用者提供／測試計畫 |
|---|---|
| projects 數、repository 數、source files／GB／LOC、history 年數 | OPEN，不以單一小 fixture 推論企業規模 |
| CPU 型號／cores、RAM、SSD/NAS、抗毒軟體 | OPEN，記錄詳細測試 host |
| 同時使用人數、每秒queries、index concurrency | 約5人已提供；互動到達率、實際同時requests與index concurrency仍OPEN，不以5人直接填5QPS |
| cold/warm query延遲及其他SLO | 搜尋優先已確認；所有數值OPEN。按查詢分組報p50/p95/p99、timeout/error rate、樣本數；startup/xref/index/reindex作持續觀測 |
| 可接受停止服務時間、rollback RTO/RPO、資料新鮮度 | OPEN |
| 額外 runtime／索引／備份磁碟預算 | OPEN |

提出實測曲線與原版比率後再凍結門檻；不得先填「<100 ms、8 GB 以內、100 projects」之類未批准數字。

## 8. 定義完成

使用者批准關鍵決策；功能 inventory 不留無說明缺項；同版本原版與產品差異全部有結論；所有承諾 OS/CPU 的發行包通過安裝、運行與失敗恢復；內網／離線、授權及 source availability 完整；性能 SLO 已批准且達標；docs/tests/證據推至指定 GitHub；產出可供使用者 npm publish 的 artifacts。只通過部分 gate 的版本只能標 alpha/beta，不能稱功能完全相同。
