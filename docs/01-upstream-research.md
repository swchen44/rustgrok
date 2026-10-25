# OpenGrok 上游研究與相容性基準

- 文件版本：v0.6
- 研究日期：2026-10-25（Asia/Taipei）
- 狀態：使用者已選擇 Rust 核心重寫（D01）；相容性研究草案，原版版本基準與具體 Rust 函式庫尚待確認。
- 範圍：官方文件、正式發行 metadata、固定 tag 的原始碼靜態分析。
- 尚未執行：下載發行檔後重算雜湊、建置、啟動 Tomcat、索引、三平台執行、功能／視覺／效能驗證。本文件不代表上述驗證已通過。

## 1. 研究結論

使用者在第一輪研究後明確選擇以 Rust 重寫核心，目的是改善效能。原先「保留 Java 索引器與 WAR，再以 npm 管理私有 Tomcat」的建議已被此決定取代；它只保留為研究歷史與建立原版 reference oracle 的方法，不是目前產品主線。

OpenGrok 既有的多專案、全文／定義／符號／路徑／歷史查詢、交叉參照、原始碼瀏覽與差異功能，現在需要被拆成 Rust 相容性契約。所有 style 仍沿用確認後的原版；CSS／JavaScript／圖示可依法沿用，JSP 的 Java 伺服端邏輯必須轉成 Rust handlers／templates。npm 是分發入口，不決定核心實作語言。

本文件的 Java、Lucene、Tomcat、JSP 與 API 描述都是「原版如何運作」的事實，用於差異測試，不能再轉述為 Rust 產品執行相依。D19已確認A：必要Universal Ctags、Git／SCM工具可隨包；精確build與能力仍待驗證；Lucene 排序／既有 index／Java plugins 等相容邊界由 D20 釐清。未確認的邊界不代表已同意降低原先完全相同要求。詳細映射見 [Rust 相容性研究](12-rust-compatibility.md)。

「Apache Web Server」仍須區分 Apache HTTP Server 與 Apache Tomcat：原版官方要求 Servlet 容器，例如 Tomcat，Apache HTTP Server 並非必備。Rust 產品將自行提供 Web Server；原版容器只用在可重現的對照環境。[S01][S03][S06]

## 2. 可重現的上游版本定位

| 項目 | 本輪確認內容 |
|---|---|
| 官方儲存庫 | `oracle/opengrok` |
| 研究用正式 tag | `1.14.20` |
| tag commit SHA | `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3` |
| 發行時間 | API `published_at = 2026-10-12T16:45:58Z` |
| 發行狀態 | `draft=false`、`prerelease=false` |
| 官方二進位檔 | `opengrok-1.14.20.tar.gz` |
| API 回報檔案大小 | 92,575,612 bytes |
| API 回報 SHA-256 | `f33d78f950ba7ed974caa9153120e4f26f9c7f4d63c1f48b7a0350e12d8e346d` |
| 雜湊驗證狀態 | 僅讀取官方 API 回報；尚未下載檔案重算 |
| 根 POM Java release | 21 |
| 根 POM Lucene 版本 | 9.12.3 |
| 根 POM Jakarta Servlet API | 6.0.0 |

本輪確認版本的用途是讓研究結果可追溯，並非替使用者選定「現用介面」的相容基準。仍須取得使用者目前 OpenGrok 版本、是否有客製化樣式／設定，以及要沿用目前版本或改以新版為基準。UI 資源已含 dark/light mode，不能假設使用者現用版本也是同一個介面。[S02][S03][S14]

發行 tag 不應單獨作供應鏈鎖定；後續 manifest 應保留 commit、各原始來源、各檔案重算 SHA-256、建置工具版本與 runtime 版本。

## 3. 官方運作需求與本輪確認的界線

官方 setup wiki 於研究時列出 Java 21–25、Universal Ctags、Tomcat 10.x、所需 SCM 工具、現代瀏覽器。Python 3.9+ 用在官方 Python repository synchronization 工具。這些是原版與 reference oracle 的依賴；Rust 產品要另外盤點同步／管理能力與外部工具，不能把原版 Python 工作流程無聲刪除，也不能把 Python 視為 Rust 的既定相依。[S01]

新增的 GitRepository.java 讀碼確認，研究版的 Git history／blame 使用 Eclipse JGit，POM 固定 JGit 7.2.1.202505142326-r。Wiki 的 Git executable 說明不能代表所有 Git 功能都是外部 git CLI。Rust 若改用 Git CLI／git2／gix，rename／merge／blame 等仍須對照原版 JGit 行為。[S18]

Universal Ctags 不能直接用較舊的 Exuberant Ctags 替代。Ctags 影響 symbols/definitions 等解析結果，因此版本也要固定，否則只固定 OpenGrok 仍無法建立穩定 parity 基準。[S01][S11]

WebappListener 原始碼遇到 Apache Tomcat 時，明確檢查版本字串必須以 `10.` 開頭。因此建立原版 reference oracle 時不能任意改用新版 Tomcat；對照環境應核對 Tomcat 10.1 的精確 patch 版本。這不是 Rust 產品的執行相依。[S06]

## 4. 原始碼模組與功能歸屬

固定 commit 下的根 POM 宣告六個模組：[S03]

| 模組 | 本輪可確認的職責 | Rust 重寫的對照用途 |
|---|---|---|
| `opengrok-indexer` | 語言分析器、Ctags 整合、Lucene 索引／搜尋、專案設定、SCM history | 建立 parser／token／symbol／search／history 對照輸出 |
| `opengrok-web` | JSP、CSS、JavaScript、Servlet filter、REST controller；產出 WAR | 固定原 UI oracle；保留資產、DOM 與路由，伺服端改寫 Rust |
| `plugins` | authorization plugin 等擴充 | 盤點功能契約；Java JAR 二進位不能宣稱 Rust 直接載入，D20 待定 |
| `suggester` | 搜尋建議相關功能 | Rust 需另行實作與驗證同等行為 |
| `tools` | Python 管理／同步工具與 client 等 | 盤點哪些功能由 Rust 取代，逐項驗證 |
| `distribution` | 組合發行檔 | 建立原版 oracle 來源與授權 inventory；Rust 另設發行流程 |

### 4.1 核心檔案定位

下列路徑均指固定 commit `3ca9d73...`。

| 目的 | 原始碼位置 | 設計影響 |
|---|---|---|
| 版本、依賴、模組 | `pom.xml` | 鎖定 Java、Lucene 等基準 |
| WAR 與 JSP 建置 | `opengrok-web/pom.xml` | 有 JSP 編譯、JSTL、Jersey、WebJars 等；不能只塞一個 HTTP server |
| UI 路由 | `opengrok-web/src/main/webapp/WEB-INF/web.xml` | 保留原始 URL 和 Servlet/filter 行為 |
| 搜尋說明 | `opengrok-web/src/main/webapp/help.jsp` | 查詢語法、快捷鍵、功能驗收來源 |
| 搜尋頁 | `opengrok-web/src/main/webapp/search.jsp` | 原始 UI 結果頁 |
| 首頁、歷史、差異 | `index.jsp`、`history.jsp`、`diff.jsp` | 首頁實際讀取；歷史／差異檔本輪確認存在，尚未逐行分析 |
| 原始樣式 | `opengrok-web/src/main/webapp/default/style-1.0.6.css` | 保留 CSS 與 dark/light 行為 |
| JS 資源 | `opengrok-web/src/main/webapp/js/` | 本輪確認樹狀清單；互動需後續 runtime 驗證 |
| 初始設定與生命週期 | `opengrok-web/src/main/java/org/opengrok/web/WebappListener.java` | 設定載入、Tomcat 版本、授權 framework 與索引檢查 |
| REST base | `.../web/api/v1/RestApp.java` | `api/v1` |
| REST 搜尋 | `.../web/api/v1/controller/SearchController.java` | API 與 UI query 名稱有差異，不能自行統一後假設同等 |
| REST 設定 | `.../web/api/v1/controller/ConfigurationController.java` | XML 設定更新與非同步任務 |
| 專案生命週期 | `.../web/api/v1/controller/ProjectsController.java` | 專案建立、索引狀態、快取與資料清理 |
| REST token gate | `.../web/api/v1/filter/IncomingFilter.java` | 受保護 API 需 bearer token；HTTPS 或明確允許 insecure token |
| 索引主程式 | `.../indexer/index/Indexer.java` | 直接 JVM 呼叫入口、參數、來源與 symlink 設定 |
| 索引資料庫 | `.../indexer/index/IndexDatabase.java` | 每專案 index、鎖、commit、xref 與中斷處理 |
| 專案資料模型 | `.../indexer/configuration/Project.java` | project path 相對單一 sourceRoot |
| 全域設定 | `.../indexer/configuration/Configuration.java` | sourceRoot/dataRoot/token/locking 等 |
| 實際環境 | `.../indexer/configuration/RuntimeEnvironment.java` | canonical path、設定讀取與 runtime 狀態 |
| Ctags | `.../indexer/analysis/Ctags.java` | 透過 ProcessBuilder 執行 Universal Ctags |
| 查詢核心 | `.../indexer/search/QueryBuilder.java`、`SearchEngine.java` | full/defs/refs/path/hist/type 等查詢語義 |
| UI query parameters | `.../indexer/web/QueryParameters.java` | query-string 相容基準 |
| SCM 工廠 | `.../indexer/history/RepositoryFactory.java` | 支援 SCM 的辨識與實作選擇 |
| 官方封裝 | `distribution/assembly.xml` | 包含 `lib/opengrok.jar`、`lib/source.war`、plugin/tools 等 |

### 4.2 完全相同需要保留的內容

沿用原始 UI 不只代表複製 CSS。Rust templates 需要重現 JSP 的頁面與資料行為；原始 JavaScript、WebJars 內資產、字型、圖示、URL、查詢參數、分頁／排序、瀏覽器儲存設定、暗色／亮色、快捷鍵、行號連結、跨參照和 history/diff 也需保持相容。JSP 原檔可作移植與授權對照，不是 Rust server 可直接執行的模板。[S04][S07][S14]

「符號」不等於 compiler/LSP 的精確語意。OpenGrok 使用自己的分析器與 Ctags、Lucene；驗收應比對原版實際結果，不應在沒有需求變更的情況下改用其他 parser 並稱結果完全等價。

## 5. 多專案與來源目錄

### 5.1 上游已有能力

IndexDatabase 類別註解與初始化邏輯表明：啟用 projects 時每專案有各自 index，配置仍共享來源根目錄與 dataRoot。SearchController 接受 project 清單，可進行多專案搜尋。[S08][S09]

多專案不能直接等同多租戶。不同使用者是否可看不同專案、是否需要登入與 ACL，是使用者決策；本輪沒有承諾隔離。

### 5.2 必須先釐清的來源模型

Project.path 明確是相對 sourceRoot 的路徑。Indexer 預設允許 sourceRoot 直接下一層的 symlink，其他連結可經明確參數／設定允許；RuntimeEnvironment 與 IndexDatabase 另有 canonical path／allowed symlink 邏輯。[S08][S10]

因此候選支援方式及風險為：

| 來源配置 | 候選處理 | 先驗證事項 |
|---|---|---|
| 多專案都在同一父目錄 | 直接設為 sourceRoot | 最少目錄映射，保留原有路徑 |
| 專案分散不同目錄 | 建受管理的來源入口映射 | symlink 的 canonical path、重複來源、刪除映射不刪原始碼 |
| Windows 多磁碟／junction | 個別 spike 後確認 | junction、權限、長路徑、UNC、重解析點、資料移動 |
| 原始碼同步／mirror | 可選模式 | 是否允許修改工作樹、切換 branch、下載與憑證 |
| 使用者工作樹直接索引 | 直接讀本地來源 | 使用者同步改檔時的快照一致性；搜尋與xref版本 |

禁止由計畫直接推論「一定要複製所有專案」或「一定要改移原始碼目錄」；這會改變容量、歷史與使用習慣，須先討論。

### 5.3 索引併發與一致性

以下描述原版 Lucene／Java 的行為，供 Rust 設計與故障測試參照。Rust 引擎要重新定義自己的 writer／snapshot／commit 契約；不能把原版設定鍵當成 Rust 已有保護，也不能直接沿用原版 index 目錄。

Configuration 預設 `luceneLocking=OFF`，Indexer 的 `--lock` 說明亦明確標示 default off。IndexDatabase 的 `pickLockFactory()` 會按設定選 Simple／Native／NoLock。[S10]

Rust 管理層必須設計互斥與持久任務狀態，避免同一專案重複 writer。Reference oracle 若做多 writer 實驗，需明確設定原版 locking；Rust 則以所選引擎實際能力驗證。IndexDatabase 有 dirty 標記與 prepareCommit/commit 等處理，但這不能直接證明「xref、history、索引、configuration 全體原子切換」。

若設計要求失敗後仍提供上一版資料，必須另外驗證完整 generation/staging 方案或採用明確的維護流程，不可只把目錄 rename 當成三平台與 mmap 的保證。

## 6. 原生 Windows：具備上游支持證據，也有明確測試缺口

官方 build workflow 的 OS matrix 包含：

- `ubuntu-latest`
- `ubuntu-24.04-arm`
- `macos-latest`
- `windows-latest`

並搭配 Java 21 和 25。Windows 步驟安裝 Chocolatey Universal Ctags，Maven build 使用 `./mvnw -B -V verify`。[S12]

但原始 POM 的 Windows profiles 明確排除以下測試：[S05][S13]

| 模組 | Windows 排除測試 | 來源註解／實務影響 |
|---|---|---|
| indexer | `**/*XrefTest.java` | 不能從官方 Windows build 推論所有 xref 已驗證 |
| indexer | `**/HistoryGuruTest.java` | history 需自行補驗 |
| indexer | `**/MercurialRepositoryTest.java` | 註解指 `hg update mybranch` timeout |
| indexer | `**/IndexerRepoTest.java` | 註解指 symlink path difference |
| web | `**/ProjectsControllerTest.java` | project 管理需補驗 |

這些排除不等於功能一定失敗；代表官方 CI 不是我們產品的完整驗收證據。第一個平台 spike 應優先在原生 Windows 64 位元執行：首次索引、definitions/refs/xref、history/blame/diff、增刪專案、非 ASCII／空白路徑、來源映射、停止重啟與更新。WSL 測試不能取代原生 Windows 驗收。

後續v0.5決策已確認Ubuntu24.04 amd64為主要部署目標，並先在macOS開發及完整驗證，再到Ubuntu原生重驗。v0.6已確認首輪M1／Apple Silicon arm64；macOS實際／最低版本與Windows具體版本／CPU仍待提供；它們決定其餘發行矩陣。此需求更新不改寫上述原版CI的靜態研究結果。

## 7. 架構決策更新與原候選退場

使用者已選擇 **Rust 核心重寫**。v0.1 的原 WAR＋私有 Tomcat、embedded Tomcat 與 Node.js 全重寫是研究歷史，不再是待使用者選擇的產品主線。

目前方向是 Rust 自帶 HTTP server、Rust handlers/templates、OpenGrok 相容的 query/analyzer/xref/history 功能，以及 npm／離線發行物。Tantivy 與 Tree-sitter 只是可行性候選，尚未選版；不能因它們支援搜尋或語法樹就推論與 OpenGrok 同等。

Java／Tomcat 原版部署僅作測試 oracle，讓 Rust 的命中、排序、頁面、路由與 history 可以逐項比較。最終產品不應以暗藏 Java 子程序包裝來冒充 Rust 核心重寫；若某項必要相容能力需要 Java bridge，必須明確提交 D20 決策。

Rust 的效能目標仍須透過原版與候選同語料、同硬體、同功能的測量證明；使用 Rust 不代表搜尋或索引必定更快。[12-rust-compatibility.md](12-rust-compatibility.md) 列出新的主要驗證問題。

## 8. 管理 API 與啟動設計細節

本節說明原版 HTTP contract，Rust handlers 要依 D20 的相容邊界實作並驗證。下列 Java 類別名稱是對照來源，不是 Rust 模組相依。

RestApp base path 是 `api/v1`。IncomingFilter 對部分讀取／搜尋路徑 allowlist，其他 API 使用 bearer token；token 要透過 HTTPS，除非配置明確允許 insecure tokens。[S15]

此行為產生幾項實作約束：

1. 內部設定更新／專案操作需配置可用的管理 channel，token 不可加入一般瀏覽器頁面或紀錄。
2. 「只綁 localhost」不會自動繞過上游 token 規則。
3. 原版 oracle 若採內部 loopback HTTP + allowInsecureTokens，必須限制管理端點的網路可達性與 token 存取；Rust 另設管理通道與核准的相容／安全契約，不能直接沿用 Java 設定當成防護。
4. SearchController REST 的 `def`、`symbol` 與 UI／QueryBuilder 的 `defs`、`refs` 命名不同；測試需針對各自既有 contract，不得假設同一套 query。
5. WebappListener 在配置讀取失敗時有繼續初始化的路徑；launcher 應在對外宣告 ready 前驗證有效配置、預期來源與專案，不可只以 port 開啟作成功判定。[S06]

若只需 npm CLI 管理，可先避免新增管理 UI，以減少風格變動。是否要 Web 管理頁仍應由使用者決定。

## 9. 授權與供應鏈工程要求

OpenGrok 的 LICENSE.txt 說明預設為 CDDL 1.0 only，另有檔案例外；根目錄有多個 LICENSE 檔及 NOTICE.txt。[S16]

依授權原文可得的工程落地要求：

- 發佈受 CDDL 覆蓋的 executable 時，也必須依授權提供對應 source，以及取得 source 的方式。
- 對 covered files 的修改需保留必要 notices，標示修改 contributor；不能將整份上游原始碼只改成 MIT／Apache-2.0。
- 可以與其他程式組成 larger work，但 covered code 仍需滿足原條款。
- 應保留原 LICENSE／NOTICE／來源定位與 patch 清單，並在發行物提供對應版本 source bundle 或確實可取用的位置。
- Rust crates、所保留的 UI 資產、Universal Ctags／SCM binaries 各有授權；若另行再散布 Java／Tomcat reference oracle，也要處理其授權。只附 OpenGrok LICENSE 不代表任何發行包已完整。
- CDDL 授權中的權利授予排除 trademark；README／套件名稱與發行說明應清楚表達是衍生封裝。UI 風格保持原樣與對外產品命名是兩個需要分開決定的事項。

以上是由原始授權條文整理的工程要求，正式發佈仍應以實際發行物的完整 component inventory 核對。單一來源 URL 不等於完整 redistribution 合規證明。

## 10. Rust 主線的優先技術驗證

全部為待執行，細節以文件 04、05、12 的 Rust 計畫為準：

| Spike | 目的 | 可交付證據 | 停止／重議條件 |
|---|---|---|---|
| SP-R01 原版 oracle | 固定上游及依賴，輸出功能／UI／token／query baseline | manifests、原始輸出、scoring解釋、頁面 | 不能以無法正常跑的 oracle 證明相容 |
| SP-R02 query/analyzer 相容 | OpenGrok query AST、欄位分詞、Unicode offsets、符號與正規化 | 與原版逐層 differential | 相同文字但命中不同時先定位，不降格成外觀相同 |
| SP-R03 引擎與排序 | 驗證 Tantivy 等候選的 postings/scorer/跨project策略 | 命中、ranking、total hits與性能曲線 | D20 未放寬前，差異仍是未達標項 |
| SP-R04 UI/xref Rust port | Rust templates配原CSS/JS，重建xref anchor與symbols | DOM、URL、鍵盤與視覺對照 | 換parser造成定義／參照差異不能自動接受 |
| SP-R05 history 與來源映射 | 對照 JGit／其他 SCM；三平台路徑與生存週期 | rename/merge/blame、junction/Unicode測項 | D19已選bundled；工具版本與完整依賴需明列 |
| SP-R06 原生平台與離線 | 以真實 Rust npm／ZIP產品在乾淨環境運行 | 原生Windows/macOS/Ubuntu、斷網冷cache證據 | WSL、開發工具或隱性下載不算通過 |
| SP-R07 初始效能 | 同語料功能一致下優先對照搜尋，並記索引／RSS／磁碟 | 可重現測量與瓶頸報告 | 沒有數據不能宣稱Rust已較快 |

## 11. 實際讀碼範圍及限制

本輪讀取固定 release 的 repository tree（2,259 個 tree entries，API 回報未截斷），以及下列關鍵檔案內容。此數量不是對所有上游程式碼做完整審核的聲明。

實際讀取內容包括：

- 根 `pom.xml`、`LICENSE.txt`、`NOTICE.txt`
- `opengrok-web/pom.xml`、`opengrok-indexer/pom.xml`
- `.github/workflows/build.yml`、`dev/main`、`dev/before`、`dev/before_install`
- `distribution/assembly.xml`
- Web `WEB-INF/web.xml`、`help.jsp`、`index.jsp`、`search.jsp`、`default/style-1.0.6.css`
- `WebappListener.java`、`RestApp.java`、`IncomingFilter.java`
- `ConfigurationController.java`、`ProjectsController.java`、`SearchController.java`
- `Project.java`、`Configuration.java`、`RuntimeEnvironment.java`
- `Indexer.java`、`IndexDatabase.java`、`Ctags.java`
- `QueryBuilder.java`、`SearchEngine.java`、`QueryParameters.java`
- `RepositoryFactory.java`

Rust 延伸研究另外讀取 CustomQueryParser、CompatibleAnalyser、PathTokenizer、HistoryAnalyzer、IndexSearcherFactory、IndexWriterConfigFactory、PlainFullTokenizer.lex、PlainSymbolTokenizer.lex、query escapers、AbstractSourceCodeAnalyzer、DefinitionsTokenStream、CxxAnalyzer、JFlexXref、GitRepository，以及原版 utils JavaScript。Tantivy 與 Lucene 的精確來源範圍見文件 12。

本輪對更多 UI、分析器、SCM adapter、tests 僅確認檔案位置；尚未對所有實作逐一分析。實作前應延伸到已核准語言、SCM、資料量及平台會走到的路徑。

## 12. 官方來源

下列原始碼連結固定於 commit，避免後續 master 更新改變本文件依據。Wiki 為研究當日讀取的動態文件。

- [S01 官方安裝需求](https://github.com/oracle/opengrok/wiki/How-to-setup-OpenGrok)
- [S02 正式 release 1.14.20](https://github.com/oracle/opengrok/releases/tag/1.14.20)；[release API](https://api.github.com/repos/oracle/opengrok/releases/tags/1.14.20)；[tag API](https://api.github.com/repos/oracle/opengrok/git/ref/tags/1.14.20)
- [S03 根 POM](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/pom.xml)
- [S04 Web 路由與設定](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/WEB-INF/web.xml)
- [S05 Indexer POM 與 Windows profiles](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/pom.xml)
- [S06 WebappListener](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/WebappListener.java)
- [S07 help.jsp](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/help.jsp)
- [S08 Project.java](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/configuration/Project.java)
- [S09 SearchController](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SearchController.java)
- [S10 IndexDatabase](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/index/IndexDatabase.java)；[Configuration](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/configuration/Configuration.java)；[Indexer](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/index/Indexer.java)
- [S11 Ctags.java](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/Ctags.java)
- [S12 官方 Build workflow](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/.github/workflows/build.yml)
- [S13 Web POM 與 Windows profiles](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/pom.xml)
- [S14 原始樣式](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/default/style-1.0.6.css)
- [S15 IncomingFilter](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/filter/IncomingFilter.java)
- [S16 LICENSE.txt](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt)；[NOTICE.txt](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/NOTICE.txt)
- [S17 distribution assembly](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/distribution/assembly.xml)
- [S18 GitRepository：JGit history/blame](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java)；JGit 精確依賴版本見 S05。

