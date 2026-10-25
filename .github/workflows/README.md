# CI 尚未建立

工作流程設計見 [Monorepo/GitHub](../../docs/07-monorepo-and-github.md)。目前沒有可執行產品或假裝通過的workflow。

Rust核心、repo名稱rustgrok及GitHub Releases已定；Public repo與Mac M1／arm64已確認；接著確認macOS版本、原版驗收版本與依賴policy，再建立真實Cargo checks、native integration、原版差異、npm/ZIP、offline、recovery、performance、SBOM等流程。Java/Maven/Tomcat僅供獨立reference job，不是產品runtime。

airgap與native Windows/macOS測試不能由Linux container替代。使用者自行上傳npm，現階段不配置自動公開npm publish。

開發及第一輪完整驗證先在macOS，其後Ubuntu24.04 amd64平台適配與完整原生重驗；Ubuntu仍是主要部署目標。每平台記獨立artifact與test結果，Mac PASS不能替代Linux PASS。Release交接見[文件16](../../docs/16-github-bootstrap-and-release-handoff.md)。
