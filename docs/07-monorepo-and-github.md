# Rust Monorepo 與 GitHub 設計

版本：v0.6，2026-10-25。**Rust與既定功能範圍不變；Mac M1／arm64先開發／完整驗證，再於Ubuntu24.04 amd64適配／原生重驗，正式部署仍Ubuntu優先。public repo swchen44/rustgrok已實際建立，發行包放GitHub Releases。** 規劃內容尚待提交，尚無Release；macOS版本、npm scope/name與新程式碼授權仍待定，具體工具仍為候選。

## 1. Repository 的責任

public Monorepo [swchen44/rustgrok](https://github.com/swchen44/rustgrok)已建立，用於保存Rust原始碼、OpenGrok相容研究、原版style資產、模板、npm分發描述、docs、tests與發行紀錄。帳號、名稱、public與Releases用途已確認，實際metadata見第10節。下列`grok-host`路徑為工程占位，npm名稱用`<package-name>`模板，不因repo定名而自動凍結。

Cargo workspace管理Rust程式；npm workspace或獨立npm package模板只管理薄launcher、平台包描述及需要的前端測試工具。npm不是第二套server架構，也不在Node層維護另一份專案／索引狀態。產品不以Java、Tomcat或Maven為runtime前提；原版Java reference若保留，只服務驗證。

D02的同一產品支援local與LAN共用，不做使用者角色／RBAC／專案ACL管理；不新增帳號、SSO或登入為必交模組。約5人是容量情境，不是固定QPS／requests上限，也不要求分散式架構。免登入讀者入口，以及「主機CLI／本機控制介面管理、LAN只讀」皆保留為PROPOSED；管理暴露與既有REST相容依D16／D20定案。

Cargo官方workspace支援共用Cargo.lock、依賴與metadata，root profiles負責共同建置設定，因此適合跨多個Rust crate維持一致版本與feature規則。[Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

這份交接包目前完成的是文件、研究lock、測試規劃與模板，尚未建立可運行的Rust crate、Cargo.lock或npm套件。遠端repo已建立，但規劃內容commit尚待提交，產品Release尚未建立；各階段分開記錄。規劃一個路徑不等於該模組已完成。

## 2. 候選目錄

| 路徑 | 責任／內容 | 發行或保留方式 |
|---|---|---|
| README.md | 使用情境、相容基線、支援矩陣、真實release與操作入口 | GitHub；npm可生成精簡說明 |
| Cargo.toml | virtual workspace、members、共同metadata/dependencies/lints/profiles | GitHub；實作時建立 |
| Cargo.lock | 應用程式完整Cargo依賴版本 | GitHub；非終端使用者建置前提 |
| rust-toolchain.toml | 已驗證的toolchain／components／targets | GitHub；目前版本OPEN |
| .cargo/config.toml | 必要alias與target建置設定；如需離線build再加source replacement | GitHub；不得硬編個人絕對路徑 |
| apps/grok-host/ | 唯一產品CLI入口；組裝server、indexer與host lifecycle | 每平台原生binary |
| crates/core/ | project/job/config等領域型別、穩定契約與錯誤類型 | 內部crate，不先公開crates.io |
| crates/query-compat/ | OpenGrok/Lucene相容query parsing、validation、canonical query model | 內部crate；不可偷換成引擎預設語法 |
| crates/analysis/ | 語言與token分析、symbols提取及D19隨包策略下經inventory驗證的工具adapter | 內部crate |
| crates/search/ | 搜尋引擎抽象、query lowering、ranking與結果取得 | 內部crate；Tantivy等候選未定 |
| crates/indexer/ | 掃描／增量／工作排程、分析串接、寫入與失敗恢復 | 內部crate／必要時同binary worker mode |
| crates/xref/ | 原版cross-reference與source顯示所需資料／HTML相容處理 | 內部crate |
| crates/history/ | SCM偵測、history／annotate／diff相容契約及adapter | 內部crate；依既有SCM inventory與D19隨包策略，不默縮為Git-only |
| crates/web/ | HTTP路由、原版相容回應、頁面render、static assets及local／LAN入口；管理暴露依D16 | 編入Rust executable；axum/Tokio仍為候選，不必交RBAC／project ACL或登入模組 |
| crates/platform/ | 原生paths、process、signal、locks及OS差異 | 內部crate；不放搜尋業務 |
| crates/storage/ | project/config/job metadata與資料格式／migration契約 | 內部crate；與search engine儲存責任分清 |
| assets/upstream/ | 固定OpenGrok基線的CSS／JS／icons等；來源、hash與license清單 | 選定資產編入binary或受控payload |
| templates/ | Rust動態頁面模板及原版DOM／互動契約 | 編入binary或受控payload；不是直接執行JSP |
| npm/cli/ | 通用npm入口的薄JSlauncher與package metadata | 候選主npm套件 |
| npm/platforms/ | 各OS/CPU native package的模板與封裝manifest | 候選平台npm套件；不把全部binary放Git |
| xtask/ | Rust開發／建置輔助：驗資產、打包、SBOM、release manifest與reference orchestration | 開發工具，不隨產品強迫終端使用者執行 |
| docs/ | 需求、設計、實作、驗證、決策與維運文件 | GitHub；本交接包的主要交付 |
| distribution/ | release manifest模板與發行產物清單契約 | GitHub；模板不是已建置或已發布證據 |
| docs/adr/ | Rust選擇、query/index相容策略、web/render與分發決策 | GitHub，記錄批准／未決／取代狀態 |
| docs/platform-support/ | tuple、最低OS／Node/npm、ABI、權限與限制 | GitHub |
| docs/distribution/ | 內網mirror、offline閉包、helpers與手動發布 | GitHub |
| docs/release/ | 每版功能差異、資料相容、升級／回復與已知問題 | GitHub及release摘要 |
| vendor/locks/ | crate以外的工具、assets、原版source、binary與license鎖定 | GitHub |
| vendor/sources/ | 如需完整對應來源或離線build，保存批准的source snapshots | 依體積與授權放Git或對應source Release |
| upstream/research-lock.json | 已查核的原版OpenGrok版本／commit、來源與研究狀態 | GitHub；供oracle使用，不是產品runtime依賴 |
| tests/reference/ | 原版OpenGrok oracle部署／設定與可重現環境描述 | 僅測試；Java/Tomcat依賴不進產品包 |
| tests/fixtures/ | 可公開合成source/history、edge cases與licenses | GitHub |
| tests/contract/ | 跨crate、CLI、配置、query AST、API與資料格式契約 | GitHub |
| tests/parity/ | 原版對Rust的功能、結果、query／ranking／history差異 | GitHub；大差異檔存CI artifacts |
| tests/visual/ | DOM、資產hash與各批准OS/browser畫面 | GitHub基線＋CI artifacts；只用公開fixture |
| tests/install/ | 真正npm TGZ/native-bin／ZIP安裝與卸載 | GitHub |
| tests/offline/ | 空cache、阻網、無toolchain／Java／Node的適用情境 | GitHub |
| tests/platform/ | Windows/mac/Linux paths、signals、child processes、ABI | GitHub |
| tests/recovery/ | 取消／crash／disk-full、generation恢復、升級與rollback | GitHub |
| tests/fuzz/ | query／analyzer／path／renderer的property與differential fuzz、最小回歸樣本 | GitHub；只用可公開fixture |
| tests/performance/ | 固定資料與硬體、約5人使用情境、性能／資源measurement schema | GitHub；人數不等於QPS或同時requests，D21數值SLO與資源預算仍OPEN |
| tests/evidence/ | 證據schema、摘要與artifact manifests | GitHub索引，大檔不塞Git |
| .github/workflows/ | docs、Cargo、原生平台、相容、封裝與release流程 | GitHub；建立後須實際執行 |
| .github/ISSUE_TEMPLATE/ | 工作包、bug、相容差異與決策範本 | GitHub |
| .github/PULL_REQUEST_TEMPLATE.md | Why/What、需求／工作包／test IDs、證據及風險 | GitHub |
| LICENSES/、THIRD_PARTY_NOTICES/ | OpenGrok複用內容、Rust crates、helpers等授權與來源 | GitHub與必要發行包 |

上述crate劃分是責任邊界，實作時可依耦合與編譯成本調整；不能為了符合目錄表建立一堆沒有契約與用途的空crate。是否公開任何Rust crate到crates.io並非本次要求，預設只規劃產品binary與npm發佈。

## 3. 依賴方向與避免兩套實作

`apps/grok-host`負責組裝。`core`保持領域契約，避免依賴axum、特定搜尋引擎或npm。`query-compat`產生穩定的相容query model，再由`search` adapter轉成所選引擎操作；不能讓引擎預設parser反過來決定原版功能範圍。

`indexer`串接analysis、xref、history與儲存／搜尋writer；web透過明確服務契約呼叫search及host操作。`platform`封裝OS差異，npm launcher僅轉交入口。相同的index、project、server及migration邏輯不得在JS、Rust CLI和HTTP handler各寫一份。

以下屬需要ADR的責任分界：

- `storage`管理的project/config/job metadata與`search`引擎的實體索引資料如何共同提交或恢復。
- query AST、檔案／symbol IDs、project選擇／搜尋scope與error model由哪個crate定義；不加入依使用者區分的project ACL。
- xref輸出與動態頁面render如何保留原版DOM、URL和escaping規則。
- helpers主要profile已選bundled；逐平台固定必要工具、版本、能力、授權與依賴閉包。external-only或禁止helpers不列為本輪必交profile。
- server與index worker在同process、獨立執行緒或獨立process的取捨；不得用無限CPU並行掩蓋效能問題。

上述契約在大量實作前先完成，避免多位AI Agent各自定義不同的path/query/job格式。

## 4. 原版style、來源與Java reference的區隔

`assets/upstream`保留固定基線、來源檔案名、hash與授權；`templates`保存Rust頁面實作。沿用原版style不代表原JSP可由Rust直接執行；動態邏輯移植與API相容屬正式工作，必須有差異測試。

OpenGrok原始碼、設定及release可以保留作reference。如何完整提供對應來源、是否納入vendor snapshot、是否另附source archive，按license與repo體積決定；不能只留不固定的master連結，也不能聲稱翻寫語言就自動消除原始授權義務。

Java/Tomcat oracle只在`tests/reference`及相應CI工作中使用。production artifact allowlist要確認沒有意外帶入WAR/JAR／Java runtime，也沒有在Rust執行時偷偷呼叫oracle。產品驗證可需要Java；使用者安裝產品不需要Java。這兩個前提須在開發README與使用README分開說明。

原版與Rust的輸入版本、Ctags／SCM工具、locale／timezone、browser／fonts、source revision及設定要記錄，避免把環境差異錯認成Rust功能差異。Java reference本身有bug或平台限制時，需列差異並討論，不能自動當作放寬D20。

## 5. 版本、locks與建置契約

| 項目 | 管理方式 |
|---|---|
| 產品版本 | 獨立於OpenGrok版本；由workspace共用metadata或受控release manifest產生 |
| Cargo dependencies | 應用程式提交Cargo.lock，workspace統一相依版本與必要features |
| Rust toolchain／MSRV | 選定並記錄確切版本；最低Rust版本是開發契約，不是使用者需安裝runtime |
| npm launcher／平台包 | 同release精確依賴；package模板由產品manifest生成，避免手工不同步 |
| 原版相容基線 | 固定OpenGrokcommit／設定與功能inventory；不是「永遠相容最新master」 |
| helpers／非Cargo來源 | 固定release/source commit、hash、OS/CPU、build flags、license／source availability |
| UI資產 | 原始hash、取用清單、必要修改與產物hash |
| 資料格式 | index／config／job／source-map schema獨立版本，記錄遷移／重建及rollback條件 |

Cargo feature集合是發行契約的一部分。測試應對每個實際發行feature組合執行；若features互斥，不應用一次`--all-features`成功或失敗概括所有組合。crosscompile只產生目標binary，不提供runtime驗收。[Cargo build](https://doc.rust-lang.org/cargo/commands/cargo-build.html)

內網使用不自動要求離線build。若D07要求，另準備Cargo vendoring／source replacement、toolchain/targets、SDK/native工具與npm測試依賴，並在阻網環境驗證；詳見[打包與離線設計](06-packaging-and-offline.md)。

## 6. GitHub內容歸屬

| 內容 | 位置 |
|---|---|
| Rust／薄JS source、templates、必要assets、docs、tests、locks與manifest | Git repository |
| 使用者決策與批准記錄 | docs/08-decision-register.md與ADR；Issues連結，不能只留聊天 |
| 詳細工作分解 | 依04建立Issues／milestones；04是唯一工作包權威清單 |
| 每次詳細測試log、screenshots、query diffs、JUnit/JSON | GitHub Actions artifacts，PR附摘要及manifest |
| native binary、npm TGZ、portable ZIP、debug symbols、source archives、SBOM、checksums | GitHub Releases |
| 企業被索引source、索引檔、私人完整logs、tokens | 使用者內網環境；不隨產品repo或npm包上傳 |

「產品開發工作都放GitHub」不是授權公開企業被索引程式碼。所有pack/release使用明確allowlist檢查；.gitignore本身不能保證npm或ZIP不帶入多餘資料。

不讓日常build自動執行大量外部下載。可以有明確的、hash驗證的依賴取得階段；真正的build與pack以鎖定輸入執行，fetch來源與允許網路範圍可查核。

## 7. CI工作流程候選

| Workflow | 核心工作 | 退出／阻擋條件 |
|---|---|---|
| docs-contracts | 決策／需求／工作包／test IDs、內部links、schema與來源manifest | 不可留無說明的缺對照或失效契約 |
| cargo-checks | 指定toolchain的format／lint／必要unit及contract tests | 真正契約回歸；不靠空test維持綠燈 |
| native-platform | 先Mac M1／arm64開發並完成完整適用驗證，再Ubuntu24.04 amd64適配／原生重驗；paths／process／signals／locks | macOS實際／最低版本及Windows細節仍待D03；Mac PASS與crosscompile不代替Ubuntu PASS，G4仍覆蓋所有承諾tuple |
| reference-parity | 原版Java oracle與Rust對相同fixture執行query/xref/history/API | 非批准差異、被默默縮掉的功能阻擋 |
| visual-regression | 原資產hash、DOM、同OS/browser畫面及互動 | 未批准style／layout／操作改動阻擋 |
| binary-dependencies | 檢查PE/ELF/Mach-O連結、helpers、crate/native來源及licenses | 未宣告共享庫、錯CPU、ABI或source缺失阻擋 |
| package-install | 實際TGZ的薄launcher／如採native-bin入口；缺失平台包與卸載 | args／stdio／exit／signal或內容閉包問題阻擋 |
| package-offline | 冷npm cache／阻網安裝；ZIP無Node／Java／Cargo；主要bundled profile屏蔽系統helpers | 隱性網路／借用未宣告工具／缺資產阻擋 |
| recovery-upgrade | worker crash、取消、資料格式變更、running exe與rollback | 不可恢復寫入、資料遺失或誤認process阻擋 |
| performance | Mac M1／arm64首輪完整量測後，在Ubuntu24.04 amd64按等價情境重新量測；約5人、query延遲及索引／RSS／CPU／磁碟成本分平台記錄 | 人數不等於5 QPS／同時requests；數值SLO待定，不以Mac速度代填Ubuntu結果或只跑toy corpus算PASS |
| release-build | 人工選取可查核commit／tag，產生最終平台產物、必要簽章與manifest | 產物hash、功能、OS與測試證據必須相符；不自動發布npm |

Java oracle所需環境與Rust產品clean-machine環境應分開，以免oracle預裝的Java/Git/Ctags遮住發行包缺檔。已確認的主要bundled-helper profile須屏蔽系統helpers，直接驗證包內工具與其依賴；若日後另批准external profile，另建測試環境，不混用證據。工具隨包不授權自動下載或git pull。搜尋速度優先也不降低原版排序／功能契約，不視為可無限增加資源的許可。

GitHub-hosted runners的預裝工具與OS會變動。精確runner／原生VM／self-hosted環境由G0/G1確認；缺少某個tuple就標未驗證。Linuxcontainer可以輔助測Linux環境，不能充當Windows或macOS的原生執行證據。

Ubuntu24.04 amd64為正式優先部署平台，開發／驗證順序是已確認的Mac M1／arm64先、Ubuntu後。M1對應`aarch64-apple-darwin` target映射；macOS實際／最低版本、SDK／toolchain與Windows細節仍需D03／G1確認，不由CPU推定OS。完整功能、原style、npm／ZIP、離線與恢復證據分tuple保存；Mac通過後仍須Ubuntu原生重驗，G4全平台承諾不變。平台表見[06分發設計](06-packaging-and-offline.md)。

簽章可能改變binary bytes，release證據應對最終簽章與封裝後產物建立。版本不可在測完之後又重新抓latest dependency或換成另一份未測binary；發布來源是已驗證artifact，不是臨時重建的同名檔。

## 8. Issues、PR與多人／多Agent協作

工作包由[04實作計畫](04-implementation-plan.md)轉成Issues，必要時用milestones表達gates。每項Issue至少包含需求IDs、待決項、輸入／依賴、輸出檔案、契約、具體test IDs、證據與風險。06的DIST-R僅作分發拆解參考，不另開第二套進度統計。

小型feature branches以一個可驗收契約為單位。先合併共用query／project／job／storage contracts及fixture，再合併依賴它們的實作。可以平行做平台封裝、原版inventory、語料、license盤點；不能各自選不同的query語法或index schema後才靠整合測試碰撞。

PR描述先說明問題與使用者影響，再寫改動及行為；附requirement/work package/test IDs、實際測試證據、資料相容性、原版差異及必要回復方式。更新snapshot不能同時默默放寬視覺／排名差異容許值。

建立實際workflow後才設定branch protection與required checks，避免先要求不存在的check。是否要求reviewer、commit／release簽章由repo owner決定，不以規劃文件擅自更改GitHub政策。

## 9. 發行與使用者的發布權限

產品release tag對應`rustgrok`完整Monorepo tree與runtime/helper/asset manifest。每個承諾tuple的原生ZIP、同版npm主入口／平台TGZ、離線依賴閉包、checksums、SBOM、licenses／notices與source放該repo的GitHub Releases；大binary不塞Git歷史。split npm packages先供應平台包再供應主入口，版本必須精確匹配。

檔名規則以`rustgrok-<version>-linux-amd64.zip`（Ubuntu24.04）、`rustgrok-<version>-darwin-arm64.zip`（Mac M1）及`rustgrok-<version>-windows-<confirmed-arch>.zip`為提案；npm使用`<package-name>`模板，不代選scope/name。詳細流程見[16 GitHub bootstrap與Release handoff](16-github-bootstrap-and-release-handoff.md)，asset欄位見[release manifest模板](../distribution/release-manifest.template.json)；占位與空hash不是建置或發布證據。

Release manifest逐asset記錄檔名、大小、SHA-256、tuple、最低OS、依賴及原生驗證證據；上傳後核驗遠端bytes與本地已測產物一致。普通TGZ集合不自動構成離線依賴閉包，仍須冷cache／阻網安裝驗證。GitHub自動source ZIP只代表tag時點repo內容，native產物及repo外應交付來源另附。[GitHub about releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)

GitHub目前每個Release最多1000 assets，每個asset須小於2 GiB；有read access可讀Release，write access才可管理。D11已選public，正式發布的Release採公開取用；仍需實際repo／asset建立與下載核驗，不能把公開性決策當交付完成。限制與超限處理見[06分發設計](06-packaging-and-offline.md)。[GitHub release limits](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)、[repository visibility](https://docs.github.com/en/repositories/creating-and-managing-repositories/about-repositories)

使用者已指定由自己上傳npm。最後交付的是可檢閱、已驗證的TGZ及操作說明，不自動執行`npm publish`；npm scope/name仍待D14。新repo `rustgrok`按已確認的public建立，這不包含啟用Pages或另部署公開網站。GitHub建置／上傳結果與npm發布狀態分開回報。

trusted publishing與provenance若之後採用，需依實際repo與npm能力設定；repo採public不代表npm發布設定已完成，使用者手動發布的安排不變。[npm trusted publishing](https://docs.npmjs.com/trusted-publishers/)

## 10. 初始GitHub交付

public repo `swchen44/rustgrok`已建立，發行包放GitHub Releases；D11已USER_CONFIRMED，不再詢問名稱或可見性，也不再次建立repo。接續交付流程：

1. 核對實際repo metadata與main分支；若已有README、AGENTS或其他內容，先讀取並保留既有工作。
2. 新空repo使用清楚的規劃初始提交；若目的地已有內容，使用獨立規劃分支與PR。
3. 上傳本次Rust v0.6 README、docs、tests規劃與來源研究lock；保存v0.1被取代及後續各輪需求確認的歷史，不把Java包裝方向當現行設計。
4. 將已決定要做的G0/G1工作建立Issues／milestones；OPEN事項是decision，不能標成已批准feature。
5. 核對commit／tree與檔案hash，回報可檢閱的repo／commit／PR連結。

目前`creation_status: created`。已由GitHub頁面與connector metadata確認[swchen44/rustgrok](https://github.com/swchen44/rustgrok)：repo id=`1433282584`、visibility=`public`、default branch=`main`、push／admin權限為true；建立時size=0。規劃內容尚未提交，commit待後續實際回填；目前沒有Release或已發布產品artifact。repo建立成功不能當內容提交、產品建置或Release完成；觀測與交接詳見[16](16-github-bootstrap-and-release-handoff.md)。
