# Rust 版本：風險與返工控制登錄

版本：v0.6，2026-10-25。**既定Rust與功能範圍不變；先Mac M1／arm64完整驗證，再Ubuntu24.04 amd64適配／原生重驗，正式部署仍Ubuntu優先。public新repo rustgrok與GitHub Releases交付已確認；建立狀態另記。沒有風險因需求確認完成而被當作實測已關閉。**

## 1. 狀態與處理規則

- `pending`：風險已識別，尚無足夠的本產品實測或已批准決策可以關閉。
- `blocked`：某項必要驗證因明確缺少決策／環境而無法進行；實際進入此狀態時須記阻擋與owner。
- `mitigated`：已有具體產物、版本及測試證據證明控制有效，仍記錄剩餘限制。
- `accepted`：使用者明確接受列明的剩餘風險／範圍變更，不能由實作者自行標記。
- `superseded`：原風險所依據的產品方案已被取代，保留ID與歷史，並指向仍適用的後續風險。

現行風險全部為 `pending`；R01、R05因Java包裝方案被取代而標為 `superseded`。未執行的測試、僅靜態研究、crosscompile成功或參考產品CI結果都不能將本產品標成mitigated。

D02、D11、D19與D21已確認；D03的M1／arm64先驗與Ubuntu24.04 amd64正式部署、D09約5人也已知，不再作未答阻擋。macOS版本／Windows細節、實際repo／Release交付、工具／平台payload、授權與企業環境，以及SLO／資料規模／硬體／request負載／資源取捨仍須完成；相關風險維持pending，Mac通過不能關閉Ubuntu風險。

v0.3的R11／R12曾以完整登入／ACL矩陣描述風險；依D02在v0.4調整為管理入口暴露與範圍誤解，不再要求使用者角色／RBAC／專案ACL，也不追加帳號／SSO／登入必交。主機CLI／本機控制介面管理、LAN只讀是D16的PROPOSED方案，尚未批准；R11／R12仍為pending，並非安全測試已通過。

原版Java／Tomcat只用作獨立oracle。遇到Rust實作困難時，先取得差異證據、調整相容層／Rust選型或提具體需求決策；不能加入隱性Java產品fallback，也不能默默刪掉原功能。

## 2. 保留的風險ID與Rust方向調整

| ID | 狀態 | 風險／觸發與後果 | 優先驗證／證據 | 若未通過 |
|---|---|---|---|---|
| R01 | superseded | v0.1以隨包JVM為產品核心，可能受公司政策阻擋 | D01已改Rust，舊JVM部署路線不再是現行基線 | 不再要求批准JVM核心；native分發／執行風險轉R37，D19工具隨包策略已確認 |
| R02 | pending | 研究版與使用者實際版本／客製UI不同，做完仍不符合style | G0 D04固定oracle；原資產、DOM、互動、畫面清單 | 基線變更重跑受影響功能／UI，不只換CSS |
| R03 | pending | Windows的paths、xref/history、程序與檔案鎖到後期才發現缺口 | 依Mac先驗／Ubuntu適配順序安排WP-B04及Windows原生驗證；G4完成OS／WIN／RST-006，不用WSL代替 | 修Rust平台層／工具adapter；新順序不取消Windows支援，也不以Mac／Linux結果代替Windows證據 |
| R04 | pending | 分散roots、junction、UNC／NAS與project identity／路徑處理不吻合 | D05、WP-B05；跨磁碟、case、canonicalization與URL全鏈 | 提可驗證mapping／snapshot方案，不能偷複製或搬動使用者source |
| R05 | superseded | v0.1的WAR/JSP/JSTL/container payload缺件風險 | Rust產品不執行WAR/JSP；舊container驗收已失去產品適用性 | Rust動態頁面風險轉R31，native依賴閉包轉R34／R21；oracle依賴另管理 |
| R06 | pending | npm／公司mirror拒絕binary、包大小或install scripts，或Release asset超限造成交付缺件 | D07/D14、WP-B04/WP-E02；實際registry、cold-cache TGZ及GitHub每檔大小／asset數量 | per-tuple包與完整閉包；超限調整獨立source／debug產物並重驗，不省略必要依賴或加外網下載器 |
| R07 | pending | optional平台包被省略／失敗或cache掩蓋缺件，install成功卻不能執行 | missing／omit=optional負測、版本/hash檢查、空cache | launcher執行前fail-fast；不能借未知PATH版本或隱性下載 |
| R08 | pending | Rust index writer、project/job metadata與配置同時寫入，造成競爭或半提交 | WP-B06/WP-C07；同index排他、generation／commit receipt、crash注入 | 修writer與發布協定；不要把上游lock設定當Rust已獲保護 |
| R09 | pending | Windows running EXE、DLL、mmap／抗毒軟體鎖檔，更新／回復失敗 | 原生upgrade／stop／rollback，版本目錄或execution cache實驗 | 停機切換／保留舊版；不強覆寫或刪仍在用的檔案 |
| R10 | pending | 只備index但live source已改，raw/xref/history／rollback版本不一致 | D12、source revision／generation模型、SRC-005/RST-009 | 選經批准snapshot或明示eventual契約，不誇大最後成功版本保護 |
| R11 | pending | 把不用權限管理誤解為所有LAN讀者都可重建／改設定／停機，管理API隨讀者入口暴露 | D16／D20列端點與既有REST契約；對主機CLI／本機控制、LAN只讀提案測實際listener、URL與HTTP methods | 先定管理暴露契約並驗證；未批准提案不當現行授權，不把local／LAN已確認需求重新泛稱blocked |
| R12 | pending | 認證、權限管理與管理入口混淆：額外加入帳號／SSO／RBAC，或把免登入提案當已批准 | D02不做角色／RBAC／project ACL；檢查需求、介面與測試範圍，管理暴露依D16、REST相容依D20 | 移除未要求的登入／角色必交gate，不自行開放管理端點或刪既有API；必要差異具體提出 |
| R13 | pending | oracle、ctags、crates或features漂移，差異來源難以歸因 | 固定原版manifest、Cargo.lock、toolchain／helpers與配對run記錄 | 分離baseline變更與產品回歸；先還原可比較環境 |
| R14 | pending | normalization／畫面遮罩抹去ranking、line、highlight等差異，製造假PASS | WP-A08白名單；保留raw responses／AST／token／score／DOM | 拒絕未批准的重排或擴大遮罩，定位真差異 |
| R15 | pending | 只驗常用語言／SCM，遺漏原版其他功能與long-tail cases | 完整inventory與Req/Test對照；D06優先深測不減範圍 | 需分期則明確提請使用者批准，不宣告全功能同等 |
| R16 | pending | Rust配置、queue或CPU並行無上限，OOM、query饑餓或磁碟耗盡 | bounded pipeline、全process tree RSS／CPU／I/O／磁碟峰值；WP-B06／WP-B07／WP-E07 | 依D09/D21與profiling限制資源；搜尋速度優先不等於資源無上限，不只縮小測試資料 |
| R17 | pending | binary升降版與新index／metadata schema混用，回復後不能讀資料 | schema相容矩陣、upgrade-check、RST-009及migration測試 | 成對回復binary／資料或明確重建；npm downgrade不當資料rollback |
| R18 | pending | 移植OpenGrok後漏notice／source，誤把所有內容標成單一license | WP-E05逐檔provenance、Cargo/native/assets/helper inventory | 阻擋發行並補來源與授權；換語言不等於無衍生義務 |
| R19 | pending | npm／ZIP誤含企業source、index或token | pack allowlist、解包檢查、只用synthetic fixture | 未通過不交付publish；產物重新生成並驗證 |
| R20 | pending | 把已知M1／arm64當作已知macOS版本，或SDK／簽章／quarantine不符，下載後不能啟動 | 記實際／最低macOS、SDK／toolchain與M1原生環境，再完整驗證最終ZIP及簽章後hash | 不由CPU推定OS；修平台矩陣與發行方式，一個Mac tuple更不代表Ubuntu通過，不停用系統保護 |
| R21 | pending | CI預裝Node／Java／Cargo／Ctags／Git／共享庫遮住成品缺依賴 | clean-machine與binary dependency report；主要bundled profile屏蔽系統helpers驗閉包 | 重做offline gate；ZIP要驗無Node／Java／Rust toolchain的適用流程；不把external-only列必交profile |
| R22 | pending | Rust管理API／CLI漏掉原Python tools或automation的行為 | D16/D20；命令、參數、JSON、非同步完成／錯誤契約盤點 | 完成Rust adapter或提出明確遷移，不用新命令名稱假稱相容 |
| R23 | pending | public rustgrok與首次規劃提交已完成，但產品Release上傳／取用未完成，誤把文件交付當產品完成 | 已核對首次規劃[commit 526f619](https://github.com/swchen44/rustgrok/commit/526f619089703f710c915af303e54aea7d25226e)的46個Git blob；產品Release／asset／manifest hash仍待實際驗證 | repo及文件提交部分完成；沒有產品Release或GitHub CI執行，不把風險整體標已解或產品測試標PASS |
| R24 | pending | 搜尋速度與約5人情境已知，但把人數當QPS／request上限，或以Mac結果推估Ubuntu效能／工期 | D09/D12/D21量化契約；Mac首輪完整量測後，在Ubuntu24.04 amd64原生重測，G5用代表資料SLO | 分平台補硬體與request模型，不移植PASS／數值，不添加假SLO或分散式必備 |

## 3. Rust重寫新增風險

| ID | 狀態 | 風險／觸發與後果 | 優先驗證／證據 | 若未通過 |
|---|---|---|---|---|
| R25 | pending | 直接使用候選引擎parser，AND／OR、欄位escapers、wildcard／Lucene regex／fuzzy／錯誤語意失配 | G1 WP-B02、RST-002/007；獨立原版AST與命中／錯誤differential | 修query-compat／typed AST／engine lowering；不能默用lenient parser或縮語法 |
| R26 | pending | analyzer把defs/refs大小寫、長identifier、標點、stopwords／position gaps處理錯，命中與phrase改變 | RST-003/007；逐欄／逐語言token、position、span對照 | 移植或重做analyzer；不能以一般文章tokenizer或少數範例替代 |
| R27 | pending | 同名BM25仍有field statistics／norm／boost／tie差異，跨project合併尤其變排名與分頁 | RST-004；稀疏fields、deleted docs、選定projects與mixed query的score/explain | 自訂scorer／stats或重評Rust引擎；D20未批准前不套任意精度容忍 |
| R28 | pending | Java UTF-16 offsets與Rust UTF-8 bytes／scalar混用，emoji／繁中／CRLF令高亮、行號與span錯位 | RST-003/005；共同座標轉換、原始bytes與line map、組合字／大小寫展開 | 修可驗證的offset／encoding模型；不能用lossy轉換掩蓋來源位置錯誤 |
| R29 | pending | 把syntax tree／identifier集合當原版defs/refs/xref，macro、scope、signature與連結語意失配 | WP-B03／WP-D02／WP-D03、RST-005；C/C++ macros、混合語言與未完成source | 完成language／Ctags interpretation與xref模型；Tree-sitter不是自動語意相容證明 |
| R30 | pending | 原版JGit與Git CLI／git2／gix在rename/follow/merge/blame／whitespace不同，history頁或註解錯誤 | WP-B05／WP-D03、SCM矩陣；固定history fixture比OpenGrok/JGit輸出 | 修Rust history adapter與選項；同一Git版本不視為等價保證；其他SCM仍保留 |
| R31 | pending | 只搬CSS／JS卻未移植JSP動態DOM、contextPath、globals／routes／events，外觀或操作失效 | WP-B03／WP-D02、UI矩陣/RST-005；SSR、URL、headers、快捷鍵、source escaping | 修templates／view model／HTTP契約；不能新增另一套首頁或只做相似風格 |
| R32 | pending | 假定XML、Lucene index或Java JAR可直接用於Rust，或悄悄取消／fallback | G0 D10/D16/D20、RST-011；按層列import／重建／功能移植需求 | 提converter／port／明確bridge及成本給使用者；不動舊index、不加隱性Java執行路徑 |
| R33 | pending | 以Rust語言、通用引擎benchmark或功能縮減宣稱更快；JVMwarmup／cache不公平 | WP-B07／WP-E07、RST-008/010/012；同功能／硬體／工作量，release與公平warmup、p95/p99／RSS／I/O | 依D21優先改善搜尋速度並呈現資源成本；不放寬排序／功能，不自動改回Java或挑最好平均值宣稱完成 |
| R34 | pending | native crates／FFI／grammar／helpers帶入未列DLL/.so／ABI／unsafe或授權依賴；單binary被誤稱無依賴 | G1 binary inventory；Cargo features、PE/ELF/Mach-O linkage、fuzz與source/SBOM | 修依賴閉包、FFI界面／選型與授權；Rust memory safety不代替C／FFI驗證 |
| R35 | pending | CPU工作阻塞async executor，或把spawn_blocking abort當成索引已停止，留下writer／資料不一致 | WP-B06、RST-008/009；受限worker、合作取消檢查點、hard-kill與復原 | 修取消／drain／flush協定；必要時隔離worker，但須測IPC及child lifetime |
| R36 | pending | oracle與Rust共用parser／normalizer／資料產生錯誤，兩邊一起錯卻differential通過 | REF/RST-001；獨立Java診斷讀取器、手工審查反例、raw evidence | 移除共用錯誤來源，補獨立oracle／metamorphic或property證據 |
| R37 | pending | D19已允許必要helpers隨包，但企業環境的native執行、簽章／掃毒、FFI或registry限制仍可能阻擋實際產物 | D07環境與D19工具inventory；逐平台payload、內含庫、依賴／授權及實際安裝執行證據 | 依已確認bundled策略修交付；若遇具體政策衝突再呈報影響，不重問策略、不繞過限制、不私自縮功能 |

Rustquery／analyzer／scorer、JGit與SSR的具體來源和已讀差異見[12相容性研究](12-rust-compatibility.md)。上述是待驗證風險；不能從靜態來源差異推論每個query或所有平台必定失敗。

## 4. 優先阻擋返工的證據順序

1. **G0先固定剩餘邊界：** D04原版基線、D20相容層次、D03的macOS版本／Windows細節及D16管理暴露。M1／arm64、Ubuntu24.04 amd64、Mac先驗順序、public rustgrok／GitHub Releases與既定Rust／功能方向已確認，不反覆詢問；遠端建立結果另外驗證。
2. **G1先驗最難語意：** R25／R26／R27的parser、tokens、scorer與跨project統計，搭配R28／R29／R31的Unicode、xref與原style頁面。選庫之前取得能定位差異的證據。
3. **同時驗原生分發與工作生命週期：** R03／R06／R07／R20／R21／R34／R35，先Mac M1／arm64完整適用驗證，再Ubuntu24.04 amd64實作／適配及原生重驗；Windows原生證據與G4全平台承諾不變，不共用跨平台PASS或把單一OS版本實測當全部平台證據。
4. **依搜尋速度優先提早量測但不超額宣稱：** R16／R24／R33在約5人使用情境對等價slice量測，記錄實際request分布、CPU／RAM／磁碟與索引成本；數值SLO及取捨待確認，完整語言、SCM、插件／資料相容與代表工作量仍在G3/G5驗收。
5. **G4/G6只接受實際發行包證據：** 最終binary與archive hash、原生平台、offline、crash／upgrade、licenses／source相符；不挪用舊版報告。

正式測試IDs、gate與需求對照以[05驗證計畫](05-verification-plan.md)及tests/matrix.json為準；本表不另計一套測試完成率。當某風險被具體阻擋時才把pending改為blocked，並列缺少的決策／環境，不先把所有風險泛稱blocked。

## 5. 相容與升級策略的持續風險

Rust重寫後，OpenGrok仍作功能／style參考基線；OpenGrok升級、Rust toolchain／crates、helpers／grammar、OS或browser更新，都可能改語意、畫面、ABI、性能或索引格式。更新須用獨立compatibility PR：列來源與授權變化、重建oracle／candidate、重跑受影響parity／native／offline／upgrade測項，不能只更新lockfile與screenshots便算完成。

候選Tantivy目前有自身index格式與版本；是否採用仍須G1。產品metadata／index migration要有明確版本，不以「都是倒排索引」當Lucene或跨版本可讀證據。重新索引需求與可接受停機時間由D10/D12/D20決定。

沒有批准自動連外更新。內網runtime不主動下載新binary、crate、資產或helpers；依D18走人工核准與可查核的發行更新。

## 6. 結案準則

風險不能以「應該可以」、「Rust比較安全／快」、「upstream CI綠燈」或文件有列測試就結案。轉mitigated至少附：

- 具體產品commit、Cargo／native依賴manifest、平台與artifact hash。
- 可重現步驟、對應需求／test IDs、raw結果與摘要。
- 原版oracle版本與必要環境控制；相容差異逐項結論。
- 剩餘限制、適用profile及失效條件。
- 若轉accepted，使用者對具體剩餘風險的明確批准與日期。

`superseded`只表示該舊方案不再適用，不是已通過產品測試。現在全部Rust功能、離線與性能驗收仍為NOT_RUN；本表只完成風險規劃。

## 7. 關鍵一級來源

- [OpenGrok GitRepository／JGit](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java)：history／blame對照基準。
- [Rust linkage](https://doc.rust-lang.org/reference/linkage.html)：CRT與實際binary linkage核對。
- [Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)：CPU併行與已開始工作不能以abort直接取消。
- [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)：native程式碼／link與build依賴。
- [npm package.json](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/)：optional dependencies與發行入口。
- [GitHub about releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)、[repository visibility](https://docs.github.com/en/repositories/creating-and-managing-repositories/about-repositories)：Release資產限制、取用與repo可見性；具體契約見[06分發設計](06-packaging-and-offline.md)。
- Parser／analyzer／BM25／assets的逐項pinned來源見[12相容性研究](12-rust-compatibility.md)與[sources](sources.md)。
