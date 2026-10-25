# ADR-0002：Rust 核心重寫與 OpenGrok 相容性

版本：v0.6；日期：2026-10-25。取代 [ADR-0001](0001-runtime-and-core-reuse.proposed.md)。

| 決策層次 | 狀態 |
|---|---|
| Rust 核心重寫 | **ACCEPTED BY USER**（D01） |
| 本機與內網共用、約 5 人、無角色／專案 ACL 管理 | **CONFIRMED**（D02）；管理操作開放位置仍依 D16 |
| Ubuntu 24.04 amd64 正式部署優先 | **CONFIRMED**；Rust 架構名稱 x86_64；point release 隨實測環境記錄 |
| 先 Apple M1／arm64 macOS 開發／完整驗證，再 Ubuntu 適配／完整 native 重驗 | M1 CPU **USER_CONFIRMED**；target mapping aarch64-apple-darwin；macOS 實際／最低版本、Windows 詳細規格仍 OPEN |
| 公開 GitHub repo swchen44/rustgrok；release packages 放 GitHub Releases | public **USER_CONFIRMED**；初始規劃已提交 main 並逐檔核實 46 檔，Release 尚未建立 |
| 必要 Ctags／Git／其他 SCM helpers 隨包 | **CONFIRMED**（D19）；工具版本／build／具體能力待 G1 |
| 搜尋速度優先 | **CONFIRMED**（D21）；數值 SLO／環境／資源取捨仍 OPEN |
| crates、engine、templates、storage 與 modules | **PROPOSED**，待 G1 spikes |
| OpenGrok 功能與 style | D02 免除角色／ACL 及 authorization／RBAC plugins 的 mandatory parity；其餘原要求維持 |
| runtime／parity／效能驗證 | **NOT_RUN** |

## 1. Context

使用者原先需要原版 OpenGrok 功能與 style、自帶 Web Server、多專案、內網、npm、macOS、Ubuntu 24.04、Windows 64 位元，以及可選離線 ZIP。後續確認本機與 LAN 共用、約 5 人、無權限管理，正式部署優先 Ubuntu 24.04 amd64；先前的 24.02 已釐清。開發與第一輪完整驗證先在 macOS 進行，再做 Ubuntu 平台適配及完整原生重驗。v0.1 曾探討 Java／Lucene／WAR 與私有 Tomcat，現已由 Rust 決定取代。

使用者先決定：「我還是想要改寫rust 因為效能比較好」，確立 Rust 核心方向；其後回覆「A+搜尋速度」，確認必要 Ctags／Git／SCM helpers 隨包，以及搜尋速度優先。這些選擇不當成本案已量測加速的證據，不再重新詢問已確認的方向。

使用者確認「3, 約5人， 不用權限管理」，之後確認 Ubuntu「24.04」及「amd64」，另指定 macOS 先開發／完整驗證、Ubuntu 後適配／完整 native 重驗。約 5 是同時使用者估計，不是 5 QPS 或硬上限；不強迫新帳號、SSO 或登入，管理可達性仍待 D16。使用者另確認 public 與 M1；公開 repo swchen44/rustgrok 的初始規劃已提交 main，46 檔 Git blob SHA 讀回一致；Release 尚未建立。Mac M1／arm64 已定，實際及最低 macOS 版本仍未定。

原版搜尋有自訂 query parser、欄位／語言 analyzers、Lucene 排名與跨 project 搜尋，動態 UI 由 JSP 提供。Rust 須重新建立這些行為契約，不能只換 server 或引擎便宣稱同等。[S1][S2]

## 2. 已接受的方向

1. **Rust 負責產品核心。** 搜尋、索引、query interpretation、Web 與 host 管理由 Rust 實作；npm 作分發與必要啟動入口。具體 crate／process 邊界未定。
2. **依確認範圍保持原版功能與 style。** D02 不要求角色／ACL 與 authorization／RBAC plugin parity；其餘頁面、操作、路由及功能目標保留。沿用 CSS／JS／圖示須維持其 DOM／資料／事件契約；JSP 動態邏輯移植 Rust templates／handlers，模板引擎未定。
3. **標準 Rust profile 不帶 JVM。** Java／Tomcat 原 OpenGrok 放在獨立測試參照環境，提供 tokens、query、結果、xref、history 與 UI oracle。不得以 Rust 外殼暗中啟動 Java backend 代替重寫。
4. **Ubuntu 24.04 amd64 正式部署優先，Apple M1 macOS 先開發驗證。** 使用者已確認 M1／arm64，Rust target mapping 為 aarch64-apple-darwin；先完成 Mac 開發與第一輪完整驗證，再做 Ubuntu 平台適配及完整 native 重驗。Mac PASS 不替代 Linux PASS。npm／ZIP 共用同一 Rust 核心，分平台 build／adapter 是工程設計，不另建第二套產品。macOS 實際／最低版本與 Windows 詳細規格依 D03 待定；Windows 保留，WSL 不能代替原生 Windows。實際 Ubuntu point release 隨測試環境記錄。
5. **搜尋速度優先。** D21 已確認首要目標；同語料、硬體、功能與資源條件量測全文 query、path、defs 與跨 project 搜尋。數值 SLO、代表環境、query mix 與可接受資源／更新取捨仍 OPEN，不先承諾加速倍數。
6. **必要 helpers 隨包。** D19 已確認 Rust 核心可附帶必要 Ctags／Git／其他 SCM 工具；將其相依與授權納入發行閉包，不要求使用者另裝必要工具或隱性連外下載。精確版本、build、features 與各 SCM 能力依 G1／原 inventory 驗證。
7. **本機與 LAN 都可使用，約 5 人，無角色／專案 ACL 管理。** 不強迫新帳號／SSO／登入；原 authorization／RBAC plugin parity 不再是 mandatory 產品能力或交付阻擋。非 auth plugins 仍保留盤點，不因這項決定省略。
8. **公開 repo 已建立，發行包放 GitHub Releases。** public 由使用者確認；[swchen44/rustgrok](https://github.com/swchen44/rustgrok) 已建立並經 API 核實，ID 1433282584、default branch main。Owner swchen44 是登入帳號／表單及 API 觀察。建立狀態 CREATED、表單 SUBMITTED_SUCCESSFULLY；規劃內容為 COMMITTED_AND_TREE_VERIFIED：初始 commit [526f619089703f710c915af303e54aea7d25226e](https://github.com/swchen44/rustgrok/commit/526f619089703f710c915af303e54aea7d25226e)、tree `3af8b051938022e067b29a8ccc4d7acac51205b2`，46 檔逐一讀回核對 Git blob SHA 一致。此收據描述初始快照，後續收據修訂不冒稱包含於該 tree；Release 為 NOT_CREATED，產品測試維持 NOT_RUN。

此策略允許必要 native helpers，由 Rust 核心管理；尚未鎖定任何 helper binary，也沒有承諾單一 executable 或所有依賴皆由 Rust 撰寫。

工程提案為 LAN 共用讀者路徑、主機 CLI／本機 control 處理管理。管理操作在哪開放仍屬 D16，提案未被另行批准；無角色／ACL 不等於已批准 LAN 管理寫入。

特定舊 Java plugin 的 bridge **尚未批准**。若 D20 發現需求，須取得新的明確使用者決定，另建可辨識的 profile、依賴／授權清單與獨立測試；bridge 不能取代 Rust 搜尋／索引核心。原版只作 oracle 是目前基線，不預先封死使用者未來明確選擇。

## 3. 提案中的分界

以下供 spikes 使用，未凍結最終 Cargo workspace 或 API：

| 分界 | 提案責任 | 待證據 |
|---|---|---|
| query-compat | 原欄位／escaping／operators → typed AST／相容錯誤 | query 與錯誤 differential |
| analysis／xref | tokens、positions、spans、defs／refs、原 HTML／navigation | 語言、Unicode、Ctags／替代方案與 xref parity |
| search／engine adapter | AST lowering、統計、命中、count、sort、snippets | ranking、跨 project、phrase／fuzzy／regex |
| indexer／storage | 掃描、增量、互斥、jobs、generations、恢復 | 完整／增量等價、故障及平台檔案行為 |
| history／sources | revision、log／blame／diff、來源映射 | JGit／其他 SCM oracle，以及 D19 已確認策略下的 helper builds／能力 |
| web／templates | 原路由、SSR、資產、API；管理可達性依 D16 | DOM、URL、互動、共用讀者路徑及同環境 screenshot，不新增角色／ACL 管理 |
| platform／CLI | 程序生命週期、native payload、診斷 | 真 npm／ZIP、離線冷 cache、原生平台 |

Tantivy 為搜尋引擎候選，Tree-sitter 為語法分析候選。Axum／Tokio、模板、metadata storage 與版本亦未選定。研究 tag／commit 記於 [Rust research lock](../../upstream/rust-research-lock.json)，不是產品 Cargo.lock、toolchain 或 native runtime lock。

## 4. 不能默認相容的邊界

- OpenGrok 預設 AND；Tantivy QueryParser 預設 OR。改 conjunction 仍未處理原 field escapers、大小寫、fuzzy／regex、錯誤及 Boolean 合併。[S1][S3]
- BM25 名稱相同不證明同分／同序；欄位統計、norm、multi-term query、tie 與跨 project 都須驗證。各 project 局部 top-K 合併不自然等同 MultiReader。[S2][S4]
- Tree-sitter CST 與 OpenGrok／Ctags 的 defs、refs、xref 是不同契約；換 parser 不代表全部語言同等。[S5]
- 研究版 Git history／blame 使用 JGit。改用 Git CLI／git2／gix 需比較 rename、merge、blame 與排序。[S6]
- Tantivy 有自己的 index 格式，未證明能讀原 Lucene 索引。XML config、cache、非 auth JAR plugins、Python tools／REST consumers 仍須逐項相容或遷移；D02 已免除的 authorization／RBAC plugin parity 不再作 mandatory 阻擋，其餘不得默默刪除。[S7]

候選庫無法滿足時，先產生可重現差異，評估 Rust adapter／scorer／其他引擎或替代實作。一般技術失敗不自動撤銷 Rust 決定，也不自動批准減少功能。

## 5. 決策狀態與仍待細化事項

| 決策 | 已確認／待細化內容 | 目前界線 |
|---|---|---|
| D02 | **CONFIRMED：local＋LAN、約 5 人、無角色／project ACL 管理** | 不強迫新帳號／SSO／登入；約 5 人不是 QPS 或硬上限；非 auth plugin 仍盤點 |
| D03 | **正式優先部署 Ubuntu 24.04 amd64；先 Apple M1／arm64 Mac 完整驗證，再 Ubuntu native 重驗** | Ubuntu CPU 對應 x86_64，Mac target mapping aarch64-apple-darwin；macOS 實際／最低版本、Windows 規格仍 OPEN，Mac PASS 不代替 Linux PASS |
| D11 | **public 已確認，swchen44/rustgrok 初始規劃已提交並核實** | repo CREATED／ID 1433282584／main；內容 COMMITTED_AND_TREE_VERIFIED，verified_files 46，Release NOT_CREATED |
| D04 | 使用者現版／客製版或 1.14.20 | 1.14.20 僅研究 oracle，非已批准驗收版本 |
| D16 | 管理操作在哪開放仍 OPEN | 主機 CLI／本機 control、LAN 共用讀者路徑是工程提案，尚未批准管理端點可達性 |
| D19 | **CONFIRMED：必要 Ctags／Git／SCM helpers 隨包** | 精確工具／build／各 SCM 能力依 G1；不是已完成分發或相容驗證 |
| D20 | 命中／排序／score、UI／URL／REST、config／index／cache／非 auth JAR／automation | D02 的角色／ACL／authorization parity 排除已明確；其他無默認差異豁免，bridge 未批准 |
| D21 | **CONFIRMED：搜尋速度優先** | 數值 SLO、query mix／規模／環境與 RSS／磁碟／索引時間／新鮮度取捨仍 OPEN |

其他來源、內網、遷移與發行細節依[決策表](../08-decision-register.md)。D02 已確認的權限範圍調整只適用該範圍；搜尋、style 與非 auth 功能不因此放寬。

## 6. 具體選型與產品驗收所需證據

Rust、必要 helpers 隨包、搜尋優先、本機＋LAN 無權限管理、Ubuntu 24.04 amd64 正式部署優先與 Mac 先開發／完整驗證均已確認。Mac M1／arm64 已由使用者確認；macOS 實際／最低版本與 Windows 細節仍待 D03，Ubuntu point release 於實測環境另記。以下門檻以 D02 調整後的功能範圍驗收，authorization／RBAC plugin parity 不再阻擋交付：

1. 可正常執行的固定 OpenGrok oracle、corpus、query／UI／SCM 配置與原始輸出。
2. Parser／tokens／offset differential：長 identifier、標點、Unicode、path、history stopwords。
3. 命中、count、ranking、score／tie、phrase／fuzzy／regex、稀疏欄位與跨 project。
4. 原資產配 Rust templates／xref 的 DOM、URL、行號、快捷鍵、畫面。
5. 已確認 bundled profile 下的 SCM／Ctags 精確 build、能力與三平台來源／程序生命週期；先 Apple M1／arm64 macOS 完整驗證，再 Ubuntu 24.04 amd64 適配及完整 native 重驗，Windows 原生驗收仍保留。
6. 以約 5 位同時使用者作規模參考，設計搜尋優先的全文 query／path／defs／跨 project 端到端基準；請求到達模式、SLO、資料與硬體仍待定，不能直接當作 5 QPS。記錄 startup／index／RSS／disk 與資源取捨，subset prototype 不外推全功能加速。
7. 實際 npm／ZIP 的無網冷 cache／原生平台驗證及完整來源／授權 inventory。

對應 G1、WP-B01–WP-B08；後續 G2–G6 完成正式移植、parity、發行與使用者驗收，依[實作計畫](../04-implementation-plan.md)、[驗證計畫](../05-verification-plan.md)、[相容性研究](../12-rust-compatibility.md)。本輪未編譯或執行 Rust 原型，未有上述測試結果。

## 7. 授權與來源

Rust 是語言決策，不是重新授權決策。OpenGrok LICENSE.txt 宣告預設 CDDL 1.0 only，並提示檔案例外。沿用資產、翻譯／移植碼、tests 與獨立新增碼應分別記錄 origin、commit、notices、修改及適用條款，依實際內容核對發行時的 source 對應要求。[S8]

本 ADR 不預判所有 Rust 檔案的法律屬性，也不宣告全產品可統一改成 MIT。Tantivy Cargo.toml 的 MIT 宣告僅描述該元件，不覆蓋 OpenGrok 或其他資產。產品名稱與新增碼授權仍屬 D17，發行前依實際 source／artifact inventory 核對。

## 8. Primary sources

- [S1] [OpenGrok CustomQueryParser](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/CustomQueryParser.java)、[QueryBuilder](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/QueryBuilder.java)。
- [S2] [SearchEngine](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/SearchEngine.java)、[search.jsp](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/search.jsp)。
- [S3] [Tantivy QueryParser](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/query_parser/query_parser.rs)。
- [S4] [Lucene BM25Similarity](https://github.com/apache/lucene/blob/f965e930673c1f5cb478dc6a8907f5fc4ef7b539/lucene/core/src/java/org/apache/lucene/search/similarities/BM25Similarity.java)、[Tantivy BM25](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/bm25.rs)。
- [S5] [Ctags](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/Ctags.java)、[JFlexXref](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/JFlexXref.java)、[Tree-sitter introduction](https://tree-sitter.github.io/tree-sitter/)。
- [S6] [GitRepository](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java)。
- [S7] [Tantivy index metadata](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/index/index_meta.rs)、[OpenGrok Configuration](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/configuration/Configuration.java)。
- [S8] [OpenGrok LICENSE](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt)、[NOTICE](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/NOTICE.txt)、[Tantivy Cargo.toml](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/Cargo.toml)。
