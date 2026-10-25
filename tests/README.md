# tests — Rust重寫驗證規格 v0.6

**狀態：proposed。80項全為planned；尚無產品測試程式、Rust原型量測或實測驗收結果。** 本版沿用原68個ID及既有RST-001～RST-012，沒有新增或刪除測項。

## 工作起點

1. 讀[驗證計畫](../docs/05-verification-plan.md)與[實作計畫](../docs/04-implementation-plan.md)。
2. 讀[matrix.json](matrix.json)的steps/expected/req_ids；22 FR與16 NFR已建立規格追蹤。
3. Rust、bundled helpers、搜尋優先、local＋LAN、約5人及不做權限管理已確認。首輪已選Apple M1／Apple Silicon arm64，先在macOS原生arm64完整移植／功能／介面／包裝完成，再於Ubuntu24.04 amd64適配並完整原生重驗；正式部署優先是Ubuntu24.04 amd64。macOS實際／最低版本及其餘平台環境、D16/D20、具體SLO與負載仍OPEN。
4. 先做獨立OpenGrok oracle與最難語意／公平性能spikes，再逐層實作Rust parser/analyzer/index/scorer/snippet/xref/SSR。
5. 實跑結果另存run manifest；不能把本plan改passed充當證據。

## JSON欄位

| 欄位 | 意義 |
|---|---|
| test_id | 穩定ID；沿用舊68與既有12個RST |
| req_ids | FR-001～022／NFR-001～016 |
| preconditions / steps / expected | 可據以實作harness的前置、操作、預期；待決部分blocked |
| evidence | 未來相對路徑範本，現在沒有實測檔 |
| gate | 最早需要實驗證據的關卡 |
| acceptance_gate | 該項完整驗收關卡；早期量測不等於SLO通過 |
| release_gate | 全部G6，交接前核對final artifact與證據 |
| status | 本版全planned |
| platform_scope | 原生目標平台；原版oracle不一定與Rust用同runtime |
| requires_user_decision / conditional | 未決與條件性適用標記，不代表默認排除功能 |

## Gate

G0需求基線；G1獨立oracle＋最難語意與公平效能可行性；G2Rust垂直流程；G3全parity；G4三平台打包／離線／故障／安全可靠性；G5使用者內網代表資料＋批准SLO的pilot；G6GitHub/npm發布交接。G4不代表使用者已驗收。Mac先完成是平台執行順序，不改各Test ID的gate映射；跨平台gate仍要等各承諾平台證據齊全。

## 原版與Rust的責任

- 所有現有產品測項適用標準Rust profile：Java/Tomcat/OpenGrok只用於獨立oracle，產品不包含或依賴JVM、不執行WAR/JSP，也不呼叫Java完成核心功能。
- 兩種運作模式都要驗：local本機與LAN共享。帳號、角色、RBAC、project ACL不列mandatory；SEC-002改驗所有讀者共同讀取已登記projects，不skip算passed，也不新增強制登入／SSO。
- 免登入讀者入口、主機CLI／本機控制面管理及LAN讀者面只讀均為工程提案；管理端點、token／CSRF等依D16/D20確認，不能寫成已批准或默認LAN可管理。
- 所有主要runtime測試採bundled helpers發行物，不要求使用者外裝ctags／SCM工具；B外裝與C無helpers不再是mandatory profile。缺helper／錯版本／不相容binary仍作故障負測，不得fallback到系統PATH或外網下載。
- 本輪沒有批准Java plugin bridge。只有未來D20明確批准特定插件bridge、另立profile／測項／gates後才可驗證該額外交付；不得取代Rust核心或放寬本版標準profile的無JVM要求。
- 首輪Mac已確認Apple M1／Apple Silicon arm64，原生完成全部移植／功能／介面／包裝；只剩macOS實際／最低版本待確認，不再問Intel或ARM。須記錄binary／程序架構與翻譯狀態，不以Rosetta x86_64當arm64 PASS；之後Ubuntu24.04 amd64平台適配並完整原生重驗，不能只跑smoke或複製Mac PASS。跨編譯只產生檔案，不是runtime證據。Windows 64位元最終原生驗收保留，CPU／版本不能默猜。
- npm安裝須具備核准Node/npm；JS launcher的執行前提另按批准形式驗證。native ZIP直接執行Rust binary，必須另在沒有Node/npm的乾淨環境驗證；兩者都不需要Java或Rust工具鏈。
- 原CSS/JS/images/fonts可依授權沿用；Rust SSR比對rendered DOM/URL/keyboard/hover及同平台screenshots。
- 不同索引格式不比binary相等；比較tokens/positions/hits/counts/order/snippets/xref/history與完整功能coverage。
- 正確性與性能報告分開，性能優先驗搜尋速度。REF合理調校JVM並記錄warmup，Rust用可發布release build與實際bundled helpers；同硬體、等價資料與功能，仍完整記錄全process成本。
- 沒有測量就不能說Rust更快、更省或達成某倍數。較少功能的slice不能外推成全產品性能。

## 搜尋優先的既有測項細化

PERF-001/002、PRJ-002及RST-008/010/012沿用原ID。按全文／defs／refs／path／Boolean／phrase／regex等完整承諾語意、單／跨project、命中量、cold/warm、cache命中率、無背景索引／並行完整或增量索引分報。記錄server排隊與處理階段，以及browser提交至結果可見／完成互動的時間，不能把其中一種冒充另一種。

加上1人參照與約5人互動情境：不同client採不同project/query組合與獨立瀏覽狀態，共用server cache，並加入背景index。約5人不是5 QPS、5個請求並行或硬性人數上限；另記實際request率、in-flight、think-time與client數。local/LAN分報；Mac先量開發結果，移植／介面／包裝完成後在Ubuntu24.04 amd64重驗。正式部署pilot優先Ubuntu24.04 amd64，各平台用自己的原版公平對照，不能移用Mac性能結果。

## 發行資產驗收

公開repo [swchen44/rustgrok](https://github.com/swchen44/rustgrok) 已建立，ID `1433282584`、default branch `main`。初始完整規劃已提交並核實commit `526f619089703f710c915af303e54aea7d25226e`、tree `3af8b051938022e067b29a8ccc4d7acac51205b2`；46個檔案blob SHA全部與當時本地規劃快照匹配。這只是來源／上傳完整性核對，不是產品PASS：產品Release尚無、CI未跑、80項測試仍planned。後續收據更新不預填未知SHA；產品發行包仍放GitHub Releases。REL-001/002先驗候選檔及文件，REL-005回讀實際Release的tag/commit、asset ID／名稱／大小／類型、OS/arch、版本與checksum，下載後核對bytes及各平台原生結果。

PKG-001/003使用同一版實際TGZ／ZIP驗安裝及完整離線閉包；每平台都要有自己的run，不以跨編譯或Mac PASS代替Linux／Windows。trace可放CI artifacts，但正式發行包必須對應GitHub Releases資產；public與repo建立已確認；正式Release發布後再驗公開下載，不虛構目前已有產品binary。

每組保留樣本數、p50/p95/p99、throughput、timeouts／錯誤／取消、慢查詢明細；cache無法觀測標unknown，不能推定全命中。成功請求分位數與timeout另列，不讓快查詢平均值掩蓋慢查詢、未完成請求或缺功能。查詢比例、具體SLO、corpus、hardware、request率／think-time／每人並行仍待批准；G1的研究配置不等於使用者內網承諾。

## 規劃目錄

| 位置 | 用途 |
|---|---|
| tests/reference/ | 原版部署、pinned manifests與獨立診斷讀取器 |
| tests/fixtures/golden/、tests/fixtures/queries/ | 合成source/SCM/query、固定seed與hash |
| tests/contract/、tests/parity/ | HTTP/CLI/config契約及分層功能差異 |
| tests/visual/ | 原版資產、rendered DOM、style與互動 |
| tests/install/、tests/offline/ | packed npm/ZIP安裝及完整離線依賴；分開Node前提 |
| tests/platform/、tests/recovery/ | 原生OS與fault injection |
| tests/performance/ | REF/Rust端到端HTTP、index jobs與全process資源量測 |
| tests/fuzz/ | query/analyzer/path property與differential fuzz；最小失敗fixtures |
| tests/evidence/ | 證據schema與索引，大型raw evidence保存在run artifacts |
| Rust crates的benches/ | 局部Criterion微基準，不能單獨代表產品加速 |

上述為monorepo根目錄的相對位置，與[Monorepo規劃](../docs/07-monorepo-and-github.md)一致；未存在目錄不代表已有測試。matrix的`evidence/<release>/<run>/...`另以run artifact根目錄解析。企業資料只留內網；GitHub只放合法公開／合成fixture與批准的匿名結果。

## 執行結果必備資料

git/upstream SHA、Cargo lock/toolchain/build profile/target/allocator、REF JVM/Tomcat與參數、bundled helpers清單/版本/hash、artifact/corpus/config hashes、精確OS/CPU/硬體/磁碟與平台階段、local/LAN拓撲、1人／約5人情境與實際client數/request率/in-flight/think-time、query類別/project範圍、共用cache hits/misses/warmup/到達模型、並行index狀態、server queue/階段與browser計時邊界、timeouts/慢查詢、test結果、CPU/alloc/I/O與整體process-tree資源、raw evidence及批准例外。發布另記rustgrok實際Release/tag/commit、asset metadata與下載hash；計時與profile runs分開。

此版不提供尚未存在的npm test/cargo test產品腳本，也不宣稱CI綠燈。
