# rustgrok：GitHub建立準備與Release交接

版本：v0.6；2026-10-25（Asia/Taipei）。本文件是交給後續實作Agent的規劃；公開repo已建立，產品尚未實作或發布。

## 1. Who／what：已確認決策與實際狀態

| 項目 | 狀態與證據 |
|---|---|
| 使用者指示 | 新repo rustgrok、Public、release packages放GitHub；首輪Mac為M1 |
| Repository | [swchen44/rustgrok](https://github.com/swchen44/rustgrok)，已建立 |
| 已回讀metadata | repository ID 1433282584；visibility public；default branch main；push／admin可用 |
| 首個README提交 | 6764a8efa3f4764133c397fc40fc71cafa330444 |
| 完整初始規劃提交 | [526f619](https://github.com/swchen44/rustgrok/commit/526f619089703f710c915af303e54aea7d25226e)；tree 3af8b051938022e067b29a8ccc4d7acac51205b2；46個檔案blob SHA與提交時本地一致 |
| 本次初始規劃提交內容 | README、AGENTS、docs、tests規格、upstream研究台帳、distribution發行模板與規劃包生成腳本 |
| 已確認首輪平台 | Apple M1／macOS arm64；actual／minimum macOS版本仍待D03 |
| 產品狀態 | 沒有Rust runtime、Cargo.lock、正式npm TGZ或產品ZIP；80項產品測試仍planned |
| 正式產品Release | 尚未建立；repository與文件提交不代表產品已發布 |

本輪已依使用者Public決策建立repository，不再詢問可見性、repo名稱或Mac CPU。repo名稱不自動決定產品商標、npm scope/name或D17新程式碼授權。

## 2. 可見性與發行目的地

Public已由使用者確認，GitHub repository及後續公開Release可供公開存取。GitHub官方說明Release取用依repo讀取權限，Release管理需寫入權限；目前取得的是專案建立與內容提交所需存取，不另邀請collaborators或改其他repo設定。來源：[About repositories](https://docs.github.com/en/repositories/creating-and-managing-repositories/about-repositories)、[About releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)。

原程式「不用權限管理」繼續指執行中的搜尋產品，不影響GitHub帳號／repo管理。D11已決不代表D16的LAN管理操作入口也已決。

## 3. How：首次提交與後續維護

1. repo已建立並回讀ID／URL／Public／main；README首個commit已取得真實SHA。完整規劃批次已提交為526f619，並核對遠端tree內容。
2. 將規劃包內容放repo根目錄，不再多包一層opengrok-host-planning。後續若HEAD變動，先讀既有內容與AGENTS再整合，不強制覆蓋。
3. 原始碼、docs、tests、來源台帳、依賴／發行規格皆保存在此repo。企業source/index、憑證、私人語料與暫存下載工具不屬於本產品提交範圍。
4. 用一個可審查的tree／commit提交完整規劃，更新main時核對expected HEAD；提交後讀回tree，比對檔名、blob hash與內容完整性。
5. 產品程式與測試仍未實作，不提交空測試或always-success工作流。文件結構檢查與產品測試的結果分開記錄。
6. [tools/build_planning_package.py](https://github.com/swchen44/rustgrok/blob/main/tools/build_planning_package.py)可在clone後重跑文件、JSON、需求／工作包／測試追溯檢查並生成合訂本與ZIP；操作見[tools/README.md](../tools/README.md)。
7. 版本v0.6是規劃文件版本，不能假作runtime release版本。正式binary完成後再按下列G4–G6發行契約處理。

上述收據指初始規劃快照；本次收據文字與後續變更另以Git history追蹤，不宣稱它們也包含在初始tree，亦不嵌入自身未知commit hash。

## 4. Release資產設計

以下檔名為工程提案，`<version>`、`<confirmed-arch>`與`<package-name>`均是待實作時填入的占位值。

| 產物 | 內容與驗證要求 |
|---|---|
| `rustgrok-<version>-linux-amd64.zip` | 對Ubuntu24.04 amd64驗證的Rust binary、必要helpers／共享庫／assets及啟動說明 |
| `rustgrok-<version>-darwin-arm64.zip` | M1／arm64同版產物；macOS版本另記，原生執行、quarantine／paths／helpers與Rosetta狀態證據 |
| `rustgrok-<version>-windows-<confirmed-arch>.zip` | 已確認Windows64 tuple的產物；不能拿WSL產物充當Windows原生 |
| `<package-name>-<version>.tgz`及平台TGZ | npm主入口與平台payload；名稱依D14，沒有因repo叫rustgrok就確定npm名稱 |
| 完整離線npm套件組 | 包含實際需要的依賴閉包和安裝方式；將TGZ並排放Release不能單獨證明離線安裝成立 |
| release-manifest.json | commit、tag、toolchain／lock摘要、各平台、helpers、每項payload hash及對應test evidence |
| SHA256SUMS | 最終payload及release-manifest的SHA-256；不對自己計hash，避免循環依賴 |
| SBOM、LICENSES／NOTICE與source | Cargo/npm/native/helpers／沿用資產來源與固定版本、對應source；依授權盤點產出 |
| 驗證摘要與已知限制 | 對應實際artifact的功能、UI、平台、離線、復原與效能結果；有未驗平台就明示，不能標全平台PASS |

GitHub會為tag提供repo source ZIP／tarball；這些自動source archives不會取代產品原生包或完整離線依賴組。GitHub官方目前允許每個Release最多1000個assets，每個檔案必須小於2 GiB；若包過大，先在包裝工作包解決，不到發布當天才發現。來源：[About releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)。

具體資料模板為[release-manifest.template.json](../distribution/release-manifest.template.json)。所有hash、版本、run ID仍為null，execution_status為not_run；它不是已產生的Release manifest。

## 5. Build、驗證、發行順序

1. **固定來源**：同一產品版固定source commit、Cargo/npm locks、工具與資產來源；與規劃文件版號分開。
2. **macOS完整階段**：先開發並完成該Mac tuple全部適用功能／UI／工具／npm／ZIP與故障測試，保留原版對照。
3. **Ubuntu完整重驗**：同一核心在Ubuntu24.04 amd64完成平台實作與build，使用真包完整重跑適用功能、UI、安裝、離線、重啟／恢復與性能；Mac PASS不能替代。Linux修正共用核心則回補Mac回歸。
4. **Windows承諾矩陣**：完成已確認Windows64 tuple的原生驗收，按既有G4及後續G5要求簽核；本輪沒有批准縮成Mac-only或Linux-only正式發行。
5. **候選包驗證**：既有PKG-001／PKG-003與REL-001／REL-002承接真包、manifest、sha、依賴與離線流程；產品尚未實作時不建立假runtime Release。
6. **建立Release草稿**：在指定repo以實際tag／commit附上驗證過的原始bytes、checksums與說明；不在上傳前又重新build而使測試hash失效。
7. **檢查與發布**：按既有G6確認Release metadata、來源、完整資產與visibility；正式發布只使用通過對應驗收的產物，不把草稿當已發布。
8. **下載回驗**：REL-005從實際Release下載，核對bytes、checksum、平台與版本；按宣告的取用模式驗證取得及離線安裝。記錄URL、Release ID、asset ID、hash和run。
9. **npm交接**：相同驗證版本的TGZ及發布說明交給使用者，使用者自行發布npm；GitHub Release不等於npm registry已發布。

發行的source commit、manifest及實測artifact hash必須一致。若平台未完成，保留planned／blocked與具體原因，不能用另一平台成功或CI job只完成編譯來替代。

## 6. To do與下一位Agent的接續點

| Owner | 下一步 | 依據與交付 |
|---|---|---|
| 使用者 | D11 Public已確認，無需重答 | repo已建立；後續實作與Release維持同一目的地 |
| 使用者 | 補首輪Mac的macOS實際／最低版本 | M1／arm64及Ubuntu24.04 amd64已確認，不重問CPU |
| 研究／交接Agent | 核對初始規劃提交的tree與blob內容 | repo建立及README提交已有證據，完整提交SHA以Git history核實 |
| 實作Agent | 按44個工作包、同一核心及分平台契約進行 | 先Mac全功能，再Ubuntu重驗；與80項測試規格追溯 |
| 發行Agent | 實作平台build、依賴閉包、manifest及Release流程 | 使用本模板補真值，G4–G6證據完整後執行 |

本輪工作為公開repo建立、初始文件提交、M1／arm64決策更新及Release交接模板。產品程式、原生驗收與正式產品Release發布均尚未執行。
