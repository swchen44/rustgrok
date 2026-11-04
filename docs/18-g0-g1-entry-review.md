# G0／G1 執行基線確認

日期：2026-10-26；執行更新：2026-11-01。狀態：**使用者以「status and continue」要求繼續，依既有基線啟動不受OPEN項阻擋的G0工作；不是G0／G1已通過，也不把未答事項視為已選。** 已選事項見[決策表](08-decision-register.md)，完整44個工作包與依賴見[實作計畫](04-implementation-plan.md)。本頁不新增工作包、需求或測試ID。

## 已確認的實驗邊界

- Rustgrok以Rust重寫核心，保留OpenGrok 1.14.20的原版功能與style；D02排除的角色／project ACL之外，不預先接受其他差異。Tantivy、Axum／Tokio、Tree-sitter及儲存等仍是G1候選。
- 約5人，10個project，每project source約50 MB；project可多repo或git-in-git，Git history／blame／diff及其他原版SCM能力保留。C、Python、JavaScript優先加深驗證，其他語言不刪除。
- 手動／排程索引，source外部更新；重建期間搜尋上一成功版本，完整就緒後整批發布。snippet／xref／raw／download依原版live／SCM分支，不加入閱讀快照。
- LAN搜尋／瀏覽；管理網頁只限主機本機瀏覽器，保留主機CLI／本機控制面。首版CLI前景／背景；watch與service／登入自啟未選。
- 先M1 arm64 Mac開發及完整適用驗證，再Ubuntu24.04 amd64適配與完整原生重驗，Linux為主要部署；Windows11 x64原生支援保留。Mac觀測15.7.7／8 GiB不代表最低OS或產品PASS。
- registry／公司mirror及搬入tgz／ZIP兩種安裝路徑都要；主npm套件rustgrok，使用者自行發布。新原創Rust／JS的MIT與沿用／移植／第三方條件按[LICENSE.md](../LICENSE.md)分開追蹤。

## 開始後先完成的既有工作

| 既有WP | 下一步可審查產物 | 驗證邊界 |
|---|---|---|
| WP-A01／A02／A03 | 已選／OPEN清單、host觀測、平台與代表工作量契約 | 已有[開發host觀測](../tests/evidence/development-host-observation-2026-10-26.json)；最低OS、Linux硬體等不代填，不能將WP整體提前標完成 |
| WP-A04／A05 | 固定版本原版source／release及hash、來源／license台帳、routes／query／analyzer／SCM inventory | 2026-11-01已驗OpenGrok source／release與selected source assets；完整JDK、逐檔license及其餘功能／runtime持續盤點，見19執行紀錄 |
| WP-A06 | 獨立Java/OpenGrok oracle與配置、啟動／查詢／history原始證據 | 目前未有可核實Java21 oracle；取得所需固定工具至user home或允許位置、驗來源與hash，不能把Java帶進產品runtime |
| WP-A07 | 公開deterministic corpus與Git history，含C／Python／JS、多repo／nested、Unicode等 | 合成資料不含企業source；精確bytes、repo數、seed及實際hash另記，不把約500 MB估計當測量結果 |
| WP-A08 | tokens／query／ranking／DOM等分層差異harness與有限normalization | 禁止任意sort、epsilon或遮罩吞掉相容差異；未有oracle輸出前不建立假golden |
| WP-B01–B08 | Mac上最小完整slice、語意／UI／分發／generation與公平效能spikes | 依原WP依賴先後；release build、同資料／功能／硬體、公平暖機。subset證據不外推全產品PASS；G1後才凍結技術棧及大量模組實作 |

## 尚未定案與影響

| 待決項 | 本階段處理方式／owner | 阻擋範圍 |
|---|---|---|
| 最低macOS、browser矩陣、正式toolchain／ABI、精確helpers | agent以G1 source／build／native證據提出，必要產品取捨再問使用者 | 支援範圍／G4與正式artifact凍結；不冒稱目前host測過所有OS |
| Ubuntu CPU／RAM／SSD或NAS、history大小、query mix、SLO／資源預算 | 使用者資料未提供時維持OPEN；agent先記研究設定與實測成本，再提出量化方案 | 正式效能／容量與G5驗收；G1可做明示配置的開發研究，不報SLO PASS |
| 建置期間來源變動、generation／journal等機制；同次工作涵蓋的所有project須一起發布已確認 | agent先凍結可測契約並以G1驗證，不能違反D12／Q9 | 正式索引發布與恢復可靠性；不因engine commit成功而假稱全體原子化 |
| npm包型／平台包名、Node/npm與scripts政策、ZIP啟動／簽章細節、是否離線build | agent以G1封裝閉包與平台證據提出；有企業限制時由使用者補充 | 最終分發／G4；不隱性下載、不猜scope權限、不公開npm發布 |
| watch、service／登入自啟、維護週期等後續選擇 | 未被選為首版必要能力的擴充保持OPEN；D18維護方案依G1實際元件及導入環境提出 | 對應擴充與正式運維承諾；不得藉此刪除已要求的原版功能 |

## 交付判定

G0／G1每個WP依實際證據更新狀態與可審查commit；未知項記owner與影響，不用它阻擋全部獨立工作。只有實際執行且有artifact／平台／輸入／結果的測試可標passed；目前80項產品測試仍planned，文件／ZIP結構檢查不代替產品測試。
