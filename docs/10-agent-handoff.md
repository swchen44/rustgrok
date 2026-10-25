# 實作 Agent 交接：Rust 重寫

版本：v0.6；2026-10-25。交給 Codex／實作 Agent 的規劃基線；實際開發前需結合使用者後續答覆。

## 1. 已經決定

**D01：核心以 Rust 重寫。** 使用者最新原文：「我還是想要改寫rust 因為效能比較好」。搜尋、索引、Web 與 host 管理採 Rust；v0.1 Java/Tomcat 產品封裝提案已被取代。Java/OpenGrok 保留為獨立的功能與效能參照。

完整原版功能、外觀與 style、內網、多專案、macOS／Ubuntu 24.04／Windows 64 位元、npm、可提供離線 ZIP、GitHub Monorepo 含 docs/tests 都繼續有效。Rust 決策沒有批准功能或語言／SCM裁減、排序差異、UI 改版或任何效能倍數。

新增使用者原文：「A+搜尋速度」。D19已選必要工具隨包，D21已選搜尋速度第一優先。工具精確build／能力、數值SLO與資源取捨仍待驗證／確認。

D02本機＋LAN、約5人、無RBAC／project ACL已確認。最新D03為Ubuntu24.04 amd64主要部署，先macOS開發及完整驗證，再Ubuntu適配／完整原生重驗；Mac已確認M1／arm64，macOS實際／最低版本未答。D11 Public repo swchen44/rustgrok已建立，發行包放GitHub Releases已確認。原版authorization plugins不再mandatory，其他功能／style保留。

## 2. 角色

產品選擇與最終驗收由使用者負責；企業環境政策由使用者提供或與 IT 確認。實作 Agent 依 SRS／ADR 與新答覆開發，留下可追溯提交。驗證以獨立固定版本原版與實際發行包進行，不能只相信實作者回報。

## 3. 已完成與尚待完成

| 項目 | 現況 |
|---|---|
| 原版研究 | 1.14.20／3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3 的關鍵檔案靜態讀碼；未完整審查整個 repo |
| Rust 可行性研究 | query／analyzer／BM25／跨 project／UI／SCM／分發差異已列入文件 12；未執行 prototype |
| 需求與計畫 | 38 需求、44 工作包、80 測試規格，具雙向追溯 |
| 工程選型 | Rust 已決；engine、HTTP/framework、templates、SCM、metadata backend與版本待 G1 |
| 開發與驗收 | 無產品程式碼／發行包，所有產品測試 planned／not_run |
| GitHub | 公開swchen44/rustgrok已建立，ID1433282584、main；本提交提供規劃，產品Release尚未建立 |
| npm | 由使用者自行發布；目前沒有正式 package 名稱或 tgz |
| ZIP | 本交接 ZIP 是文件包；最終產品 native ZIP 屬後續工作 |

## 4. 開始工作的順序

1. 讀 README、02、08、ADR-0002，再讀 01／03／04／05／06／07／12／13；目標 repo 若已有 AGENTS，先讀既有指引。
2. 把新答覆寫入 Dxx，保留舊決策歷史；不要把 Rust 再變回未定，不重問已授權的動作。
3. D19已選A、D21已選搜尋優先；接著收具體工具inventory、D20相容邊界與D09搜尋規模／硬體；D02不再重問；接著確認D03的macOS實際／最低版本；M1／arm64、Ubuntu amd64、repo名稱與Public不重問，D04基線及D16管理入口仍待決。
4. 先完成不受 OPEN 決策阻擋的來源台帳、功能 inventory、公開 deterministic corpus 與 oracle 方案。具體受阻的工作標明決策，不因此停掉全部工作。
5. 取得指定 repo 後先讀取現況，再以 reviewable commit／PR 保存 docs/tests；不擅選另一個 repo，也不假稱已推送。
6. 先於macOS執行G1高風險實驗，完成Mac全部適用功能／UI／包裝後再進Ubuntu24.04 amd64適配與完整原生重驗：AST/token／scoring scope、原 CSS/JS 搭 Rust SSR、跨平台 native 分發、source一致性、SCM及早期公平 benchmark。
7. G1 有證據後，更新 ADR 與估算，凍結公共契約，才大規模並行 Rust modules。
8. 每個 WP 關聯 Req／Test IDs、實作檔、測試結果與 PR；最後驗收實際 tgz／ZIP，而非只測 source checkout。

## 5. 避免造成返工的實作邊界

### 搜尋

寫 OpenGrok 相容 parser／typed AST／analyzer 契約，再做 engine lowering；不可把 Tantivy 預設 parser 等同 OpenGrok。原版預設 AND、defs/refs 的大小寫、長 tokens、Unicode offsets、phrase／fuzzy／regex、multi-field query 都要測。跨 project 需要相同 scoring statistics scope，不能只合併各 index 的 Top K。

遇到不相容，先定位 token／AST／stats／scorer／renderer 哪層有差，保留原始證據，調 adapter 或重新評估 Rust engine。若需要變更需求，提出具體差異交使用者決定。

### UI

保留原版 CSS／JS／images、DOM ids/classes、URL、行號、shortcut與互動。JSP 動態邏輯移植為 Rust templates，不開新 React 首頁、不換色彩／字型。管理 UI 是否新增依 D16。相同 assets hash 只是證據之一，仍需 rendered DOM、互動與同環境 screenshots。

### 外部工具、舊 plugins 與原始碼

D19已選A：必要Ctags／Git／SCM工具隨包，bundled為主產品方案；B外裝及C無helpers不是目前必交付profile。固定build、平台能力、授權與shared libs，缺件不能隱性下載或借用PATH。原版 Git history 使用 JGit，換成其他 adapter 需對照 rename／merge／blame。D20 不允許默默刪除 plugins，也不允許默加 Java runtime。未解決的必要功能阻擋完整相容驗收。

索引操作不授權 git pull/reset、修改／刪除使用者 source 或上傳企業程式碼。project remove 與 source delete 分開。

### 分發與平台

Rust executable 預編譯；終端使用者無需 Cargo。npm 通用入口候選只負責選平台與轉交程序；ZIP 直接 native binary 無需 Node。shared libraries、Ctags／SCM工具及其資產依 D19 明確。Windows、macOS、Ubuntu 每個承諾 tuple 都要原生執行；交叉編譯成功不是驗收證據。

### 性能與證據

D21已選搜尋速度第一優先。依[14搜尋效能設計](14-search-performance-plan.md)按查詢類別、project範圍、cache與並行索引量測，保留queue、timeout與瀏覽器體感結果；資源／索引新鮮度仍須記錄，不自訂可接受代價。

使用相同 corpus／功能／查詢 mix／硬體／資源預算，原版合理調校及暖機，Rust使用實際 release build。分 cold/warm、首次／增量、並行、RSS／mmap／page cache／IO。微基準不能取代端到端結果；未測量不得宣稱 Rust 版本更快。

所有現有 tests 是規格。PASS 必須有 artifact hash、commit、平台、輸入、步驟、實際結果與證據。上游 CI 或本規劃包結構檢查不能代替產品驗收。

### 發行

版本、來源、license、SBOM、checksums與平台證據一致後才交付至rustgrok的GitHub Releases。Release草稿、發行與下載回驗依文件16；當前沒有產品binary，不把規劃ZIP當runtime發行。新寫 Rust 不代表原版移植內容全部可改成 MIT。使用者目前自行上傳 npm；不要建立自動公開發布工作流。

## 6. 下一輪需要的輸入與可繼續工作

| 決策 | 要取得的具體輸入 | 未答覆前仍可做 |
|---|---|---|
| D19剩餘細節 | A已確認；固定必要工具清單、build／能力、各平台與授權 | bundled adapter與依賴閉包實驗 |
| D20／D10 | 是否有舊設定、Lucene indexes、JAR plugins、API scripts；逐項相容需要 | 公開 fixture、query／UI observable inventory |
| D21／D09 | 搜尋優先與約5人已知；資料量、query mix／到達率、硬體與SLO | benchmark schema、分組查詢與profiling方法 |
| D04／D16 | 實際OpenGrok基線；管理操作主機CLI／本機控制面或LAN入口 | D02已定雙模式約5人無ACL；不新增角色系統 |
| D03／D07 | Ubuntu24.04 amd64與首輪M1／arm64已定；macOS版本、browser／registry與安裝權限 | 準備Mac原生arm64、Linux完整重驗與Windows契約 |
| D11 | Public swchen44/rustgrok及GitHub Releases已確認，repo已建立 | 用Git history核實本規劃提交；後續實作按相同repo保存，不再重問D11 |

## 7. 每次交接必須記錄

repo／branch／HEAD、未提交檔案、已批准與 OPEN 的 Dxx、已完與進行中的 WP、產品 artifact SHA、每項測試最後狀態、原版與Rust版本、已知差異、目前風險、下一步命令及必要環境。不得只留「繼續實作」。
