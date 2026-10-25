# 決策登錄與需求訪談

版本：v0.6，2026-10-25。**D01／D02／D19／D21的既定方向不變。D03已確認Mac M1／arm64先完整開發驗證，再Ubuntu24.04 amd64完整原生重驗，正式部署仍Ubuntu優先；D11已確認public新repo rustgrok與GitHub Releases交付。macOS版本、Windows細節、npm名稱、授權與其餘契約仍OPEN；repo實際建立狀態另記。**

## 1. 狀態與決策歷史

- **USER_CONFIRMED**：使用者已明確選定，後續設計依此推進。
- **OPEN**：缺少會影響實作或驗收的資訊，不以預設值冒充使用者選擇。
- **SUPERSEDED**：舊決策或問題已被新方向取代，保存歷史，但不再當現行前提。

| ID | 狀態 | 已確認內容 | 適用範圍與影響 |
|---|---|---|---|
| D01 | USER_CONFIRMED | 採 Rust 重寫產品核心；保留自帶Web Server、npm、內網、多專案、macOS／Ubuntu24.04 amd64／Windows64位元、原版功能與style目標 | 現行v0.6設計；原版Java／Tomcat改作獨立reference oracle；精確平台見D03，HTTP框架、搜尋引擎與工具版本仍需驗證 |
| D02 | USER_CONFIRMED | 同一產品支援local與LAN共用，約5人同時使用，不做使用者角色／RBAC／專案ACL管理 | 約5人是容量情境，不是QPS或同時requests硬值；不追加帳號／SSO／登入為必交，管理操作暴露仍依D16提案與D20相容盤點 |
| D11 | USER_CONFIRMED | 建立public新repo swchen44/rustgrok，release packages放GitHub Releases | repo已實際建立，creation_status為created；規劃內容commit尚待提交，Release／artifact未建立，分階段以metadata與hash驗證 |
| D19 | USER_CONFIRMED | Rust核心可隨包附必要Ctags、Git及其他SCM工具；主要產品profile採bundled helpers | 工具清單、固定版本、各OS／CPU payload、能力、授權與所需SCM仍依inventory驗證；不把外裝或禁止helpers列為必交profile，不授權自動下載或git pull |
| D21 | USER_CONFIRMED | 效能優先方向為搜尋速度 | 約5人情境已知；數值SLO、資料規模／硬體／request負載、CPU／RAM／磁碟預算與取捨仍OPEN，不代表接受排序／功能差異或無限資源 |

確認來源為本次對話，日期2026-10-25；上表是決定摘要。v0.1以「能否隨npm附JVM」為核心的選型問題已 **SUPERSEDED**。原先混在該問題中的Ctags／Git等工具政策移至D19，Node/npm環境限制仍由D07管理。

v0.2時D19工具政策與D21效能優先順序均為OPEN。使用者回覆「A+搜尋速度」，A對應Rust核心隨包附必要工具，因此兩項策略於v0.3轉為USER_CONFIRMED。精確工具產物與數值效能契約並未隨此回覆自動確定；不再重問是否選A／B／C或搜尋速度是否優先。

v0.3時D02使用模式與權限需求仍為OPEN。使用者本輪回覆「3, 約5人， 不用權限管理」，其中3對應local與LAN兩者，因此D02於v0.4轉為USER_CONFIRMED，並將約5人記入D09容量情境。此回答來自同時搜尋人數問題，不轉寫為QPS、HTTP request並行值或產品固定連線上限。

先前「24.02」的版本歧義，已由使用者回覆「24.04」解決。

v0.4曾以Linux開發／首輪驗證先行。v0.5確認Ubuntu24.04的CPU為amd64，並改為先macOS完整驗證，再Ubuntu實作／適配與原生重驗；舊執行順序已SUPERSEDED，正式部署仍Ubuntu優先，Mac PASS不能當Linux PASS。當時Mac CPU尚未指定，已由v0.6補齊；G4仍覆蓋全部承諾平台。[Ubuntu releases](https://releases.ubuntu.com/)、[Ubuntu release cycle](https://ubuntu.com/about/release-cycle)

v0.5確認新repo名`rustgrok`及GitHub Releases用途；帳號`swchen44`已查核，建立表單備妥、名稱可用，當時public/private尚未回答。repo名稱不自動指定npm scope/name。

v0.6使用者回覆「1. public 2. M1」：D11的public已USER_CONFIRMED；D03的Mac CPU為M1／Apple Silicon arm64。`aarch64-apple-darwin`僅是對應target映射，不能由M1推定實際／最低macOS或toolchain；這些仍待D03／G1。

同輪已實際建立[swchen44/rustgrok](https://github.com/swchen44/rustgrok)，`creation_status: created`；GitHub頁面與connector確認repo id=`1433282584`、visibility=`public`、default branch=`main`、push／admin權限true，建立時size=0。規劃內容commit尚待提交，目前無Release；不把repo建立當成產品或發行已完成。

選Rust不代表已選定Tantivy、Tree-sitter、axum或Tokio，也不代表已接受查詢、排名或插件差異。若某個Rust候選庫不符契約，先調整相容層或重新評估Rust技術方案，不自動改回Java產品。

## 2. 接下來優先釐清的事項

D02、D11、D19與D21已確認；D03／D09已有Mac M1／arm64、Ubuntu24.04 amd64、驗證順序及約5人情境。接著完成macOS版本／Windows細節、工具inventory及量化契約；D04、D20仍影響原版oracle與相容層。可以分輪回答；不再詢問Mac CPU、Ubuntu版本／CPU、repo名稱／公開性、local／LAN或權限管理。

| ID | 狀態 | 要回答的問題 | 選項／所需資訊 | 影響與誰決定 |
|---|---|---|---|---|
| D04 | OPEN | 原版功能與style以目前哪個版本／客製版，或研究版1.14.20為基準？ | 版本、可分享畫面、客製CSS／首頁／插件摘要；研究版不自動成為正式基線 | oracle、資產來源、UI與全部相容測試；使用者決定 |
| D20 | OPEN | 「完全相同」在查詢、排序、API、資料格式與插件各層要求什麼相容程度？ | 全部原功能／style要求繼續；將既有XML、Lucene index、cache、JAR plugins、API與score/tie精度分項確認 | 是否需移植、converter、明確bridge或可批准差異；使用者決定，未決不自動放寬 |

### 2.1 已確認的使用模式與管理邊界

D02排除的是使用者角色／RBAC／專案ACL管理，不把這項選擇自動擴成帳號、SSO或登入系統需求。免登入讀者入口為PROPOSED方案，不另設必答的登入確認關卡。

管理操作暴露仍未定。D16的具體PROPOSED方案是「主機CLI或本機控制介面管理，LAN只提供搜尋／瀏覽等讀取操作」；這不是使用者已批准的部署規則。沒有權限管理不能推導為任何LAN人員都可重建、改設定或停機；若需既有REST管理相容，依D16／D20盤點端點與automation後處理，不默刪原功能。

## 3. D19：隨包策略已確認，具體工具仍須驗證

Rust核心可隨包附必要Ctags、Git及其他SCM executables，主要產品profile採bundled helpers。這項政策已確認；尚待完成的是實際功能／SCM清單、版本、各平台payload、依賴與授權驗證。native library／FFI與外部executable仍是不同的交付契約，不能把工具隨包許可當作所有候選庫均已選定。

| 對象／層次 | 已確認範圍／剩餘事項 | 實作與驗證要求 |
|---|---|---|
| Universal Ctags executable | 必要Ctags可隨包；固定build、features及各平台依賴尚待驗證 | adapter與原版語言fixture對照；記hash、能力、DLL／資源與授權 |
| Git與其他SCM executables | 必要工具可隨包；支援SCM inventory、版本與輸出契約待完成 | 不以先做Git prototype推論最終Git-only；不自動git pull/reset，更新行為仍由D08決定 |
| C/C++ native libraries／FFI | 候選庫未因此選定；盤點實際所需library與D07環境限制 | Tree-sitter／libgit2等僅列候選，驗證ABI／授權／unsafe邊界 |
| 工具版本與能力 | 每個產品release固定可查核的工具版本與各OS／CPU payload | 版本、來源、hash、能力及輸出對照；不默默掃PATH挑版本 |
| 缺少工具時 | 精確診斷與各命令失敗呈現需實作；主要隨包profile不得缺件仍宣稱完整 | 缺功能不能算完整同等PASS；不隱性連外下載補件 |

external-only或完全禁止helpers不是本輪必交profile；未來若新增需求，再另立決策、成本與測試。D19也不重新允許Java作產品核心，不授權自動網路下載或source同步。主要bundled profile與clean-machine驗證見[06分發設計](06-packaging-and-offline.md)。

## 4. D20：相容層次與尚未批准的差異

以下拆項是為了讓需求具體可驗證，**不是宣告某些原版能力已被排除**。D02已明確不做使用者角色／RBAC／專案ACL管理，其餘仍以原版全部功能與style為要求；沒有使用者明確批准，不採較寬鬆的命中、排名、API或UI標準。

| 相容維度 | 需要確認的具體內容 | 未決前禁止的假設 |
|---|---|---|
| Query語法與語意 | 欄位、operators、escaping、wildcard／regex／fuzzy、phrase、錯誤處理 | 不能直接用候選引擎parser或lenient修復取代 |
| Hits／排序／score | 命中集合、total count、relevance／field sort、數值score精度、tie與分頁 | 不能任意排序結果後比較、套未批准epsilon或忽略排名 |
| Web／URL／REST／既有automation | 原網頁、快捷鍵、原URL、參數、headers、JSON、錯誤與非同步完成語意 | 不能換成新API就算同功能，不能只保留CSS |
| JavaBeans XML configuration | 是否要匯入／匯出既有檔案，實際使用哪些property／自訂內容？ | 不能把普通XML parse當作任意Java物件配置可執行；不執行未受信任XMLDecoder輸入 |
| Lucene index | 是否必須直接讀現有index，還是可從source完整重建？ | 不能宣稱Tantivy或其他Rust引擎直接相容Lucene格式；不動原索引作試驗 |
| xref／history／其他cache | 是否需沿用既有cache，還是可重建？ | 不能只改副檔名／路徑就當作格式相容 |
| Java JAR plugins | 有哪些自訂plugin；要求原JAR binary沿用，還是移植等價能力？ | Rust不能直接提供原Java類別載入；不能默刪插件，也不能加隱性JVM fallback |
| 例外與差異接受 | 若實測差異不可避免，是否接受某個明確、有限的改變？ | OPEN不是已接受差異；任何變更需記原行為、候選行為、影響與使用者確認 |

必要時可以提出converter、功能移植或獨立compatibility bridge，但必須列出部署／授權／維護成本並由使用者決定。標準Rust產品目前不含Java執行前提；特定bridge也不能悄悄使已選Rust核心退回Java。

D10負責盤點「現有哪些資料要搬」，D16盤點「哪些現有管理／整合介面在用」，D20確認「需要哪一層相容與是否接受具體差異」。三者互相關聯，但不能互相代答。

## 5. D21：效能優先順序與量化契約

搜尋速度優先已由使用者確認。後續先釐清代表搜尋工作量與延遲／吞吐量測，再協議可驗收的數值及資源成本；不再把「優先優化什麼」當作未答問題。

| 項目 | 需使用者確認／G1提供的資訊 | 目前狀態 |
|---|---|---|
| 優先順序 | 搜尋速度優先；其餘指標仍需量測，不能忽略功能、穩定性與資源成本 | USER_CONFIRMED |
| 代表工作量 | 約5人同時使用已知；專案／files／LOC／GB／history深度、語言比例、查詢與更新分布待補 | 部分已知，其餘OPEN，與D09共用資料；人數不是request硬值 |
| 指標與統計口徑 | cold／warm、p50/p95/p99、throughput、error／timeout、總RSS與磁碟峰值 | OPEN，先提出可核實測量再凍結 |
| 絕對門檻／改善目標 | 哪些必達、哪些可接受退步；是否相對原版比較、容許多少成本 | OPEN，不填未批准的速度倍數或容量 |
| 資源預算 | 約5位使用者情境已知；CPU／RAM／SSD或NAS、index concurrency、request負載與背景工作待補 | 部分已知，其餘OPEN |
| 可接受tradeoff | 例如以較大index、更多RSS／CPU改善查詢，或增加索引時間是否可接受 | OPEN，搜尋速度優先不等於這些成本已獲批准 |

Rust採用本身不是效能驗收。G1先在功能等價的slice量測並標示尚未移植範圍；正式SLO在相同功能／資料／硬體條件下驗證。若收益不足，報告parser／analyzer／SCM／I/O／engine／HTTP等瓶頸，優先改善搜尋速度並呈現成本；不自動撤回Rust選擇。搜尋速度優先不代表接受排名／score、命中、完整功能或style差異，也不授權無限CPU／RAM／磁碟。

## 6. 其餘環境、功能與容量決策

| ID | 狀態 | 問題 | 選項／所需資訊與影響 |
|---|---|---|---|
| D03 | OPEN | Mac M1／arm64與Ubuntu24.04 amd64已確認；macOS版本／Windows／browser細節？ | 先M1 Mac完整開發驗證，再Ubuntu適配與完整原生重驗，正式部署Ubuntu優先；Mac實際／最低OS、SDK／toolchain、Windows版本／CPU與browser仍待定。Mac PASS不代Linux PASS，G4承諾不變 |
| D05 | OPEN | 多專案位於哪裡？「同時」要哪些操作？ | 同一root／分散磁碟／NAS／UNC；同時可查、跨project查、同時重建分開；project數。影響mapping、OS檔案存取權限、排程與scoring scope，不新增project ACL |
| D06 | OPEN | 語言、SCM與原版擴充清單？ | 實際語言、Git／SVN／Perforce／Mercurial等、history／annotate／plugins使用。用於inventory、優先測試與adapter，不能當成只做常用子集的授權 |
| D07 | OPEN | npm與內網限制？ | public npm／公司mirror／私有registry／只匯入TGZ／完全離線；Node/npm版本、install scripts、一般使用者權限、binary與大小policy；是否需要離線build。影響包型與clean-machine測試 |
| D08 | OPEN | source如何更新與何時索引？ | 外部更新＋手動／定時／watch；是否允許工具fetch/pull；dirty files處理。影響排程、source一致性與操作授權 |
| D09 | OPEN | 約5人同時使用已知；其餘資料規模、硬體與效能基線？ | 人數為已確認容量情境；projects/repos/files/GB/LOC/history、CPU/RAM/SSD/NAS、query分布／request並行與數值SLO仍OPEN。不能以約5人代填5 QPS或5個同時requests，D21搜尋速度優先不變 |
| D10 | OPEN | 要遷移哪些現有資料與URL？ | sources、XML設定、Lucene index、history/xref cache、bookmarks/context path。需與D20共同選擇匯入、converter、重建與rollback策略 |

「尚未提供語言／SCM／plugins清單」不等於不需要。若要分期或以常用子集先發布，須明確提出差異與影響，取得使用者批准；prototype scope本身不會縮小最終需求。

## 7. 運維與發行決策

| ID | 狀態 | 問題 | 選項／決策要點與影響 |
|---|---|---|---|
| D12 | OPEN | 索引更新／升級能否中斷？需要何種一致性與回復？ | eventual／停機／shadow generation／source snapshot；RTO/RPO與磁碟預算。Rust index commit不等於index/xref/history/config全體原子提交 |
| D13 | OPEN | 原生離線ZIP的精確內容、啟動與發行方式？ | 原生binary、依D19隨包策略完成inventory的helpers／共享庫／資產、CLI或雙擊入口、mac簽章/notarization及企業驗證要求。ZIP可直接跑Rust，不預設需要打包Node |
| D14 | OPEN | npm包型、名稱與發布體驗？ | 通用薄JSlauncher＋平台包／各平台native-bin package／offline TGZ；scope/name、registry限制、精確支援npm版本。發布仍由使用者自己執行 |
| D15 | OPEN | 常駐方式？ | 終端前景／背景user process／登入自啟／OS system service；是否自動開browser。影響Rust supervisor與安裝權限 |
| D16 | OPEN | 管理操作如何暴露，有哪些既有管理／整合介面要接續？ | PROPOSED：主機CLI／本機控制介面管理、LAN只讀，尚未批准。盤點REST consumers、原Python CLI／scripts、plugins與automation的命令／格式／完成語意，依D20保留相容，不默刪或默換介面 |
| D17 | OPEN | 產品名稱、新程式碼授權與來源交付方式？ | 名稱不冒充官方；新Rust／JS內容、移植檔及原資產逐件處理；repo/source archives政策。改寫Rust不自動將衍生內容變成MIT |
| D18 | OPEN | 上游相容、安全更新與產品支援如何維護？ | OpenGrok相容基線、Rust toolchain／crates、helpers、browser與native依賴的人工review頻率、支援週期、企業導入節奏；不在內網runtime自動連外更新 |

## 8. 決策確認與變更紀錄

每個決策至少記錄：問題、候選選項、使用者實際回覆或忠實摘要、選定值、確認日期、適用版本、受影響需求／設計／工作包／tests，以及替代方案未選原因。若沒有逐字記錄，標示摘要，不編造引文。

變更已確認決策時保留前值與取代原因；不得把v0.1 Java包裝方向無痕改成彷彿從未討論過，也不得讓舊方向留在現行gate中繼續約束Rust實作。

進入G0/G1所需決策依04／05判定。依已確認的Rust、local／LAN約5人無權限管理、bundled helpers、搜尋速度、M1 Mac先驗／Ubuntu24.04 amd64部署與public rustgrok交付方向推進；不以已確認事項再次阻擋。若某項實作仍缺工具驗證、macOS版本／Windows細節、相容決策、SLO或D16管理暴露決策，標明實際阻擋與owner；遠端建立與測試另以證據判定，不誤標PASS或擅自填答案。
