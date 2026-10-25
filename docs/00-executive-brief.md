# rustgrok：macOS先開發驗證，Ubuntu amd64為主要部署目標

版本：v0.6；2026-10-25（Asia/Taipei）。

## 已確認決策

| 項目 | 目前基線 |
|---|---|
| D01 核心 | Rust實作搜尋、索引、Web與host管理 |
| D19 工具 | A方案：必要Ctags／Git／SCM隨包 |
| D21 效能 | 搜尋速度第一優先，數值SLO尚待確認 |
| D02 使用模式 | 本機與內網共用；約5人同時使用，不做使用者／角色／專案ACL管理 |
| D03 部署目標 | Ubuntu 24.04 amd64優先；保留macOS與Windows64支援 |
| D03 開發與驗證 | 先M1 Mac／arm64完成開發與完整驗證，再Ubuntu amd64平台適配與完整原生重驗 |
| D11 GitHub | Public repo swchen44/rustgrok已建立；release packages放GitHub Releases |

首輪Mac已確認M1／Apple Silicon arm64；macOS實際與最低支援版本仍未提供。先前Linux先開發的v0.4順序已被本輪macOS先開發／驗證取代，但Ubuntu仍是主要部署目標。詳見[15部署與平台順序](15-deployment-and-linux-priority.md)。

## 設計影響

同一Rust核心搭配各平台build／adapter是現行工程提案。macOS完成query/analyzer/ranking、原版UI、SCM、索引、HTTP、native/npm/ZIP及故障驗證後，Ubuntu使用同一版source與fixtures做平台適配和完整原生重驗；跨編譯成功不能代替執行。每個平台各記artifact、環境與結果，不複製PASS。Windows原生支援仍須完成。

本機loopback與LAN使用共用索引和多專案搜尋；約5人維持單主機候選架構，不新增角色、專案ACL或分散式叢集。免登入閱讀是提案；D16的新增／移除專案、重建、設定與stop管理入口仍待確認，建議主機CLI／本機控制面。

性能使用1人參照及約5人互動情境，分記query mix、到達率、cold/warm與背景索引成本。Mac先做同機原版/Rust對照；Ubuntu再做同機對照與內網pilot，不能跨不同硬體直接宣稱Rust收益。SLO、資料量及資源預算仍待補。

## GitHub與Release交接

已建立公開[swchen44/rustgrok](https://github.com/swchen44/rustgrok)，repo ID1433282584、default branch main。[初始規劃提交526f619](https://github.com/swchen44/rustgrok/commit/526f619089703f710c915af303e54aea7d25226e)已保存46個檔案且逐一核對blob SHA一致；產品Release仍未建立。後續將原始碼、docs、tests、CI與release manifest放同一Monorepo，ZIP／npm TGZ與可追溯checksums、SBOM、來源放GitHub Releases；npm由使用者發布。具體交接見[16 GitHub與Release](16-github-bootstrap-and-release-handoff.md)。

目前保留38需求、44工作包、80測試規格，所有產品執行仍planned／not_run。此包是規劃交付，尚無Rust產品、實測速度或正式npm套件。
