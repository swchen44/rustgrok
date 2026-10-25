# 實作交接指引：Rust v0.6

此目錄是規劃包，尚無可執行產品。先讀 README.md、docs/02-requirements.md、docs/08-decision-register.md、docs/10-agent-handoff.md，再讀詳細設計與測試規格。

- 使用者要求與最新答覆優先；Rust 核心重寫（D01）已確定，不再詢問是否改用 Java 包裝。
- 搜尋、索引、Web 與 host 管理在 Rust；npm 是分發入口。Java/OpenGrok 僅供獨立 oracle。特定舊 JAR plugins 若需要 bridge，先處理 D20，不默加 JVM。
- D19 已選 A：必要 Ctags／SCM 工具隨包；D21 已確認搜尋速度第一優先，不重問。D02已確認本機＋LAN、約5人、不用角色／專案ACL。Ubuntu24.04 amd64為主要部署目標；先macOS開發及完整驗證，再Ubuntu平台適配與完整原生重驗，Mac PASS不能當Linux PASS。Mac已確認M1／Apple Silicon arm64；macOS實際／最低版本、Windows細節、精確工具build、原版基線、SLO與資源取捨仍OPEN，不重問Mac架構、Ubuntu版本或CPU。
- 依新答覆移除使用者權限管理／RBAC／project ACL需求；其餘原功能與原style保留，不縮成Git-only、少數語言、近似排名或新UI。不要加必需帳號／SSO；免登入閱讀是提案。管理操作是否開LAN依D16，不自行推論任何人可重建／停機。Windows 原生支持不能用 WSL 代替。
- Tantivy／Axum／Tokio／Tree-sitter 等是候選；版本與功能符合性經 G1 驗證後再凍結。Rust 已選不代表這些庫已選。
- query、tokens、offsets、scoring scope、generation、DOM／HTTP 契約先寫清楚，再並行實作；引擎 defaults 不代表 OpenGrok 語意。
- 保留沿用與移植內容的來源、commit、license、notices；Java 翻成 Rust 不自動消除來源條件。
- 使用者指定新GitHub repo rustgrok，產品程式碼、docs/tests與發行證據放該Monorepo，發行包放GitHub Releases。公開repo swchen44/rustgrok已建立（id1433282584、main）；Public已由使用者確認，不再重問可見性。不得把repo名稱當成npm名稱或新程式碼授權已決。企業source/index/secrets不外傳。
- 工作包用 WP-ID；需求 FR/NFR、決策 Dxx、測試 IDs 各自追溯，不能互代。
- 只有實際執行的測試可標 passed；planned/blocked/skipped 不算通過。結構檢查成功不等於產品通過。
- 性能優化先處理搜尋延遲；以同功能、同資料、同硬體、合理調校／暖機的參照及 Rust release 測量。不得靠漏結果、改排名、延後資料更新或無限資源換取未批准的改善；不預設倍數。
- 使用者自行上傳 npm；準備可審查的 tgz/ZIP、checksums、source/SBOM 與發布指引，不自行公開 npm 發布。
