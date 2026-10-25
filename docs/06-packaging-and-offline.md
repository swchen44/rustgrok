# Rust 版本：npm 打包、離線交付與跨平台執行

版本：v0.6，2026-10-25。**Rust核心、工具隨包、搜尋速度優先及既定使用模式不變；先在Mac M1／arm64完整開發驗證，再於Ubuntu24.04 amd64適配與原生重驗，正式優先部署仍為Ubuntu。rustgrok採public、發行包放GitHub Releases已確認；實際建立狀態另記。macOS版本、框架等仍待定，尚未產出或驗證可安裝產品。**

本文件取代 v0.1 的 Java/Tomcat 產品包裝設計。需求對照為 NFR-001、NFR-002、NFR-003、NFR-009、NFR-010、NFR-012、NFR-015，以及 FR-013、FR-016、FR-018。GitHub repo名稱rustgrok已確認；npm scope/name與範例CLI命令仍為占位，不由repo名稱代填；確認內容以[決策表](08-decision-register.md)為準。

## 1. 分發與執行邊界

### 1.1 已確定的方向

產品的搜尋／索引核心與自帶 HTTP server 採 Rust。npm 負責提供可安裝的發行包；不將「npm 發行」解讀成搜尋或 server 業務仍由 Node 執行。D19 已確認可隨包附必要 Ctags、Git 及其他 SCM 工具，採 bundled helpers 為主要產品 profile；D21 已確認搜尋速度優先。原版 OpenGrok 的外觀、操作與功能要求繼續有效，D20 未批准放寬前不能縮減相容範圍。

D02已確認同一產品支援local與LAN共用，約5人同時使用，不做使用者角色、RBAC或專案ACL管理。約5人是容量情境，不是5 QPS、5個同時HTTP requests或固定連線上限；資料量、硬體與數值SLO仍待D09／D21補齊。帳號、SSO與登入不列為必交需求；免登入讀者入口可作PROPOSED方案，管理操作暴露仍依D16另定。

候選設計把 CLI 命令、設定、索引工作、HTTP server 與停止／恢復邏輯放進 Rust executable。通用 npm 入口可用很薄的 JavaScript launcher 選擇平台產物並轉交執行；portable ZIP 可直接執行同一版本的 Rust binary，無須 Node、JRE、Tomcat 或 Rust 編譯工具。仍需的作業系統 ABI、共享函式庫與 Ctags／SCM 工具必須另外盤點。

Java/Tomcat 若用於原版 reference deployment，只存在開發與驗證環境；不能混入正式產品的依賴，也不能當作 Rust 功能尚未完成時的隱性 fallback。

以上描述目前的標準Rust profile。特定舊JAR plugins的compatibility bridge只是D20尚未批准的候選；若未來確有需要，須使用者明確決策，另立profile、依賴與tests，不能替代Rust核心或成為隱性fallback。

### 1.2 「原生 binary」不等於全部沒有依賴

Rust 可產生原生可執行檔，但實際產物可能使用 Windows CRT、Linux libc、macOS 系統函式庫，以及第三方 crate 引入的原生依賴。Rust 官方也要求檢查產物是否符合預期的靜態／動態連結；設定 `crt-static` 本身不構成零 DLL／零 `.so` 的證明。[Rust linkage](https://doc.rust-lang.org/reference/linkage.html)

因此須分開宣告：

- **沒有 Java／Tomcat runtime：** 新產品架構的目標。
- **ZIP 不需要 Node：** 原生 Rust 入口可以達成；須以真實發行包驗證。
- **不需要額外安裝工具：** 主要 bundled profile 須提供所需 helpers 及可再散布相依；依賴是否封閉仍須逐平台實測，OS 前提另列。
- **只有一個 executable、完全沒有 sidecar：** 不是現行要求；D19 已接受必要工具隨包，實際檔案由 inventory 決定。
- **全部靜態連結：** 不是預設承諾，也不等於跨 OS 通用 binary。

### 1.3 已確認的隨包工具 profile

D19 已由使用者選定「Rust 核心可隨包附必要工具」。主要產品 profile 是 Rust core＋bundled helpers；精確工具、版本、各 OS／CPU payload、能力與授權仍須依原版功能及 SCM inventory 驗證。

| 主要 profile | 產品行為 | 驗證與限制 |
|---|---|---|
| Rust core＋bundled helpers | Rust 處理業務；所需 Ctags／Git／其他 SCM 工具按已驗證平台隨附 | 乾淨環境不得借用系統預裝工具；helper 與其 DLL／資源／授權一併盤點 |

external-only 或完全禁止 helpers 不是本輪必交產品 profile；若日後出現具體需求，才另立決策、依賴契約與測試。主要 profile 不默默從 PATH 借用未知版本，也不得在安裝或首次執行時隱性連外補檔。隨包許可不授權自動 git pull／reset 或其他 source 更新；更新行為仍依 D08 決定。不得因先驗 Git 而縮減其他原版 SCM 功能。

## 2. npm 與 ZIP 的候選形式

### 2.1 形式比較

| 形式 | npm 安裝需要 Node/npm | 正常入口執行需要 Node | 平台選擇 | 主要取捨 |
|---|---|---|---|---|
| 通用 npm CLI＋薄 JS launcher＋平台包 | 是 | 是，只執行 launcher | npm metadata 與 launcher 選擇 | 使用者記一個套件名稱；需處理 optional dependency 缺失與程序訊號轉交 |
| 各平台 npm 套件，`bin` 直接指向 native executable | 是 | 原生入口本身不需要；shim 行為須實測 | 使用者安裝正確平台包 | 入口更直接，但名稱、npm shim、cmd／PowerShell 相容性要逐項驗證 |
| 完整平台 offline `.tgz` | 是 | 取決於使用 JS 或 native 入口 | 使用者挑平台 | 可在冷 cache 斷網安裝；不得留未封裝的 production dependencies |
| portable ZIP | 否 | 否，直接執行 Rust binary | 使用者挑平台 | 不經 npm；須測執行權限、簽章、移動目錄及所有必要 helper／共享庫 |

建議先驗證「通用 npm 薄 launcher」與「直接執行 Rust 的 ZIP」，兩者皆採已確認的 bundled helpers 策略；其他發行形式依使用者需要採用，不一次擴成全部發佈模式。

npm 官方支援 `bin`、`os`、`cpu`、`optionalDependencies` 與 `bundleDependencies`。optionalDependencies 安裝失敗或被省略時，npm install 仍可能完成，因此 launcher 必須在執行前診斷缺失。[npm package.json](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/)

### 2.2 native executable 能否直接作 npm bin？

可以列為有源碼依據的候選。此次讀取 npm 官方 `cmd-shim` 實作，沒有 shebang 的目標走直接呼叫分支；`bin-links` 則依 Windows／Unix 選 shim 或 link。這表示不能把 npm 的 `bin` 誤認為只接受 JavaScript。[cmd-shim 原始碼](https://github.com/npm/cmd-shim/blob/main/lib/index.js)、[bin-links 原始碼](https://github.com/npm/bin-links/blob/main/lib/link-bins.js)

但目前只有靜態程式碼查核，尚未在指定 npm 版本與三個 OS 實測 native bin。正式承諾前必須驗證：

- Windows 封裝真正 `.exe`，確認 cmd、PowerShell、參數、stdin／stdout／stderr、exit code 與中斷行為。
- Unix 的 executable bits、symlink 與移動安裝 prefix 行為。
- 不在 postinstall 把 JS placeholder 覆寫成 native binary，避免 shim 已按舊檔案類型產生後失配。
- 各平台 package 的 `bin` 路徑指向包內實際檔案；不靠會變動的 node_modules hoisting 路徑。
- 「安裝後 native 入口可不用 Node」與「安裝 npm 套件不用 Node」是不同主張；後者不成立。

本次 npm source 的 blob SHA：cmd-shim `1b6d1d8e491f57400da4a41ed36623b05fa97009`、bin-links `9bf7d72117fbbe642c500548d8636e7beb469548`。這些是研究記錄，不是最終支援 npm 版本的鎖定。

### 2.3 薄 JS launcher 的責任

launcher 只允許負責平台／架構判斷、定位同版 native payload、必要的完整性檢查、參數與標準串流轉交，以及明確的安裝錯誤。它不得另行實作 server、索引佇列、專案註冊、搜尋語法、資料遷移或版本控制操作。

應把 `start`、`stop`、`status`、`index`、`doctor` 等命令交給 Rust binary，令 npm 與 ZIP 使用同一份業務實作。呼叫 native executable 使用明確路徑與 argument array，不串成 shell command。Node 版本與訊號轉交策略在 G1 鎖定；不能假設所有平台都能透明地把父程序 signal 傳給子程序。

unsupported OS／CPU、`--omit=optional`、payload 缺失或版本不一致時，要在啟動之前報錯；不能偷偷改用未知系統版本或改從 GitHub 下載。

## 3. 平台與 artifact 矩陣

### 3.1 目標平台

正式優先部署目標已確認為Ubuntu24.04 **amd64**。開發與驗證順序為先在**Mac M1／Apple Silicon arm64**完成開發與該平台完整適用驗證，再於Ubuntu24.04 amd64進行實作／平台適配及原生重驗。M1不代表某個macOS版本：實際版本／最低OS仍待D03，SDK／toolchain在G1驗證；Windows原承諾保留，G4仍覆蓋全部承諾平台。Ubuntu官方資訊見[releases](https://releases.ubuntu.com/)、[release cycle](https://ubuntu.com/about/release-cycle)。

macOS的完整適用驗證包含功能／原style相容、npm／ZIP、離線、程序與檔案生命週期、恢復及效能量測；Ubuntu以相同契約及平台對應產物原生重跑，另外驗證Linux ABI、helpers與系統行為。**Mac PASS不等於Ubuntu PASS**；兩者的環境、產物hash與證據分開保存，不因共享Rust原始碼或crosscompile成功便沿用另一平台的結果。

| 需求平台 | Rust target映射／候選 | 尚待確認／驗證 |
|---|---|---|
| macOS M1／Apple Silicon arm64 | `aarch64-apple-darwin`（target映射） | CPU與完整首驗已確認；實際／最低macOS、deployment target、SDK、toolchain與簽章待G1驗證 |
| Ubuntu24.04 amd64 | `x86_64-unknown-linux-gnu` | OS／CPU與正式部署優先已確認；GNU target為候選，另驗glibc／共享庫ABI、指令集基線與helpers |
| Windows 64 位元 | `x86_64-pc-windows-msvc` | 原支援承諾保留；x64與最低Windows版本、是否需ARM64、CRT及helper依賴待確認 |

Rust target tiers、OS 最低條件與 toolchain 可用性會變動，實作時須鎖定 toolchain 再確認；不把今天的 `latest` 文件當作發布支援保證。`--target` 編譯成功也不能替代原生運行與功能驗收。[Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)、[Windows MSVC](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)、[macOS targets](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html)

不能用 Rosetta 成功代替 native arm64 驗收，不能用 WSL／Docker 代替 Windows 原生支援，也不能用 `ubuntu-latest` 名稱代替已確認的Ubuntu24.04具體環境。

### 3.2 候選發布產物

發行包放public repo `rustgrok` 的GitHub Releases已確認。下列檔名為候選規則，npm scope/name仍待使用者指定；repo名稱不自動確定npm名稱。每個正式支援tuple提供獨立原生ZIP，manifest標明最低OS與CPU；M1／arm64已確定，macOS版本與Windows細節仍須確認並實測。

| 產物 | 內容 |
|---|---|
| `<package-name>` | npm主入口名稱模板；薄JS launcher、精確平台dependency版本與診斷metadata，不含Node server業務 |
| `<package-name>-<platform>` | npm平台包名稱模板；同版Rust binary、manifest、helpers／非系統共享庫與licenses；實際scope/name依D14 |
| 主入口與各平台的實際npm `.tgz` | 經`npm pack`與安裝驗證的同版TGZ；放GitHub Release供使用者自行發布npm或匯入內網registry |
| `<package-name>-<version>-<platform>-offline.tgz` | offline檔名模板，實際名稱與package metadata由打包流程決定；選定入口＋native payload＋完整production依賴閉包，cold cache可安裝 |
| `rustgrok-<version>-linux-amd64.zip` | 已確認目標Ubuntu24.04 amd64；Rust executable、必要resources/helpers、可再散布共享庫、notices、manifest與說明；無Node/JRE/Tomcat前提，不泛稱所有Linux均支援 |
| `rustgrok-<version>-darwin-arm64.zip` | 已確認Mac M1／Apple Silicon arm64，內容閉包同上；實際／最低macOS、SDK與toolchain仍須確認並實測 |
| `rustgrok-<version>-windows-<confirmed-arch>.zip` | 每個最終承諾Windows tuple提供對應原生ZIP，內容閉包同上；CPU／最低OS待D03，不預設x64 |
| `SHA256SUMS`、`release-manifest.json` | 最終簽章／封裝後每個asset的檔名、byte size、SHA-256；manifest關聯產品commit／tag、tuple、最低OS、features、helpers與原生驗證證據 |
| SBOM、licenses／notices、source archives、release notes | 對應原始碼／crates／native依賴／helpers／assets、授權與應交付來源；包含功能／格式相容、升級與已知限制 |
| 獨立 debug-symbol artifact | 供故障定位的符號檔；是否公開與保留期限待決，不強迫所有使用者下載 |

若 UI 資產已嵌入 binary，無須另外維護 Java 時代的 common engine／WAR 包。各平台會重複攜帶相同資產；是否拆資產應由體積與升級一致性證據決定，不能先加一層容易失配的 runtime dependency。

offline 套件若與公開主套件內容不同，使用明確的 platform/offline package 身分，或以受控完整 bundle 發行；不能對同一個已發布 `name@version` 私下產生不同 bytes，令使用者無法對照 hash。

將主入口與平台TGZ並列在Release，不代表npm會從鄰近檔案自動解析依賴。一般split packages須按精確版本匯入批准registry；完全離線安裝則交付已封閉production依賴的對應平台包，或已驗證的完整匯入方案。以空cache、阻網及實際使用流程驗證，不能用「TGZ下載完成」代替離線安裝證據。

## 4. 自帶 Web Server 與原版 style 資產

Tokio＋axum 是候選；axum 以 Tokio／Hyper 為基礎，能由程式建立 listener 並提供 HTTP 路由。這些元件編入 Rust 程式，使用者不需另裝 Web Server。版本、features、listen與傳輸設定尚待設計確認，不把帳號、SSO或登入系統加入必交範圍。[axum 官方 crate 文件](https://docs.rs/axum/latest/axum/)、[axum serve](https://docs.rs/axum/latest/axum/fn.serve.html)

靜態資產可由 build-time manifest 配合 `include_bytes!`，或經審查的 asset-embedding crate 編入產物。release 必須測試從完全不同的工作目錄啟動、移動整個解壓目錄、刪除 build source tree後仍能提供 CSS／JS／圖示等資產；不能在 debug 模式讀得到檔案便誤判已嵌入。[Rust include_bytes](https://doc.rust-lang.org/std/macro.include_bytes.html)、[rust-embed 文件](https://docs.rs/rust-embed/latest/rust_embed/)

原版 JSP 的動態頁面不能靠嵌入原始 JSP 檔案就運作。Rust 需產生相容的 DOM、URL、資料與互動契約；原 CSS／JS／圖示可沿用，但必須保留來源、授權、hash 與必要的修改紀錄。HTTP caching、content type、資產 URL、壓縮、相對路徑與 error pages 都需對照原版。

D02已確認local與LAN兩種模式。是否需要TLS及憑證來源依部署條件確認；若採用，盤點TLS library、crypto backend、certificate／trust roots，不把外置reverse proxy變成使用者必裝條件。

管理入口的具體PROPOSED方案是：主機CLI或本機控制介面處理重建、設定與停機，LAN入口提供搜尋／瀏覽等讀取操作。此方案尚未由使用者批准；D16／D20須盤點既有REST管理及automation相容，不能默刪原介面。沒有權限管理不等於任何LAN讀者都能重建或停機，也不要求另建使用者角色系統。

## 5. Rust 執行與平台風險

### 5.1 CPU 工作與停止

HTTP async executor 不應被長時間的解析、壓縮、掃描或索引工作佔滿。候選採受限 CPU worker pool、專用工作執行緒或同一 binary 的獨立 worker mode，與 request handling 分離。Tokio 文件指出已開始的 `spawn_blocking` 不能透過 abort 強制停止，無限增加 blocking tasks 也不適合作為 CPU 併行策略。[Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)

停止流程需有「停止接工作→通知取消→完成或撤回正在寫入的狀態→等待 flush→釋放鎖」的契約。採合作式取消時要設計檢查點；使用隔離 process 時要保證 index writer 的失敗恢復。不能把 HTTP graceful shutdown 成功當作索引資料已安全寫入。[Tokio shutdown](https://tokio.rs/tokio/topics/shutdown)

Windows 的 Ctrl-C／Ctrl-Break／console close 等事件有專門語意；Unix signal／process groups 與 Windows child process tree 必須分別驗證。[Tokio Windows signal](https://docs.rs/tokio/latest/tokio/signal/windows/index.html)、[Tokio process](https://docs.rs/tokio/latest/tokio/process/struct.Command.html)

### 5.2 Windows

MSVC target 為候選。CRT 採靜態或動態連結須用實際 dependency report確認；若採動態且系統未提供所需版本，需明示或隨包提供具合法再散布條件的相依，不能假設每台機器都裝過 Visual Studio。

執行中的 `.exe`、DLL、索引 mmap／file locks 仍可能影響升級與刪除。候選策略是停機後切換版本目錄，或校驗後放進版本化 execution cache；應在 G1 比較，避免把 Java 時代的複雜 runtime cache 不經證據原封搬入。無論哪種，npm upgrade/uninstall 都不得刪 workspace 或使索引變成半遷移狀態。

Rust filesystem／process 層要保留原生 path 表示，避免過早轉成 lossy UTF-8。中文、空白、引號、`&`、跨磁碟、UNC、case collision、長路徑與映射權限要按批准範圍驗證；不可自動修改 registry 或要求提權來掩蓋未定的平台條件。

### 5.3 Linux／Ubuntu24.04 amd64（正式優先部署，macOS之後原生重驗）

以已確認的Ubuntu24.04 amd64及候選`x86_64-unknown-linux-gnu`盤點ELF interpreter、NEEDED entries與所需symbol versions。macOS完整適用驗證後，在此平台完成實作／適配與原生重驗；不能直接搬用Mach-O產物或Mac測試報告。是否另採musl、完全靜態或更低glibc基線屬打包實驗，不能只為「看起來單檔」增加未驗證ABI組合。

Rust crates若帶 C/C++依賴、壓縮／加密／資料庫原生庫，也要納入產物與授權盤點。不得依賴 build host 的某份 `.so` 恰好存在。CPU flags不能直接用發行建置機的 `target-cpu=native`，除非支援硬體契約明確如此限制。

### 5.4 macOS

Mac M1／arm64是已確認的首輪開發與完整驗證平台，對應Rust target映射為`aarch64-apple-darwin`；這不選定macOS版本、SDK或toolchain，仍需G1查核。只對實際驗證tuple宣告結果；deployment target、SDK與相依binary最低版本必須一致，編譯成功不代表其他macOS版本也能執行。[Rust macOS targets](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html)

下載後帶 quarantine 的 ZIP要真實驗證，保留 executable bits與已有簽章。是否需要 Developer ID signing／notarization 依 D13／企業政策決定；不能把刪除 quarantine 或關閉 Gatekeeper 當一般安裝步驟。簽章後 bytes 可能改變，checksum與驗證證據要對最終發行產物產生。[Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)

### 5.5 Ctags／SCM helpers

Universal Ctags 有官方 Windows build專案，Git for Windows的 MinGit定位為第三方工具使用的非互動分發；但 MinGit排除部分功能，不能把它當作所有SCM與工具需求都已滿足。[Ctags Windows](https://github.com/universal-ctags/ctags-win32)、[MinGit](https://gitforwindows.org/mingit.html)

依 D19 已確認的隨包策略，runtime manifest 至少記錄固定版本、build flags／features、能力探測、binary hash、必要 DLL／資料檔、授權與 source 來源。包一個 `git.exe`或`ctags.exe`本身不保證依賴完整，也不保證輸出與原版基線一致。

## 6. 內網與離線依賴閉包

### 6.1 三個不同契約

| 契約 | 必須具備 |
|---|---|
| 離線執行 | native binary、嵌入／本地資產、批准的helpers／共享庫、設定與資料都已就緒；browser也無CDN要求 |
| 離線安裝 | 可搬移的完整 `.tgz` 或ZIP；空npm cache與阻網環境仍可完成安裝，不靠postinstall下載或目標端編譯 |
| 離線建置 | 額外準備Rust toolchain/targets、Cargo sources、native compiler/SDK/linker、npm build/test依賴及browser等工具；此項仍待D07確認 |

Rust終端使用者不需 `cargo install`、rustup、C compiler或SDK。正式npm包應帶預編譯成品，不能把建置工作轉嫁給內網機器。

### 6.2 npm offline驗證

候選驗證以實際 `.tgz` 在空 cache執行：`npm install --offline --ignore-scripts --no-audit --no-fund <已交付.tgz>`。通過安裝後必須真的啟動、索引、搜尋與停止；install exit 0不是完整成功。

通用包的所有必要平台metadata與transitive dependencies需能由批准的registry取得。完全offline包應封閉依賴，避免引用沒有一併搬入的平台包。`npm pack`不會自動將任意外部runtime資料或全部dependency graph收齊，必須審查pack內容與冷cache行為。[npm metadata](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/)、[npm config](https://docs.npmjs.com/cli/v11/using-npm/config/)

目前不以未核實的固定數字承諾 npm registry大小上限。產物壓縮／解壓大小與公司mirror限制要實測；`npm publish --dry-run`不能證明遠端server會接受大小與政策。

### 6.3 Cargo與離線build

應提交應用程式的Cargo.lock、鎖定rust-toolchain與release feature集合。`cargo vendor`可保存Cargo依賴；`--frozen`結合locked與offline，但不會神奇提供未準備好的Rust targets、SDK或native build tools。[Cargo vendor](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html)

Cargo build scripts可以編譯或連結C程式碼，也可進行額外build工作；offline-build gate仍需外部阻網與輸入清單，不能只看Cargo的flag。跨編譯時build script執行於host，應使用target相關的CARGO_CFG資訊，避免把host架構條件寫進目標binary。[Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)

## 7. 版本、供應鏈與升級

每個release manifest應記錄：產品commit／版本、原版相容基線、Rust toolchain、target triple、features、linker/native compiler、build OS、CPU基線、資產hash、helper profile及版本、native依賴、binary SHA、授權與source、已完成的平台證據。

使用者選Rust不等於新程式碼或所有資產可隨意換成某個license。複用OpenGrok的CSS／JS／圖示、翻寫或移植原始碼、第三方crates及helpers都需保留各自的來源與授權處理；最後用實際內容產生SBOM與對應source交付，不能只列Cargo依賴名稱。

索引／設定的格式版本跟binary版本分開。新版Rust engine的索引不得預設可被舊版讀取；也不能預設可直接讀原版Lucene index。npm／ZIP切換程式版本不等於資料自動rollback。升級前檢查相容性、保留經驗證的一致備份或採已批准的重建／generation策略，詳見[系統設計](03-system-design.md)。

## 8. 分發工作包與驗證重點

以下 DIST-R 編號為分發拆解參考，正式工作包與test IDs由[實作計畫](04-implementation-plan.md)、[驗證計畫](05-verification-plan.md)及tests/matrix.json統一，避免產生第二套完成率。

| 參考工作包 | 工作 | 退出證據 |
|---|---|---|
| DIST-R01 | 依D19已確認的隨包策略，凍結D03/D07/D13平台前提與具體工具inventory | 每個tuple、工具payload、前置條件與未知項有明確狀態 |
| DIST-R02 | 先macOS封裝與完整適用驗證，再Ubuntu24.04 amd64平台適配／原生重驗；Windows承諾保留 | 各tuple不需Java／Node的ZIP啟動與資產證據；Mac PASS不轉作Linux PASS，G4保留全部承諾平台 |
| DIST-R03 | npm薄launcher與必要時native-bin方案 | 指定npm/shell的args、stdio、exit、signal與錯誤行為記錄 |
| DIST-R04 | 分析CRT/libc/native crates/helper依賴 | 實際依賴清單；乾淨目標環境可解析；無隱含build工具 |
| DIST-R05 | 完整offline.tgz／ZIP與manifest | 冷cache、無網路、ignore-scripts安裝與真正使用流程 |
| DIST-R06 | Windows升級鎖、Mac簽章與跨平台paths | 最終archive與binary的原生測試，不只crosscompile |
| DIST-R07 | 索引取消、worker退出、資料升級／復原 | HTTP停止與資料flush／鎖釋放皆有證據；無orphan helpers |
| DIST-R08 | SBOM/licenses/source/checksum | 全部內容對應產品commit與實測產物，無企業資料混入 |
| DIST-R09 | public rustgrok GitHub Releases與使用者手動npm發布交接 | 實際repo／Release metadata、各tuple ZIP／TGZ、完整依賴、checksums、SBOM／license／source、順序與操作文件 |

release至少覆蓋以下情境：

1. npm normal與ignore-scripts安裝、optional依賴缺失、錯誤OS/CPU、版本不匹配及hash損壞。
2. 完全offline的冷cache安裝與首次啟動；ZIP環境無Node/JRE/Cargo，主要bundled profile屏蔽系統helpers，驗證包內依賴閉包。
3. 先Mac M1／arm64完整驗證，再Ubuntu24.04 amd64適配／原生重驗；Windows cmd／PowerShell與全部承諾tuple仍須驗證中文／空白／特殊字元路徑及移動安裝目錄。
4. browser資產、search／xref／history整條鏈無外網需求；缺helper時不假裝全功能通過。
5. Ctrl-C／停止服務時仍在索引、helper hang／crash、磁碟滿、殘留鎖與PID、兩個實例競爭。
6. 執行中更新、版本切換、資料格式不相容、rollback及uninstall保留workspace。
7. 最終簽章／封裝後hash與實測binary一致，npm／ZIP兩入口使用同一核心版本與相同行為。

上述都是待實測情境，沒有任何一項已PASS。Rust採用本身不提供效能改善保證。D21已確認搜尋速度優先，D09已知約5人同時使用；數值SLO、資料規模／硬體、request分布與CPU／RAM／磁碟取捨仍為OPEN。搜尋速度優先不授權變更排序、減少完整功能或使用無上限資源，也不因約5人場景加入分散式部署為必備條件。

## 9. GitHub與npm發布責任

原始碼、docs、tests、lock與發行描述進新GitHub Monorepo `rustgrok`；各tuple原生ZIP、npm TGZ、debug symbols、source archives、SBOM、licenses／notices、checksums與release manifest放該repo的GitHub Releases。不得把企業source、索引、cache或憑證混入產品repo或npm包。

### 9.1 GitHub Release契約

建立與發行交接見[16 GitHub bootstrap與Release handoff](16-github-bootstrap-and-release-handoff.md)；具體欄位見[release manifest模板](../distribution/release-manifest.template.json)。這些是未建置／未發布的交接模板，不把占位檔名、空hash或待驗證欄位當作實際產物。

每版Release綁定確切tag／commit，附同版平台產物與來源資料。發行程序以最終簽章與封裝後bytes產生checksums，再依manifest核對所有asset；上傳後重新取得或核驗遠端asset的大小／hash。發布動作不能改變已驗證內容，也不能只附Mac產物卻宣稱Ubuntu24.04 amd64已通過。

GitHub官方目前規定每個Release最多1000個assets、每個檔案必須**小於2 GiB**。封裝工作須檢查實際檔案大小與數量；若達限制，調整封裝或分離獨立的source／debug產物，更新manifest並重驗離線閉包，不省略必要執行依賴。此限制不同於npm／公司mirror的政策，後者仍依D07實測。[GitHub about releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)

Release讀取需要repo的read access，管理需要write access；public repo可由所有人存取。D11已確認`rustgrok`採public，未來已發布產物以公開Release交付；是否已建立repo／上傳asset仍須實際metadata與hash證據，不以公開性決策當作交付完成。產品安裝／執行仍不得隱性連外下載helpers或其他缺件。[GitHub release permissions](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)、[GitHub repository visibility](https://docs.github.com/en/repositories/creating-and-managing-repositories/about-repositories)

GitHub自動提供tag時點repo內容的source ZIP／tarball；這不是已建置的native ZIP，也不自動涵蓋repo之外所有helpers／第三方相依的應交付來源。因此仍依實際license inventory附完整對應source與notices。[GitHub source archives](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)

### 9.2 npm發布責任

使用者已說明將自行上傳npm。本計畫交付確切 `.tgz`、發布順序、scope/access設定與核驗方式，**不自動執行公開npm發布**。若採split packages，先發布同版平台包，再發布精確依賴它們的通用入口；發布的是已驗證bytes，不在publish前重新下載或重建不同內容。

GitHub新repo名稱、public與Release目的地已定；npm scope/name仍待指定。trusted publishing／provenance若之後採用，依實際repo與npm能力設定；公開repo不等於已完成npm發布設定，也不以provenance替代功能驗證。[npm trusted publishing](https://docs.npmjs.com/trusted-publishers/)

首次規劃已提交main並回讀核對，見[規劃commit 526f619](https://github.com/swchen44/rustgrok/commit/526f619089703f710c915af303e54aea7d25226e)。目前沒有已完成的Rust產品binary、npm套件、原生ZIP或產品Release；本文件是實作與發行規劃。文件提交／檢查不代表產品測試PASS，也沒有執行GitHub CI。
