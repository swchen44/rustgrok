# Rust 重寫的 OpenGrok 相容性研究

- 版本：v0.6，2026-10-25（Asia/Taipei）
- 已確認：D01 選擇 Rust 重寫核心，保留自帶 Web Server、npm、內網、多專案、三作業系統與原版 style 目標。
- 已確認：使用者回覆「A+搜尋速度」；D19 採 Rust 核心隨包附帶必要 Ctags／Git／其他 SCM helpers，D21 以搜尋速度為效能首要目標。
- 最新確認：D02 依「3, 約5人， 不用權限管理」支援本機與內網共用、約 5 位同時使用者，不做使用者角色／專案 ACL 管理，不強迫新帳號、SSO 或登入。原版 authorization／RBAC plugins 不再是 mandatory 產品功能或完整 parity 阻擋；其他 plugins 仍須盤點。
- 正式部署優先：Ubuntu 24.04 amd64 已確認；Rust 架構命名對應 x86_64。實際 24.04.x point release 隨測試環境記錄，不作本輪需求確認阻擋。
- 開發／完整驗證順序：使用者已確認首輪開發機為 Apple M1／arm64，Rust target mapping 為 aarch64-apple-darwin；先 macOS 完整驗證，再做 Ubuntu 24.04 amd64 平台適配及完整 native 重驗。Mac PASS 不能當作 Linux PASS。macOS 實際／最低版本及 Windows 詳細平台仍依 D03 待定，Windows 支援保留。
- GitHub：使用者已確認 public；公開 repo [swchen44/rustgrok](https://github.com/swchen44/rustgrok) 已建立並經 API 核實，預設分支 main。規劃內容仍待初始提交，GitHub Release 尚未建立；repo 建立不代表已有產品或測試結果。
- 仍待細化：helper 精確版本／build／各 SCM 能力依 G1 與原 inventory 核對；搜尋數值 SLO、代表工作量／環境及資源取捨仍 OPEN。D16 管理操作開放位置、D20 其餘相容邊界與 D04 原版介面版本待決。除 D02 明確排除的權限管理範圍，沒有批准其他功能／style 差異或 Java bridge。
- 候選：Tantivy、Tree-sitter 與其他 Rust 函式庫均未被選為最終依賴。
- 驗證狀態：僅官方文件與固定版本 source review；**未編譯 Rust 原型、未執行原版／Rust differential、未實測效能。**
- 決策原則：D02 明確不要求的角色／ACL 與 authorization／RBAC plugin parity 從 mandatory 範圍移除；其餘功能與 style 要求保持。改用 Rust 不自動批准差異，非授權範圍的差異仍列為未達標或待釐清。

## 1. 架構改變了什麼

原版 Java／Tomcat 部署改作 reference oracle。產品核心應由 Rust 負責 HTTP、專案與任務管理、索引、搜尋、xref 與 history；npm 負責分發與必要的啟動入口。保留原版畫面與操作，不代表繼續在產品執行 JSP。

建議把相容工作拆成三種：

| 種類 | 處理方向 | 不可混淆之處 |
|---|---|---|
| 原始靜態資產 | 依法保留 CSS、JS、字型、圖示及第三方資產 | 必須同時保留它們依賴的 DOM、URL、資料及事件契約 |
| Java／JSP／JFlex 功能 | 移植到 Rust handlers、templates、analyzers、xref renderer 與 SCM adapter | 翻譯程式語言不會自動移除原始授權義務 |
| Java／Lucene 生態系格式與插件 | 依 D20 盤點設定匯入、索引重建與非 authorization JAR 功能移植；D02 已免除權限 plugin 的 mandatory parity | 不可把 Java plugin 或 Lucene index 直接稱為 Rust 可用；bridge 尚未批准 |

Rust 可減少 JVM 啟動與執行環境負擔，但使用 Rust 本身不能證明查詢、索引或峰值記憶體已改善。瓶頸仍可能在分詞、磁碟、解壓縮、SCM、Ctags 或工作排程。官方 Tantivy benchmark 也提醒結果受 queries 與負載影響，不能直接把其通用資料集數字當成此產品效能。[T01]

## 2. 本輪固定來源

| 元件 | 本輪研究版本／定位 | 用途 |
|---|---|---|
| OpenGrok | 1.14.20；commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3` | 研究用功能 oracle，正式使用者基準未批准 |
| Lucene | 9.12.3；commit `f965e930673c1f5cb478dc6a8907f5fc4ef7b539` | 對照原版 parser／scorer 語意 |
| Tantivy | tag 0.27.0；commit `67889269cae67063c372bc236b102d632a021568` | 候選引擎靜態研究；不是已選版 |
| Tantivy tag object | `a22c1b184e861d1065b964bb45f013ef8945dae4`；tagger 日期 2026-10-17 | 明確區分 annotated tag 與實際 commit |
| JGit | OpenGrok POM：7.2.1.202505142326-r | 原版 Git history/blame 對照 |
| Universal Ctags | 尚未鎖定 build | 須與原版 oracle 使用相同版本／選項後比較 |

研究時 GitHub `releases/latest` 回傳 Tantivy 0.26.1，而 docs.rs/latest 與官方 tag 可確認 0.27.0。此文件以實際讀取的 0.27.0 commit 為準，**不宣稱 latest metadata 是一致或最終選型**。[T00]

Tantivy 0.27.0 Cargo.toml 指定 minimum Rust 1.88，並具有自己的 index-format version 常數。這是該研究版本的事實；最終 Rust toolchain、crate features、最低 CPU／OS 與 Cargo.lock 要在可行性 gate 後另鎖。[T02][T08]

## 3. 最大相容風險：搜尋的「同功能」包含多層語意

### 3.1 原版不是 generic Lucene 預設

已讀的 CustomQueryParser 與 CompatibleAnalyser 顯示：

- OpenGrok 預設以 AND 組合沒有明寫運算子的 terms。
- `defs`／`refs` 大小寫敏感；其他欄位採各自的 normalization。
- `full`、`defs/refs`、`path/project`、`hist` 使用不同 tokenizer／analyzer。
- QueryBuilder 先按欄位經 Full／Path／DefaultQueryEscaper，再各自 parse，最後依 MUST／SHOULD／MUST_NOT 規則組合多欄位 query。
- `path` 的 slash 及 regexp 邊界有額外處理；Windows 路徑不能直接把反斜線塞進 query。
- 歷史欄位有特定 stop-word 清單及 position increments。
- 原版語言分析器與純文字 query tokenizer 不是同一份實作。[O01][O02][O03][O04]

因此推薦設計是「相容層先表達原版語意，底層引擎接收明確的型別化 query」。不能把使用者的 OpenGrok 字串直接傳入 Tantivy QueryParser 並稱作相容。

### 3.2 差異清單與實作要求

| 範圍 | 已確認的原版／候選差異或風險 | Rust 必須做的事 |
|---|---|---|
| 預設運算子 | OpenGrok 設 AND；Tantivy QueryParser 預設 OR，可改 conjunction | 相容 parser 明確保存原版 AND，並測混合 AND/OR/NOT 與 parentheses |
| 欄位合併 | OpenGrok QueryBuilder 有多欄 Boolean clause 合併規則 | 保存欄位 provenance 與原始 clause tree，不能只把各欄字串用 AND 拼接 |
| 大小寫 | defs/refs case-sensitive，其他欄位各有 normalization | 不使用整份 query 先 lowercase 的捷徑 |
| 長識別字 | Tantivy default tokenizer 使用 RemoveLongFilter::limit(40) | 程式碼 tokenizer 必須另建；驗證長函式／變數名不被丟棄 |
| 標點與底線 | OpenGrok PlainFullTokenizer 定義 identifier、number 與獨立標點 tokens | 以 source/token oracle 移植；不能用一般文章 tokenizer 取代 |
| path | OpenGrok PathTokenizer 保留獨立 dot token、切分目錄，使用其 lowercasing | 路徑 query／索引 token 保持成對相容，驗證副檔名與帶點名稱 |
| phrase／proximity | 都有 positions/slop，但 tokenizer 與 position gaps 會改變命中 | 比對位置、停用詞、重複詞、transpose、多 token identifiers 與 phrase score |
| fuzzy | Lucene 有逐 term 的 `term~N` 語法；Tantivy提供 FuzzyTermQuery 與 field-level fuzzy 設定 | 自己解析原版語法，明確 mapping edit distance、transpose、prefix、expansion與score；不能用整欄 fuzzy 代替單一term fuzzy |
| wildcard | 原版支援 `?`、`*` 及可配置 leading wildcard；Tantivy RegexQuery 文件提到 wildcard-to-regex | 相容層辨識與 escape wildcard，依引擎 API 建 query；以命中及計分測試證明轉換 |
| regexp | 原版以 Lucene automaton RegExp 為準；Tantivy QueryParser regex 預設停用，啟用後仍要求指定 text field | 不直接把 Lucene regexp 當 Rust regex；列出語法、Unicode、限制與錯誤對照 |
| 錯誤／pure-negative | parser 的錯誤類別、lenient repair、負向 query 政策可能不同 | UI 與 API 的錯誤處理要比原版，不能悄悄使用 lenient parser |
| 高亮／snippet | 搜尋命中不代表相同 matching lines／片段／HTML | 另做原版 context formatter、摘要選擇、行號及 escaping 對照 |
| total hits／分頁 | 原版追蹤命中數並採 score 或 field sort | 全量 hit count、頁界、tie 行為與排序單獨驗證 |
| 跨 project | 原版使用 MultiReader 對選定 projects 一起搜尋 | 不把各 project 獨立計分的 top K 合併視為相同 |

來源：[O01–O05][T03–T07][L01]。上述是實作風險與測試要求；此輪沒有執行候選引擎確認全部差異的實際輸出。

### 3.3 建議的模組分界

1. **HTTP input adapter**：保留原 UI／API 的欄位、URL 編碼與參數名稱。API 的 `def`／`symbol` 和 UI 的 `defs`／`refs` 本來就不同。
2. **OpenGrok compatibility parser**：處理欄位 escapers、operators、field grouping、boost、range、wildcard、regex、fuzzy、phrase，產生包含 source span 與 field 的 AST。
3. **Analyzer registry**：逐欄位／逐語言產生 tokens、positions、offsets 與 canonical form。
4. **Query planner／engine adapter**：將已明確的 AST 建成 Tantivy 或其他引擎的 query API；保留與原版解釋輸出的對照。
5. **Result adapter**：處理 count、sort、paging、line matches、snippets 與 HTML／JSON contract。

AST 與 token 的 debug 輸出先穩定，才能判別差異來自 parser、分詞、postings、scorer 或 renderer。這比只看最後搜尋頁有沒有資料更能降低返工。

## 4. Ranking 不是選到 BM25 就結案

兩套引擎都採 BM25 家族，不代表分數或排序自動相同。本輪直接讀取的程式碼存在具體差異：

| 層次 | Lucene 9.12.3 | Tantivy 0.27.0 研究來源 | 影響 |
|---|---|---|---|
| k1／b 預設 | 1.2／0.75 | 1.2／0.75 | 參數看似一致不足以證明同分 |
| 欄位文件數 | 使用 CollectionStatistics.docCount，即有該欄位的文件統計 | 預設 StatisticsProvider 的 total_num_docs 加總 segment.max_doc | 稀疏 defs／hist 等欄位須測 IDF 與平均長度 |
| term weight | boost × IDF | IDF × (1 + k1)，再套 boost | 同條件下分數可能差固定倍率；混合 constant-score／不同query還需對照 |
| 浮點計算 | 使用 `weight - weight/(1 + freq × normInverse)` | 使用 `weight × freq/(freq + norm)` | 浮點捨入與 tie 要實測 |
| 文件長度 | Lucene norm／overlap token 規則 | Tantivy fieldnorm／token pipeline | 重疊位置、長文件、stop words需覆蓋 |
| 跨 project | OpenGrok MultiReader 建立合併讀取視野 | 要由產品決定如何統一統計 | 獨立index各算IDF再合併可能變排序 |

來源：[O05][L02][T06]。

固定倍數差異本身不一定改變單純同類 query 的相對名次；此文件不把它誇大成所有排序必定不同。但它足以證明不能宣稱「score 完全相同」，且欄位統計、multi-term rewrite、phrase、fuzzy／regex 的 scoring 與 cross-project statistics 都必須驗證。

若 D20 要求原版 relevance order／分數，需研究：

- 自訂 statistics provider／scorer 或其他引擎的可行性。
- 全 project／選定 project 的統計範圍；project 選擇與跨 project 搜尋仍保留。本版依 D02 不做使用者角色／project ACL，因此不新增按使用者 ACL 過濾的 mandatory 相容要求。
- score ties 的觀察契約與原版 doc-order 影響。
- 常數計分 query 與 BM25 clauses 混合時的排序。
- 分頁不能漏掉或重複結果，不能只把兩邊結果任意排序再比較。

若使用者最後接受某些內部 scoring 數值不同，也必須明確記錄核准範圍；目前尚未核准任何放寬。

## 5. Analyzer 與 xref：需要保存的資料比語法樹更多

### 5.1 最低資料契約候選

| 資料 | 至少保存內容 | 相容目的 |
|---|---|---|
| Source document | project/path、來源revision、原始bytes／encoding、內容hash、行起點 | raw、xref、搜尋與history可對齊版本 |
| Search token | field、canonical text、原始span、position、position length、origin analyzer | phrase、term、snippet、normalize差異可定位 |
| Symbol／definition | name、kind、scope、signature、line/span、語言、來源工具 | symbols欄、definitions、navigation與xref |
| Reference occurrence | token／symbol occurrence、所在source span、查詢／連結策略 | 不把名稱相同就誤當compiler resolved reference |
| Xref markup model | line anchor、CSS classes、identifier spans、href、folding scopes | 原CSS/JS可用、行號與快捷鍵一致 |

原版 `defs`／`refs` 是 OpenGrok analyzers／Ctags 協作的結果；不是把語法樹全部 identifier 節點存起來就完成。CxxAnalyzer、JFlexXref 與 DefinitionsTokenStream 的分工需移植或用等價輸出證明替代實作。[O06]

### 5.2 Unicode 與 offsets

Tantivy Token API 明確定義 offsets 是 UTF-8 text 的 byte indexes。Java／JFlex 路徑有自己的 character offsets。Rust 不能直接把參考輸出的 char offset 當成 byte offset；需建立可驗證的對照，覆蓋繁中、emoji、組合字、Unicode lowercase expansion、CRLF、tab 與無效編碼。[T09][O02][O04]

Tantivy index-record option 可保存 postings positions；這不表示已提供 OpenGrok 一樣的 term vectors、原始碼行號、xref HTML 或 snippet 策略。必要的精確 source-span／line map 可由產品另外保存，並量測其磁碟與更新成本。[T07]

### 5.3 Tree-sitter 的角色

Tree-sitter 官方定位是 parser generator 與 incremental parsing library，能建立 concrete syntax tree；runtime 以 C11 實作，提供 Rust binding。[R01]

它可以作為語言分析候選，但不能由「有 CST」推出下列結論：

- 與 OpenGrok 的 defs/refs 集合完全一致。
- 自動提供跨檔案符號解析、型別與呼叫語意。
- 全部原版語言都有相同成熟度與錯誤恢復。
- 替換語言 parser 後仍可直接沿用原版 xref DOM。
- Rust 專案使用它就變成完全沒有 C／native dependencies。

若評估 Tree-sitter，按語言建立 mapping 與 differential corpus；純文字、未完成程式、preprocessor、C/C++ macros、組語、generated source、embedded languages 均要涵蓋。它是否被採用仍是工程 spike 的結果，沒有替使用者決定。

## 6. Ctags：可降低定義移植範圍，但不是免費取得全部相容

原版 Ctags.java 明確以 ProcessBuilder 啟動 Universal Ctags，並設定 filter 模式、fields、extras、language-specific 選項。[O07] D19 已確認允許將必要 Ctags 隨包附帶並由 Rust 啟動。具體 build 尚未選定；候選以原版 oracle 的相同版本建立 adapter，先比對傳入內容、參數與輸出 interpretation。

在已確認的 bundled helpers 策略下，優先驗證沿用 Ctags 的 adapter。下表第二項保留為個別語言或相容障礙的技術備案，不重新開啟是否允許外部工具的需求問題：

| 候選 | 優點 | 必須驗證 |
|---|---|---|
| 保留相同 Universal Ctags build 與等價參數 | 接近原版定義擷取來源，減少一次改動面 | process lifecycle、filter framing、timeouts、版本、各OS parser能力、escaping、metadata |
| Rust／其他 parser 自行重做 | 有機會減少外部程序 | 每語言 definitions、scope、signature、file scope、parser fallback與xref差異全部自負 |

若改用 Ctags JSON Lines 輸出，官方文件指出需要 build 含 libjansson，且欄位／types、scope representation 與傳統 tags 格式不完全一樣。不能假設所有三平台 binaries 都有 JSON 支援，也不能把改輸出格式視為只有 parser code 的小改。[C01]

全文 tokenization、refs、xref render、history 整合、搜尋與管理仍由 Rust 核心負責。D19 的隨包策略同時涵蓋必要 Ctags／Git／其他 SCM helpers；各工具是否納入、精確版本、parser／SCM 能力與相依檔案由 G1 依原功能 inventory 核對，不把策略確認當成任何 binary 或相容測試已完成。

## 7. History：原版 Git 路徑實際使用 JGit

OpenGrok 1.14.20 的 GitRepository 使用 Eclipse JGit 讀取 revisions、history、diff/rename、blame。已讀碼可見 blame 使用 `RawTextComparator.WS_IGNORE_ALL`，並依原版設定決定 follow renames。[O08]

因此即使選 Git CLI，也不能只用預設 `git log`／`git blame` 的結果判斷已同等。候選 adapter 要明確處理：

- commit 選取、merge traversal、parent 順序與歷史排序。
- rename／copy detection、follow 行為、刪除後查舊檔、path filtering。
- blame 的 whitespace 政策、作者顯示與短hash長度。
- raw revision contents、編碼、CRLF與binary檔。
- worktree、submodule、shallow clone、空repo、dirty工作樹與不可讀revision。
- timeout、取消、讀取失敗與快取失效。

Git CLI、git2/libgit2、gix 都是候選，尚未選定。Git 官方命令文件可作 CLI adapter 的行為參考，但最終 parity oracle 是選定版本的 OpenGrok/JGit observable behavior。[G01]

其他 SCM 必須照上游 inventory 列出並與使用者核對；不能因先做 Git prototype 就默認最終只有 Git。

## 8. JSP 改成 Rust templates，style 保持原版

JSP 內含 Java server-side 邏輯，Rust HTTP server 不會直接執行它。移植方案應分開處理：

1. **模板與資料模型**：把 JSP／tag files／PageConfig 的動態資料改為 Rust view models與templates；維持可見內容、表單、元素階層與頁面組合。
2. **資產**：把原CSS／JS／font／icons與WebJars提供的資產轉成Rust可提供的靜態資產，固定來源、hash與必要notices。
3. **瀏覽器契約**：保留 DOM ids/classes、`window.contextPath` 等 JS globals、行號anchors、cookie/local-storage keys與初始化順序。
4. **HTTP契約**：原URL、query names、redirect、status、content type、raw/download、search/history/diff/suggest endpoints要逐項保留。
5. **互動**：symbols浮窗、highlight、folding、鍵盤快捷鍵、annotation切換、搜尋建議、project選擇及dark/light都要用真瀏覽器比對。
6. **輸出escaping**：程式碼文字與帶連結markup不能混用unescaped templates；正確性測試要包括包含HTML樣式字元的source。

已讀原版 utils JavaScript 確認其依賴 DOM、contextPath與 `/api/v1/suggest`、`/api/v1/suggest/config` 等路由。只保持 CSS 檔案不變，畫面與功能仍可能失效。[O09]

建議先挑首頁、搜尋結果、xref三種頁面做Rust垂直prototype，固定同OS／瀏覽器／字型／viewport與原版比較。prototype 只是提早驗證方法，不代表其餘頁面被移出正式範圍。

## 9. 設定、API、索引與 Java plugins 的不同相容層

| 相容層 | Rust 重寫面臨的事實 | D20／實作需要的明確答案 |
|---|---|---|
| 搜尋頁／URL／REST | 可以重建observable contract | D02 排除的權限管理以外，既有契約仍須盤點；管理操作在哪開放依 D16，不能直接把寫入端點開到 LAN |
| JavaBeans XML configuration | 原版透過XMLDecoder/XMLEncoder表達Java objects/properties | 支援哪些import/export與未知欄位；不能以一般XML parse宣稱可執行任意原配置 |
| Lucene index | Tantivy有自己的metadata、segments與index version；尚無證據可直接讀原Lucene索引 | 是否允許從source重建；若需遷移，另做有驗收的converter |
| History／xref cache | 可能含Java序列化或原版專用結構 | 是否需要直接匯入；不可僅改副檔名就沿用 |
| Java JAR plugins | Rust程序不能直接提供Java類別載入與原Servlet介面 | authorization／RBAC plugins 依 D02 不再 mandatory；非 auth plugins 仍逐項盤點／移植，不能默刪；bridge 尚未批准 |
| Token／auth／ACL | 原版filters與plugin stack是Java實作 | D02 不做角色／專案 ACL，不強迫新帳號／SSO／登入；管理控制通道與操作可達性仍由 D16 決定 |
| CLI／Python tools | 可能有既有自動化使用 | 命令／格式相容或migration mapping，使用者沒有確認前不自行縮範圍 |

來源：[O10][O11][T08] 描述原版技術事實。產品範圍依使用者 D02 決定，角色／ACL 與相關 authorization／RBAC plugins 不再 mandatory；**其他相容維度沒有因此獲准縮減**。舊 API／config／plugins 仍可盤點，非 auth plugin 不因這次決定被默默刪除。

## 10. 授權：依內容來源管理，不能只看實作語言

OpenGrok 預設 CDDL 1.0 only，檔案可能另有例外。直接複製或改寫的 CSS/JS/JSP片段、lexer規則、Java演算法與source tests，應保存來源、commit、授權、修改紀錄及對應source。把Java移植成Rust不能直接聲稱「現在全部MIT」。[O12]

建議建立逐檔 provenance：

| 欄位 | 內容 |
|---|---|
| origin | 原版repo／commit／檔案或自主新增 |
| operation | 原樣沿用、port／translation、局部修改、獨立新增 |
| applicable_license | 依來源檔與元件條款判定 |
| notices | 原copyright、作者、修改者、LICENSE／NOTICE |
| distribution | 哪些npm／ZIP／source artifacts包含 |
| verification | source availability與實際發行檔的對應 |

Tantivy 使用MIT，但它的授權不能覆蓋被移植的OpenGrok內容。Ctags、Git、Tree-sitter runtime／各語言grammar、Rust crates、字型與其他assets另做inventory。獨立新增程式碼與衍生檔的授權選擇應分清楚；正式發佈前依實際來源及發行內容核對。

## 11. 已確認決策與仍待釐清的邊界

### D02：本機與內網共用、約 5 人、無權限管理 — CONFIRMED

使用者原文「3, 約5人， 不用權限管理」確認本機與 LAN 都要支援，約 5 位同時使用者，不做使用者角色或 project ACL 管理，不強迫新增帳號、SSO 或登入。authorization／RBAC plugins 的完整相容不再是產品 mandatory 功能或全 parity 阻擋；非 auth plugins 保留 D20 盤點。

約 5 人是使用規模估計，不是 5 QPS、固定同時查詢數或產品硬上限。工程提案以 LAN 共用讀者路徑、主機 CLI／本機 control 處理管理；這個管理配置尚未批准，實際管理操作開放位置依 D16 決定。無權限管理也沒有授權把所有管理寫入端點開到 LAN。

### D03：Ubuntu 24.04 amd64 正式部署優先；Apple M1 macOS 先開發與完整驗證

使用者已確認 Ubuntu 24.04 與 amd64（Rust 架構名稱 x86_64）作為正式優先部署平台。先前 24.02 的說法已由後續 24.04 確認釐清；實測 point release、kernel、硬體與環境資訊記入測試 manifest，不把 point release 當成本輪阻擋。

最新工作順序已確認為：先在 macOS 開發並完成第一輪完整驗證，再進行 Ubuntu 24.04 amd64 的平台實作適配與完整 native 重驗。macOS 的通過結果不能替代 Ubuntu 原生證據；正式部署優先平台不等於開發先後順序。

使用者已確認第一輪 Mac 開發機為 Apple M1，CPU 架構 arm64；Rust target mapping 為 aarch64-apple-darwin。這項確認沒有指定 macOS 實際版本或最低支援版本，兩者仍為 OPEN；Windows 版本與 CPU 詳細規格也仍 OPEN。

同一 Rust 核心搭配各平台 build／adapter 是工程設計提案，平台適配不另建一套獨立搜尋產品。Windows 64 位元支援保留；M1／arm64 與 Ubuntu amd64 不再是待決的 CPU 選項，但完整 build 設定與平台驗證尚未完成。

### D11：公開 rustgrok repo 已建立；內容與 Release 待交付

使用者已確認 repository visibility 為 public。公開 repo [swchen44/rustgrok](https://github.com/swchen44/rustgrok) 已成功建立，API 核實 repository ID 為 1433282584、owner 為 swchen44、default branch 為 main；建立表單狀態為 SUBMITTED_SUCCESSFULLY。Owner 的來源仍是已登入帳號／建立表單及 API 觀察，不另改稱使用者逐字指定。

本紀錄的規劃內容上傳狀態為 PENDING_INITIAL_PLANNING_COMMIT；GitHub Release 為 NOT_CREATED，尚未發布 release packages。使用者要求的 GitHub Releases、npm 與離線交付保持；repo 建立成功不代表產品實作、測試或發行已完成。

### D19：必要工具隨包附帶 — CONFIRMED

使用者以「A+搜尋速度」選擇 Rust 核心可隨包附帶必要 Ctags／Git／其他 SCM helpers，由 Rust 管理其執行。產品的必要工具依賴應納入發行物與離線驗證，不要求使用者另行安裝這些工具，也不在安裝或首次執行時隱性連外補檔。

已確認的是 bundled helpers 策略。工具清單、精確版本／build／features、native 相依閉包與各 SCM 的具體能力仍依原功能 inventory 與 G1 驗證確定；不據此刪除尚未完成的語言或 SCM。這項決定不批准 Java backend 或 plugin bridge，D20 維持原狀。

### D20：完全相同的驗收範圍

除了 D02 明確不要求的角色／ACL 與相關 authorization／RBAC plugin parity，其餘原版功能與 style 要求繼續有效，需把容易混在一起的維度確認清楚：

- 查詢語法與命中內容、結果順序、數值score／tie容忍。
- UI／快捷鍵／URL／REST與既有自動化。
- 現有config、index、xref/history cache、非 auth Java plugins 是否需直接沿用；authorization 設定可納入舊資料盤點，但不據此重新要求產品具備角色／ACL 功能。
- 原版與新引擎內部機制不同時，哪些差異可接受，若有，必須由使用者明確批准。

沒有批准前，不得把「同功能」改寫成只有幾種常見query與類似UI。

### D21：搜尋速度優先 — CONFIRMED；數值與取捨仍 OPEN

使用者已明確選擇搜尋速度優先，不再重問效能首要目標。G1 優先量測一般全文 query、path、defs 與跨 project 搜尋的端到端延遲，保留 cold／warm、並行及背景索引中的結果；此量測順序是候選工程安排，具體 workload 比例與數值門檻尚未批准。

已知使用規模約 5 位同時使用者；數值 SLO、代表資料量／query mix／請求到達模式、硬體／磁碟／記憶體環境，以及可接受的 RSS、磁碟、索引時間與資料新鮮度取捨仍 OPEN。不能直接把約 5 人換算為 5 QPS 或硬性並行上限。先建立相容的結果與公平 baseline，再決定門檻；搜尋優先不會放寬 D20 的命中、排序、UI 或功能要求。本輪仍沒有 Rust 加速實測證據。

## 12. 建議的可行性證據

正式大規模移植前，至少產出：

1. **Parser/token parity report**：同欄位、同語料、長識別字、標點、Unicode、path與history stopwords；保留原AST/token與候選差異。
2. **Search/scorer report**：命中、count、relevance/path/date sort、phrase/fuzzy/regex、稀疏欄位、跨project統計；ranking差異不被任意重排隱藏。
3. **Xref/UI report**：原CSS/JS搭配Rust templates；DOM、行號、links、快捷鍵與同環境screenshots。
4. **SCM report**：JGit原版與Rust adapter的rename/merge/blame/diff對照，依已確認的D19 bundled helpers策略驗證精確工具與平台能力。
5. **Perf report**：以搜尋速度優先，比較同資料／功能／硬體下的全文query、path、defs及跨project搜尋端到端cold/warm延遲與分布，同時記錄index/update、總RSS、磁碟峰值與正確性；不能只報最漂亮平均。數值SLO及可接受取捨仍待決。
6. **Platform/offline report**：先在已確認的 Apple M1／arm64 macOS 完成第一輪完整驗證，實際／最低 macOS 版本仍需記錄與確認；再於正式優先部署平台 Ubuntu 24.04 amd64 完成平台適配及完整 native 重驗。兩平台分別保存證據，不能以 Mac PASS 代替 Linux PASS。point release 記錄於實測環境，Windows 支援及其原生驗證仍保留。所有測試使用真正Rust成品，沒有Java/Tomcat隱性執行，也沒有未批准的外部工具。

本輪以上項目全部為 NOT_RUN。選庫與實作工期應以這些spikes的結果更新，不能沿用Java包裝方案的工作量。

## 13. Primary sources

### OpenGrok：commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3

- [O01 CustomQueryParser](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/CustomQueryParser.java)
- [O02 CompatibleAnalyser](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/CompatibleAnalyser.java)；[PlainFullTokenizer.lex](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/jflex/analysis/plain/PlainFullTokenizer.lex)；[PlainSymbolTokenizer.lex](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/jflex/analysis/plain/PlainSymbolTokenizer.lex)
- [O03 QueryBuilder](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/QueryBuilder.java)；[FullQueryEscaper.lex](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/jflex/search/FullQueryEscaper.lex)；[PathQueryEscaper.lex](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/jflex/search/PathQueryEscaper.lex)
- [O04 PathTokenizer](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/PathTokenizer.java)；[HistoryAnalyzer](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/HistoryAnalyzer.java)
- [O05 SearchEngine](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/SearchEngine.java)；[IndexSearcherFactory](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/configuration/IndexSearcherFactory.java)
- [O06 CxxAnalyzer](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/c/CxxAnalyzer.java)；[JFlexXref](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/JFlexXref.java)；[DefinitionsTokenStream](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/plain/DefinitionsTokenStream.java)
- [O07 Ctags](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/Ctags.java)
- [O08 GitRepository](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java)；[JGit dependency POM](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/pom.xml)
- [O09 原版 utils JavaScript](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/js/utils-0.0.48.js)；[search.jsp](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/search.jsp)
- [O10 Configuration](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/configuration/Configuration.java)
- [O11 plugins tree](https://github.com/oracle/opengrok/tree/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/plugins)
- [O12 LICENSE.txt](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt)

### Tantivy：研究 commit 67889269cae67063c372bc236b102d632a021568

- [T00 官方 tag](https://api.github.com/repos/quickwit-oss/tantivy/git/ref/tags/0.27.0)；[annotated tag object](https://api.github.com/repos/quickwit-oss/tantivy/git/tags/a22c1b184e861d1065b964bb45f013ef8945dae4)
- [T01 README與benchmark說明](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/README.md)
- [T02 Cargo.toml](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/Cargo.toml)
- [T03 QueryParser source與文件註解](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/query_parser/query_parser.rs)；[query grammar](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/query-grammar/src/query_grammar.rs)
- [T04 default tokenizers](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/tokenizer/tokenizer_manager.rs)
- [T05 RegexQuery](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/regex_query.rs)；[FuzzyTermQuery](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/fuzzy_query.rs)；[PhraseQuery](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/phrase_query/phrase_query.rs)
- [T06 BM25](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/bm25.rs)
- [T07 IndexRecordOption](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/schema/index_record_option.rs)
- [T08 index format constants](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/lib.rs)；[index metadata](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/index/index_meta.rs)
- [T09 Token API](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/tokenizer-api/src/lib.rs)

### 其他 primary sources

- [L01 Lucene 9.12.3 classic query syntax](https://lucene.apache.org/core/9_12_3/queryparser/org/apache/lucene/queryparser/classic/package-summary.html)
- [L02 Lucene 9.12.3 BM25Similarity source](https://github.com/apache/lucene/blob/f965e930673c1f5cb478dc6a8907f5fc4ef7b539/lucene/core/src/java/org/apache/lucene/search/similarities/BM25Similarity.java)；[官方API文件](https://lucene.apache.org/core/9_12_3/core/org/apache/lucene/search/similarities/BM25Similarity.html)
- [C01 Universal Ctags JSON輸出](https://docs.ctags.io/en/latest/man/ctags-json-output.5.html)
- [G01 Git log](https://git-scm.com/docs/git-log)、[blame](https://git-scm.com/docs/git-blame)、[diff](https://git-scm.com/docs/git-diff)
- [R01 Tree-sitter官方介紹](https://tree-sitter.github.io/tree-sitter/)

