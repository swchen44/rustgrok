# 部署與平台：macOS先驗證，Ubuntu24.04 amd64為主要部署

版本：v0.6；2026-10-25（Asia/Taipei）。本文件細化既有需求與工作包，未新增已執行產品測試。

## 1. 使用者已確認的範圍

最新原文：

- 「3, 約5人， 不用權限管理」
- Ubuntu版本「24.04」；最新確認CPU為「amd64」
- 先在macOS開發及全部驗證，再到Ubuntu amd64實作／適配並重驗
- 新repo名稱「rustgrok」，release packages放GitHub Releases

選項3對應同一產品支援本機和內網共用；前一輪問的是同時搜尋人數，因此約5人記為代表同時使用情境。這不表示5QPS、固定最多5個HTTP requests、總帳號數或硬性連線上限。

D02取消使用者角色、RBAC、project ACL與相關authorization plugin的mandatory功能／parity。原版搜尋、排名、UI style與其餘功能保留；不新增必交帳號管理、SSO或登入頁。免登入讀者入口是工程建議，不將「不用權限管理」額外解讀成已批准任何認證機制或管理操作暴露方式。

D03確認Ubuntu24.04 amd64為主要部署目標；macOS先開發及第一輪完整驗證，其後Ubuntu原生完整重驗。Mac已確認M1／arm64，macOS版本與Windows細節尚未指定，三平台支援保留。

## 2. 開發平台與部署目標分開記錄

| 維度 | 使用者已確認 | 尚待提供／工程處理 |
|---|---|---|
| 正式主要部署 | Ubuntu 24.04 amd64 | desktop/server、CPU型號／cores、RAM與磁碟 |
| 開發及第一輪完整驗證 | M1 Mac／Apple Silicon arm64 | 最低macOS／實測OS；原生執行與Rosetta狀態記錄 |
| 第二輪 | Ubuntu24.04 amd64平台實作／適配及完整原生重驗 | 真實24.04.x映像與build/helper依賴 |
| 其他承諾平台 | Windows64 | OS版本、CPU、企業環境；最終原生驗收保留 |
| GitHub | Public swchen44/rustgrok已建立，發行包放GitHub Releases | repo ID1433282584、main；產品Release尚未建立 |

Ubuntu的amd64與Rust中的x86_64是對應架構名稱；具體GNU／musl ABI、toolchain與CPU feature baseline是後續工程選型，不能因已知amd64而假定AVX等擴充一定可用。Mac硬體已由使用者直接確認為M1，對應Apple Silicon arm64；macOS版本不能由CPU型號推定。

現行設計共用同一Rust核心及資料契約，平台差異集中在paths／filesystem、process／signals、鎖、原生依賴與包裝。先在Mac完成所有適用功能、原版UI、工具及發行包流程，再在Ubuntu做必要平台實作與整套原生重驗。這是同一產品的分平台實作，不另建立兩套搜尋邏輯。

Mac PASS只證明該Mac tuple、commit與artifact；Linux仍須自己的完整證據。交叉編譯成功、容器中的單元測試或Mac上的瀏覽器測試，不能單獨取代Ubuntu主機安裝、helper、索引、故障與效能驗收。若Linux修正共用核心，需補對應Mac回歸。24.04.x point release、kernel與映像於實測記錄，不再作本輪版本確認阻擋。

來源：[Rust平台支援](https://doc.rust-lang.org/rustc/platform-support.html)、[Ubuntu發行列表](https://releases.ubuntu.com/)。以上分平台測試方法是工程設計，不是目前已取得的執行證據。

## 3. 同一產品的兩種執行模式

| 模式 | 提供的能力 | 候選部署契約 |
|---|---|---|
| 本機 | 自己的瀏覽器搜尋多專案 | Rust服務使用loopback；同一套CLI、索引、reader與原UI |
| 內網 | 約5人從瀏覽器使用共用服務 | 明確指定LAN listen介面；共用已註冊projects與索引 |
| 共用程式碼 | 搜尋、xref、history、專案與索引工作管理 | 不另寫第二套LAN搜尋實作；npm／ZIP執行相同版本的native binary |

候選為單主機部署，不因約5人加入分散式叢集或多租戶系統。正式CPU／RAM／磁碟預算由D09資料量與benchmark決定。

D05仍需確認source位於主機路徑、分散磁碟或網路掛載。LAN模式不自動授權同步、複製其他人的repo，亦不自動git pull/reset；來源更新方式由D08決定。

前景、背景或登入自啟／OS service由D15確認，不把Ubuntu優先直接轉成必需systemd、root或某個固定port。

## 4. 讀者與管理入口的提案

目前沒有逐使用者／project權限差別，讀者共同存取服務中已註冊project的搜尋、source/xref/raw、history/diff、建議及適用read APIs。顯示與操作保留原版style；不另加角色選單或帳號頁。

D16尚待決定下列管理操作從何處提供：新增／移除project、指定source、完整／增量索引、設定變更、備份／升級與停止服務。

**建議方案（PROPOSED）：主機CLI／本機控制面管理，LAN提供讀者操作。** 這是避免誤解需求的具體方案，尚未被使用者確認。若使用者要LAN網頁管理，再討論明確操作集合與完成／取消行為；不要先把所有管理端點開放，也不要直接刪除既有REST管理自動化，D16／D20先盤點。

這是操作入口的分工，不是新角色／RBAC系統。OS檔案存取、source root限制、程序instance identity、writer鎖及輸入escaping仍屬程式正常契約；本輪不用權限管理不會讓任意路徑或錯誤程序控制變成合法輸入。

## 5. 約5人的搜尋驗證

由既有PERF-001／PERF-002／PRJ-002／RST-010等測項承接，不新增一套完成率。

1. **1人參照**：固定query/corpus，量完整搜尋頁、server分層時間與cold/warm，建立沒有互相競爭的參照。
2. **約5人互動負載**：5個獨立client代表估計情境，執行不同query／projects／頁面。client標籤只是測試識別，不是產品帳號。
3. **共同熱門與cache miss**：包含相同query共用cache及不同queries，量hit/miss/eviction；共享範圍不代表不同project選擇可混用結果。
4. **查詢與背景index**：增量、full／merge與generation切換期間仍比較各query組尾端延遲、queue與source-to-searchable時間。
5. **慢query與取消**：保留昂貴query、錯誤、timeout、取消及未完成request；不以刪樣本製造速度。
6. **人數和請求率分開**：request arrival／think time／每秒queries／實際同時requests在manifest另外記錄，不能由人數直接代填。
7. **門檻待資料**：資料量、硬體、query mix、延遲與資源SLO未定，先只報測量；不因5人情境完成就宣稱容量無限或性能達標。

本機和LAN皆驗；LAN的browser／HTTP時間含實際網路情境，server時間另列。cache不需要逐使用者ACL分區，但仍依projects、query、sort/page、config/analyzer/index generation與source內容版本保持正確。

完整方法見[14搜尋效能設計](14-search-performance-plan.md)與[量測模板](../tests/performance/search-benchmark-manifest.template.json)。

## 6. 先macOS，再Ubuntu amd64的工作順序

| 階段／既有工作包 | 執行內容 | 完成與移交條件 |
|---|---|---|
| 環境契約：WP-A02／G0 | Mac已定M1／arm64，確認macOS版本；Ubuntu24.04 amd64已知；記硬體、browser、registry | 固定可重現corpus、oracle、平台與artifact識別 |
| Mac可行性：WP-B01～B08／G1 | query／analyzer／ranking／UI移植、native與bundled helpers、npm/ZIP及早期搜尋profile | 找出高成本差異；不把subset當全功能 |
| Mac完整功能：WP-C01～D08 | 原版全部適用query、UI、SCM、索引、API及本機／LAN流程 | Mac tuple所有mandatory feature證據齊全，未解相容項不算完成 |
| Mac完整真包：WP-E01～E06的平台階段 | npm、ZIP、空cache／離線、工具閉包、升級／rollback、故障、1人／5人查詢 | 保存Mac完整結果；這是平台階段完成，不自動等於全域G4 |
| Ubuntu適配與重驗：相關C/D/E工作包 | 從同一source版建Linux真包；完成paths、process、locks、helpers及ABI等平台適配，再完整執行適用矩陣 | Ubuntu24.04 amd64各項有自己的runtime evidence；共用修改補Mac回歸 |
| Windows驗收：WP-E06／G4 | 完成已承諾Windows tuple的原生實作及驗收 | 全域G4按正式支援矩陣判定，不能用WSL代替 |
| 效能與pilot：WP-E07/E09／G5 | Mac／Ubuntu各自同機原版對照，主要Ubuntu內網約5人pilot | 正確性、SLO、RSS／IO／freshness成本與使用者驗收 |
| GitHub發行：WP-E10／G6 | rustgrok Release同版ZIP／TGZ／checksums／SBOM／來源及驗證摘要 | tag/commit/artifact/tests相符；下載回核對；npm由使用者發布 |

部署目標優先與開發順序是兩個不同欄位。v0.4的Linux先開發順序已由本輪答覆取代；Ubuntu主要部署、macOS與Windows支援均保留。現階段沒有預先批准Mac-only或Linux-only正式發行。

## 7. 目前交接狀態

D01/D02/D19/D21的方向已確認；D03的Ubuntu24.04 amd64部署及Mac先驗、D09約5人、D11的rustgrok與GitHub Releases已知。M1／arm64與D11 Public已確認，repo已建立；macOS版本、容量與SLO、D16管理入口仍待決。

38需求、44工作包、80測試規格保持；本輪修正FR-017與相關驗證的scope。所有產品測試仍planned／not_run。來源研究不改寫，Ubuntu查核不是產品測試證據。
