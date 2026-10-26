# npm 名稱與新增程式碼授權的界線

查核日期：2026-10-26；registry觀測時間：`2026-10-26T08:14:30+0800`。用途：D14（npm名稱／分發）與D17（名稱／新碼授權）的決策事實。**狀態：唯讀HTTP觀測＋固定原始文件查閱；未登入、保留名稱、publish、安裝或改帳號。**

後續決策：Q13已確認顯示名稱Rustgrok／npm主套件rustgrok，Q14已確認本文第3節的原創新碼MIT分工。以下保留查核當時尚未決定的歷史文字；名稱保留、發布與完整逐檔盤點仍未執行，政策見[授權範圍](../../LICENSE.md)。

## 1. Unscoped npm rustgrok：目前未查到，未驗證可發布

實際執行匿名HTTP GET：

```sh
curl -sS --max-time 30 -w '\nHTTP_STATUS=%{http_code}\n' 'https://registry.npmjs.org/rustgrok'
```

回覆body為 `{"error":"Not found"}`，HTTP status為 `404`。這只能記為**當時從公開registry未查到unscoped rustgrok的package metadata**；不是可發布保證、名稱保留或帳號權限證明。沒有查任何scope所有權。來源為[npm官方registry的rustgrok endpoint](https://registry.npmjs.org/rustgrok)；web工具不能讀此endpoint，狀態與body由上述curl GET直接觀測。

**查核當時的D14待選項，後續Q13已選rustgrok**：GitHub repo名稱已是rustgrok，但不因此自動決定npm名稱。若選unscoped `rustgrok`，實際發布前仍須重查；本輪沒有使用publish作可用性探測。

## 2. OpenGrok基線的授權與notice

固定基線為OpenGrok 1.14.20／commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`。其LICENSE開頭宣告預設 **CDDL 1.0 only**，個別來源檔可有例外；並指向NOTICE。故不能從repo總標籤推定每個CSS／JS／圖片／移植片段都是同一條件。見[LICENSE.txt L1–6](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt#L1-L6)。

CDDL 1.0中與本案直接相關的界線如下，屬條文摘要：

| 範圍 | 原條文含義 | 一級來源 |
|---|---|---|
| 沿用／修改檔案 | Covered Software包含原始軟體及modifications；modifications亦包括修改原檔及含原軟體部分內容的新檔。不能只因Java改寫為Rust或換檔名便宣告舊條件消失；特定移植成果的屬性仍需依實際內容判定。 | [LICENSE §1.3、§1.9，L22–25及L44–55](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt#L22-L55) |
| Covered source交付 | 散布Covered executable時，亦須讓其source可取得；Covered source依CDDL提供、附授權全文，並告知取得方式。modifications受該授權約束。 | [LICENSE §3.1–3.2，L156–175](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt#L156-L175) |
| Notice與修改者 | 修改需標識contributor；不得任意移除或改動原有copyright、patent、trademark、license或attribution notices。 | [LICENSE §3.3，L177–184](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt#L177-L184) |
| Binary及混合產品 | executable可有不同授權條件但不能消除Covered source權利；Larger Work可結合其他授權的碼，Covered部分仍須履行CDDL條件。這不是把整包或移植碼統一改MIT的許可。 | [LICENSE §3.5–3.6，L202–225](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt#L202-L225) |

[OpenGrok NOTICE.txt](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/NOTICE.txt#L1-L32)列有Lucene等第三方attributions；它不是尚未建立的Rust產品依賴清單。應按實際沿用／分發內容保留適用notice並增補其他元件條件，不能把這份NOTICE當成「全產品授權inventory完成」。CDDL條文另可對照[OSI官方CDDL 1.0頁面](https://opensource.org/license/CDDL-1.0)。

## 3. D17提案：只為可授權的獨立原創新碼選MIT

**查核時的Codex提案，後續Q14已接受此分工**：對具有授權權利、可辨識為獨立原創的新增Rust／JS檔案採MIT；OpenGrok沿用資產／修改／移植內容及第三方元件分別保留其適用條件。MIT要求在軟體副本或重要部分保留copyright與permission notice；它不會替沒有權利重授權的上游內容自動換約。見[OSI MIT全文](https://opensource.org/license/mit)，以及上述CDDL §3.2／§3.6。

建議實作時逐檔記錄origin URL、固定commit／hash、原授權／notice、修改與source對應；此為**工程追溯建議**，不是宣告所有新Rust檔案都獨立原創，亦不是完整法律相容結論。

## 4. 本輪未查證

npm實際發布權限／名稱保留、任何scope所有權、完整商標／命名核對、逐檔衍生屬性、所有assets與未來helpers／crates／JS依賴的license及source inventory，均未完成。查核動作本身沒有發布產品或套用根目錄MIT授權；其後Q13／Q14的使用者決策另記於決策表，不由HTTP查詢或條文查閱自行代決。
