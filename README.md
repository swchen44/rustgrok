# rustgrok：OpenGrok Rust 重寫研究與實作交接包 v0.6

日期：2026-10-25。  
狀態：**Rust、必要工具隨包、搜尋優先、本機＋LAN約5人且無權限管理已確認。先在macOS開發及完整驗證，再於主要部署目標Ubuntu 24.04 amd64適配與完整原生重驗。Public repo rustgrok已建立；首輪Mac為M1／arm64，GitHub Releases交付已確認；macOS版本等仍待決。產品尚未實作或發布。**  
GitHub repo名稱：**rustgrok**（使用者指定）。產品顯示名稱、npm scope/name與新增程式碼授權仍待決；repo名稱不表示與OpenGrok官方有關係，也不自動保留同名npm套件。

## 已確認的目標

把 OpenGrok 做成內網可用、自帶 Web Server、支援多專案的產品；功能、外觀、style 與操作沿用原版。以 **Rust 重寫搜尋、索引、Web 與 host 管理主體**，目的是改善效能。目標環境保留macOS、Ubuntu 24.04 amd64、Windows64；macOS先開發及第一輪完整驗證，再做Ubuntu amd64適配與完整原生重驗，主要部署優先為Ubuntu；以 npm 分發並交由使用者上傳 npm，也可交付離線 ZIP。原始碼、docs、tests 與開發紀錄進同一個 GitHub Monorepo。重大決策要與使用者討論，不猜測意圖。

使用者先確認「我還是想要改寫rust 因為效能比較好」，再回答「A+搜尋速度」。現行方向是 Rust 核心、必要 Ctags／SCM 工具隨包、搜尋速度第一優先。原版 Java 保留為獨立參照；v0.1 Java/Tomcat 產品包裝建議已被取代。

既有決策：「3, 約5人， 不用權限管理」。本機與LAN都要；約5人是同時使用情境，不是5QPS或硬性使用者上限。最新答覆確認Ubuntu為amd64，先在macOS完成開發與完整驗證，再於Ubuntu驗證；新建repo叫rustgrok，release packages放GitHub Releases。最新回覆另確認Public與M1；首輪Mac架構為Apple Silicon arm64，macOS實際／最低版本仍待提供。

## 候選設計

- Rust native executable 提供 CLI、HTTP、搜尋、索引與多專案管理。
- Axum/Tokio、Tantivy、模板引擎與 SCM adapters 是待實驗的工程候選，尚未選定產品版本。
- Rust 查詢相容層、分析器與搜尋引擎 adapter 保留原版語意；不能直接採用候選套件預設 parser／tokenizer／ranking 就宣稱相容。
- 原 CSS／JS／images 等資產保持來源與 hash；JSP 動態邏輯改成 Rust templates，驗證 DOM、URL、快捷鍵、xref 與可見畫面。
- npm 負責分發與啟動；離線 ZIP 可直接執行 Rust binary，無需 Node 或 Tomcat。共享函式庫與外部工具仍須逐平台盤點。
- D19 已選 A：必要 Ctags／Git／其他 SCM 工具隨包，正式工具清單、版本與平台能力仍要驗證；D21 已選搜尋速度優先。D20 原設定／索引／plugins 與排序相容邊界，以及數值 SLO、容量和資源取捨仍待確認。
- Rust 改善效能是目標，尚無產品量測結果或提升倍數。

OpenGrok 研究基線為 1.14.20，commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3。它不能代替使用者目前版本／公司客製版的正式驗收基線（D04）。

## 閱讀順序

| 文件 | 用途 |
|---|---|
| [00-executive-brief.md](docs/00-executive-brief.md) | Rust 方向、核心結論與優先決策 |
| [01-upstream-research.md](docs/01-upstream-research.md) | 原版靜態讀碼與來源，區分 oracle 和產品 |
| [02-requirements.md](docs/02-requirements.md) | 38 項功能與非功能需求及未決邊界 |
| [03-system-design.md](docs/03-system-design.md) | Rust 元件、資料契約、查詢、索引與生命週期 |
| [04-implementation-plan.md](docs/04-implementation-plan.md) | 44 個工作包、依賴、G0–G6 進出條件 |
| [05-verification-plan.md](docs/05-verification-plan.md) | 原版差異、平台、離線、故障與效能驗證 |
| [06-packaging-and-offline.md](docs/06-packaging-and-offline.md) | Rust binary、npm、ZIP 與離線依賴閉包 |
| [07-monorepo-and-github.md](docs/07-monorepo-and-github.md) | Cargo/npm Monorepo、CI、Issues、PR 與 GitHub |
| [08-decision-register.md](docs/08-decision-register.md) | D01／D02／D11／D19／D21已決，M1先驗／Ubuntu部署及剩餘事項 |
| [09-risk-register.md](docs/09-risk-register.md) | 返工風險、提早驗證與處置 |
| [10-agent-handoff.md](docs/10-agent-handoff.md) | 給 Codex／實作 Agent 的指引 |
| [11-traceability.md](docs/11-traceability.md) | 38 需求、44 工作包、80 測試規格的雙向對照 |
| [12-rust-compatibility.md](docs/12-rust-compatibility.md) | parser／analyzer／BM25／xref／SCM 的讀碼差異 |
| [13-rust-change-impact.md](docs/13-rust-change-impact.md) | v0.1 → v0.2歷史變更與先做的實驗 |
| [14-search-performance-plan.md](docs/14-search-performance-plan.md) | 搜尋優先的設計、量測、實驗順序與決策變更 |
| [15-deployment-and-linux-priority.md](docs/15-deployment-and-linux-priority.md) | 本機＋LAN、約5人、macOS先驗與Ubuntu amd64部署 |
| [16-github-bootstrap-and-release-handoff.md](docs/16-github-bootstrap-and-release-handoff.md) | rustgrok建立準備、Release產物與交接狀態 |
| [sources.md](docs/sources.md) | 一級來源、版本與查核限制 |
| [tests/README.md](tests/README.md) | 測試目錄與證據規約 |
| [tools/README.md](tools/README.md) | 可重跑的規劃文件檢查與封裝腳本 |
| [tests/matrix.json](tests/matrix.json) | 80 個完整測試規格；全部 planned |
| [upstream/research-lock.json](upstream/research-lock.json) | 原版參照用鎖定資訊 |
| [upstream/rust-research-lock.json](upstream/rust-research-lock.json) | Rust 候選研究紀錄；不是產品 Cargo.lock |
| [ADR-0002](docs/adr/0002-rust-core-and-compatibility.md) | Rust 方向已接受；候選技術選型仍待證據 |

## 完成狀態與交接

完成官方文件與關鍵原始碼靜態研究，以及需求、設計、實作、驗證與分發計畫更新。未宣稱完整逐行審查整個上游，也未在目標 OS 建置／啟動產品。全部 80 測試是規格，沒有產品功能、效能、離線或視覺 PASS。

這份 ZIP 是可提交 GitHub 的**規劃與文件包**，不是已可執行的 Rust 程式或最終產品離線包。沒有用空白 tests／占位 npm test 偽裝可驗收產品。

公開repository已建立：[swchen44/rustgrok](https://github.com/swchen44/rustgrok)，預設分支main，repo ID 1433282584。完整規劃已在[初始提交526f619](https://github.com/swchen44/rustgrok/commit/526f619089703f710c915af303e54aea7d25226e)保存至repo根目錄，46個檔案的Git blob SHA已逐一回讀並與提交時本地一致。此後續交接紀錄更新另以Git history追蹤。正式產品發行包將放[GitHub Releases](https://github.com/swchen44/rustgrok/releases)；目前沒有產品Release。企業source、索引與憑證不屬於要上傳的產品原始碼。npm由使用者自行發布。
