# OpenGrok 1.14.20：搜尋、xref 與 raw 的來源一致性

研究日期：2026-10-26（Asia/Taipei）。固定 commit：`3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`，對應已確認的 OpenGrok 1.14.20。**狀態：STATIC_SOURCE_REVIEW；oracle runtime、產品驗收及以下情境均為 NOT_RUN。**

本筆記只查「外部 source 已修改／刪除、尚未 reindex」時的讀取來源。Q8 已由使用者選定：更新期間保留上一成功搜尋，新 index 及相關資料就緒後整批切換。**這不代表已批准保留所有 raw source snapshot、改原版 URL 或把每個閱讀入口改成 generation snapshot。** Q8 是 Rust 產品政策，以下是原版實作事實，兩者不能互代。

## 1. 可查證結論

原版各入口**沒有一律從同一份索引時 source snapshot 讀取**：

| 入口／資料 | 固定 source 的讀取來源 |
|---|---|
| 搜尋命中集合、count、排序與stored metadata | 目前開啟的 Lucene reader／searcher。外部修改 source 不會自行更新這份 index。 |
| HTML 搜尋結果的 `Genre.PLAIN` snippet | 從 source root 讀檔；新 highlighter 用 stored UID與mtime檢查是否對齊index。不對齊時，可fallback重新分析live source。 |
| HTML 結果的 `Genre.XREFABLE` snippet | 從 data root 的預生成xref產物取文字摘要。HTML genre則另讀source；不能將所有genre視為同一來源。 |
| 一般xref閱讀 | 視revision參數、預生成xref是否可用／夠新、annotate及economy mode分支，讀預生成xref、live source或SCM revision。 |
| `/raw`、`/download` 無revision | 直接讀目前source root的檔案。 |
| `/raw`、`/download` 有revision | 經HistoryGuru取指定SCM revision；仍先經共同resource可用性檢查。 |

來源鏈與條件見以下各節；此表不是 runtime 成功宣告。

## 2. 搜尋結果：index命中與snippet來源分開

HTML入口先執行 `SearchHelper.prepareExec(...).executeQuery().prepareSummary()`，再用 `Results.prettyPrint` 輸出。SearchHelper 的 `executeQuery` 呼叫 Lucene searcher，從 TopDocs取totalHits與scoreDocs；這段並不重新掃live source決定命中。見 [search.jsp L81–86](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/search.jsp#L81-L86)、[L210–228](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/search.jsp#L210-L228)、[SearchHelper.java L473–491](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/SearchHelper.java#L473-L491)。

Results從index stored fields取path／genre，然後分支產生context。XREFABLE從data root的xref讀摘要，HTML genre從source root讀，PLAIN進 `printPlain`。見 [Results.java L210–247](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/Results.java#L210-L247)。

PLAIN的主要路徑是 `Results.printPlain → Context.getContext2 → OGKUnifiedHighlighter`。highlighter依index的PATH與U（UID）開啟live source；檔案不存在或用path＋mtime組出的UID不符，回null。這是timestamp檢查，不是保存了索引時完整文字的證據。見 [Context.java L201–227](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/context/Context.java#L201-L227)、[OGKUnifiedHighlighter.java L209–237](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/context/OGKUnifiedHighlighter.java#L209-L237)、[L284–323](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/context/OGKUnifiedHighlighter.java#L284-L323)。

若主要highlighter未產生context，Results會用FileInputStream重新讀source、以舊context路徑分析；註解直接說這可發生於source已更新而reindex尚未完成。檔案讀取失敗則記錄「No context」，沒有在此從舊snapshot補回文字。見 [Results.java L275–318](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/Results.java#L275-L318)。

**靜態推導**：未reindex時，舊命中仍可能出現在results，但context已來自新source、沒有原查詢的命中行，或因刪檔而沒有context。不能把保留舊index等同保留舊snippet。精確可見文字與highlight結果仍需oracle實跑。

## 3. Xref：預生成、live與SCM revision三條路徑

預生成xref是indexing產物：`IndexDatabase.addFile` 在生成HTML開啟時建立xref writer，與populateDocument一起處理檔案，之後排入pending rename。這能證明它有索引時產物來源，不能單靠這段證明所有資料有跨資產原子切換。見 [IndexDatabase.java L1152–1196](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/index/IndexDatabase.java#L1152-L1196)。

xref UI的入口是 `list.jsp`（透過 `/xref/*` mapping），不是一個 `xref.jsp` servlet。無revision時，它可能先取latest SCM revision並redirect，將revision放入URL；economy mode且沒有可用revision時，可改用dummy `unknown` revision來直接渲染resource。見 [web.xml L214–220](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/WEB-INF/web.xml#L214-L220)、[list.jsp L61–87](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/list.jsp#L61-L87)。

latest revision也不是單純「目前工作樹版本」：`LatestRevisionUtil` 先查index的LASTREV與檔案時間是否仍適用，不適用才fallback history；未提交dirty source沒有因此成為一個SCM commit。見 [LatestRevisionUtil.java L57–112](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/LatestRevisionUtil.java#L57-L112)。

一般檔案渲染分支：

- 沒有已知revision：先 `findDataFile`，找到就dump預生成xref；否則include `xref.jspf` 即時生成。見 [list.jsp L289–306](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/list.jsp#L289-L306)。
- 預生成檔不是只要存在就使用：`PageConfig.checkFileInner` 要求xref mtime >= source mtime，否則不接受，後續可fallback。壓縮／未壓縮也有尋找次序。見 [PageConfig.java L1234–1275](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/PageConfig.java#L1234-L1275)、[L1326–1328](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/PageConfig.java#L1326-L1328)。
- 指定revision且它是latest、預生成檔亦可用：非annotate分支dump預生成xref；annotate分支可另開live source配definitions／annotation渲染。因此連「URL帶revision」也不宜一律外推為每條渲染路徑只讀SCM blob。見 [list.jsp L212–275](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/list.jsp#L212-L275)。
- 預生成路徑未採用而進 `xref.jspf`：rev空白或dummy時開live source；真正revision則呼叫HistoryGuru取SCM內容到temp file再渲染。見 [xref.jspf L49–74](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/xref.jspf#L49-L74)。

因此外部修改但未reindex時，xref**可能變成live即時渲染，也可能顯示某個SCM revision內容**，取決於上述參數與設定；不能宣稱它一定保留上一成功xref或一定跟目前raw相同。完整annotation/cache分支不在本次研究範圍。

## 4. Raw／download：無revision讀live source

`/raw/*` 和 `/download/*` 都映射到 `GetFile`。它取得resource file後：

- 有revision：呼叫 `HistoryGuru.getRevision`。
- 無revision：直接FileInputStream(resource file)，並用當前檔案長度／mtime設HTTP metadata。
- download加attachment header；raw回text/plain；兩者不因入口不同而切換到索引內的source snapshot。

見 [web.xml L158–170](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/WEB-INF/web.xml#L158-L170)、[L222–229](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/WEB-INF/web.xml#L222-L229)、[GetFile.java L62–105](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/GetFile.java#L62-L105)。

一般resource路徑由 `sourceRoot + request path` 取得且需canRead；找不到時回退為不可用resource，`canProcess` 拒絕。GetFile與xref使用的mast入口會對此回404。故刪除live source後，不能只因舊index或xref仍存在就保證原URL還可閱讀／下載。見 [PageConfig.java L1153–1181](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/PageConfig.java#L1153-L1181)、[L1208–1212](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/PageConfig.java#L1208-L1212)、[L437–440](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/PageConfig.java#L437-L440)、[GetFile.java L49–55](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/GetFile.java#L49-L55)、[mast.jsp L47–54](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/mast.jsp#L47-L54)。

## 5. 外部變動後的可見差異與Q8界線

以下為固定source控制流程的**推導情境**，不是runtime驗收結果：

| source變動、未reindex | 可能的結果 |
|---|---|
| 修改文字並更新mtime | 命中仍依舊index；PLAIN snippet可fallback讀新文字。預生成xref可能因mtime過舊被放棄。無revision raw下載新文字。 |
| Git工作樹有未提交修改，xref被redirect到真正SCM revision | xref fallback可取SCM blob，而無revision raw仍讀dirty工作樹；兩者內容不保證一致。 |
| source已刪除 | 舊index仍可能命中；PLAIN context可能讀不到。一般xref/raw resource入口可能404，而不是保證從舊xref恢復完整閱讀。 |
| 修改內容但保留mtime | 已讀程式以mtime／UID判斷部分有效性；這不構成content hash一致保證。具體顯示仍UNKNOWN。 |

Q8已選的「保留上一成功搜尋、就緒後整批切換」應在Rust設計中明確界定哪些產物受generation控制。原版source顯示**命中、snippet、xref及raw有各自來源**，不能把Q8自動擴成「所有閱讀入口永遠同snapshot」，也不能把採snapshot後raw讀舊文字的使用者可見差異隱藏成純內部細節。

本輪僅提供上述事實，不替使用者選「raw維持live」或「保留source snapshot」，不批准URL／revision／閱讀契約變更。後續若需選擇，應呈現「讀到外部最新source」與「仍能閱讀／下載上一成功generation內容」的具體差異及儲存成本；不以單獨index commit當成完整一致性證明。

## 6. 驗證界線與重現起點

- **STATIC_SOURCE_REVIEW**：已讀固定commit的Web入口、Results／Context／highlighter、xref生成／選用、raw/download與resource檢查。
- **NOT_RUN**：未啟動OpenGrok或Rust、未修改合成source做HTTP重現、未驗HTML/API完整差異，也未執行產品測試。
- **UNKNOWN**：各實際設定下redirect後精確畫面、HTTP/browser caching、rename／刪檔競態與annotation/cache全分支；本輪沒有擴查它們或將其填成PASS。
- 後續可在既有 `SRC-002`、`SRC-005`、`UI-002`、`UI-004`、`QRY-006`／`RST-005` 中使用同一合成檔，依序保存index後輸出、外部修改未reindex、外部刪除未reindex，再完成reindex。每步記query/hits/context、xref redirect/revision、raw/download bytes及狀態；不新增test IDs、不改planned狀態。

來源取得備註：最初以 `xref.jsp`／`raw.jsp` 猜測檔名的官方GitHubURL回404；之後以固定commit tree確認實際入口為list.jsp／xref.jspf及GetFile。404只表示該猜測path不存在，不是相關功能不存在。
