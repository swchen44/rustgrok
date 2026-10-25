# v0.1 → v0.2：Rust 決策與變更影響

日期：2026-10-25。這是v0.1→v0.2的歷史規劃紀錄，並非產品release notes。其D19/D21待決狀態已在v0.3被使用者「A+搜尋速度」取代；現行狀態以08與[14搜尋效能設計](14-search-performance-plan.md)為準。

## 1. 變更依據

使用者明確要求改寫 Rust，動機是效能。D01 因此確定，不再以 Java/Tomcat 封裝作產品方向。所有原版功能、style、跨平台、內網、多專案與 npm/GitHub 交付要求保留。

「選 Rust」和「特定 Rust 版本較快」是兩個不同狀態：前者已決；後者是要用產品測量達成的目標。不能以語言選擇直接推導延遲、吞吐、RSS或索引時間。

## 2. 哪些設計改了

| 項目 | v0.1 提案 | v0.2 決策／提案 |
|---|---|---|
| 產品核心 | 沿用 Java OpenGrok／Lucene | Rust 核心重寫已確認；engine 候選經 G1 |
| Web Server | 私有 Tomcat 提供原 WAR | Rust 內建 HTTP；Axum/Tokio 待選型 |
| 動態頁面 | 沿用原 JSP 執行 | Rust SSR templates 移植邏輯，保留原 style 與 DOM／URL |
| 搜尋 | 原 parser/analyzer／Lucene | 自建相容層＋Rust engine adapter；逐層差異驗證 |
| xref／語言 | 原 Java/JFlex 與 Ctags | Rust 分析、xref；Ctags 是否保留待 D19 |
| Git history | 原版 JGit | Git CLI／git2／gix 或其他方案待選；對照 JGit 行為 |
| npm | CLI 管理 Java／Tomcat runtime | 薄入口／平台包分發 native binary |
| ZIP | 是否包含 Node/JRE 等 runtime 待選 | native binary 可直接執行；helpers／簽章與內容待 D13/D19 |
| Monorepo | Node packages＋上游 Java | Cargo workspace＋npm封裝＋原資產＋reference tests |
| 驗證 | 著重部署包裝與原版結果 | 新增 parser/token/scoring/UTF8/SSR/index format 與公平跨語言量測 |
| 舊資料／plugins | 主要沿用原版格式與 ABI | XML／Lucene index／JAR 移植與相容需求需 D10/D20 |
| 授權 | 上游與 bundled runtime | 沿用／翻譯來源逐檔 provenance，再加 Rust crates/native helpers |

## 3. 保留及新增

保留原有 FR-001～FR-020、NFR-001～NFR-015 的目標，依 Rust 修改實作相關驗收文字。新增 FR-021（Rust 核心）、FR-022（明確相容 parser／AST／adapter）、NFR-016（效能與資源改善證據）。總計 **38 項需求**。

保留原 **68 個 test IDs**，更新其產品執行假設；新增 **RST-001～RST-012**，共 **80 項測試規格**。所有狀態仍為 planned，沒有以重新編號掩蓋原需求。44 個工作包 IDs 保留結構，但內容、依賴與交付已改成 Rust；舊 WP 描述不可繼續作現行需求。

D01 已確認。新增 D19 外部工具、D20 相容邊界、D21 性能優先順序。既有 D02～D18 在沒有使用者答覆時仍保持 OPEN；其中 D13 改問 native ZIP 內容與啟動／簽章細節，不預設需在 ZIP 放 Node。

ADR-0001 標為 superseded 保存背景；ADR-0002記錄 Rust 方向與仍待實驗的選型。

## 4. 最先驗證的高風險問題

| 實驗 | 具體輸入 | 應交付的證據 | 通過後才擴大 |
|---|---|---|---|
| Query/token | 原版固定語料、多欄 query、長識別字、繁中／emoji | raw query、AST、token text/position/span 與差異解釋 | 全語言分析與查詢矩陣 |
| Scoring/project | A/B同名符號、稀疏 defs、selected projects、phrase/fuzzy | 命中、次序、統計與 scoring explain、分頁結果 | 大量索引與並行搜尋 |
| UI/xref | 同一 source、首頁/search/xref、同OS/browser | 原資產hash、DOM、行號、links、快捷鍵、screenshot | 全頁 history/diff/help 等 |
| Native/offline | 每個候選tuple、空cache、阻網、無開發工具 | native dependencies、真 npm/ZIP安裝、CLI/stop結果 | 正式release pipeline |
| SCM/tools | 公開history含merge/rename/whitespace，D19各可行profile | 原版JGit/SCM與candidate的annotate/diff/history差異 | 其餘SCM與既有插件 |
| Consistency | 索引中source變動、取消、crash、Windows reader持有 | job/generation/reader recovery timeline與剩餘檔案 | 完整增量、升級與rollback |
| Performance | 已證相容的subset、固定資源／快取／warmup | startup/index/query/RSS/IO分項、瓶頸及不確定性 | 整產品SLO與企業pilot |

這些實驗避免把不確定性推到全部功能寫完後；subset 的成功不代替完整相容或完整效能驗收。

## 5. 如何依結果繼續

若候選 engine 的 defaults 不相容，先修相容層；需要自訂 scorer、tokenizer 或統計時要量測維護與性能成本。若候選 engine 無法滿足契約，再比較其他 Rust 實作方案。Rust 方向不因此自動撤回，原功能也不自動放寬。

若不用任何外部 helpers，語言與SCM能力需擴大自行實作範圍；用實驗結果更新工作量。若允許 helpers，仍需確保原生平台、內網、授權與可追溯來源。

若 Rust 第一輪沒有帶來性能收益，先以 profile 拆開分析、IO、SCM、index、query與HTTP成本。優先改善D21指定指標，報告代價；不得挑掉不利樣本或刪功能製造加速。

## 6. 本輪交付狀態

文件與測試規格已更新為 v0.2；來源靜態研究有可定位連結。尚未建立 Rust prototype、運行原版對照或測到速度差異。規劃包能供後續實作使用；GitHub推送等待D11目標，npm由使用者自行發布。
