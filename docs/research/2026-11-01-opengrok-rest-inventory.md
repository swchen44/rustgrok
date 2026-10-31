# OpenGrok 1.14.20：REST 完整方法 inventory

日期：2026-11-01。工作包 **WP-A05**；Issue [#2](https://github.com/swchen44/rustgrok/issues/2)。**狀態：STATIC_SOURCE_REVIEW；oracle HTTP、上游 tests、Rust 產品及平台驗收全部 NOT_RUN。** 本文只盤點固定來源可見的 REST 宣告、方法本體及必要 glue，不執行 OpenGrok、build 或 tests。

來源固定為 OpenGrok **1.14.20**，commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`。本機來源為 `~/Library/Caches/rustgrok/oracle/opengrok-1.14.20/source`；本輪另對 13 個 controller 的實際 bytes 重算 Git blob SHA-1，全部與該 checkout 的 `git ls-tree -r HEAD` 一致。以下上游連結皆 pin 到同一 commit，來源條件仍按各檔 CDDL／copyright header 及專案 LICENSE 保留；研究摘要不表示上游內容改採 MIT。

## 1. 邊界、列舉與閱讀方式

`RestApp` 的 `@ApplicationPath` 是 **`api/v1`**，遞迴註冊 `org.opengrok.web.api.v1`，另註冊 constraints、error packages 與 suggester binder。實際 URL 為 **`{部署 context}/api/v1/{class path}/{method path}`**；不能固定假設 context 是 `/source`。來源：[RestApp.java L29–39](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/RestApp.java#L29-L39)。

本文逐一讀完 controller 目錄中的 13 個 class，展開 **48 個帶 HTTP verb 的 Java resource methods：GET 28、PUT 7、POST 7、DELETE 6**。以 `application prefix + class path + method path + HTTP verb` 去重後為 **47 個 route keys**；差異來自 `GET file/content` 的 `text/plain` 與 `application/octet-stream` 兩個 `@Produces` overload。這是來源宣告計數，不是功能、測試或 runtime endpoint 成功數。

13 個 class 均未繼承自訂 resource superclass；本目錄未見無 HTTP verb 的 `@Path` subresource locator，也未見自行宣告 `HEAD`／`OPTIONS`／`PATCH`。因此表內沒有漏加的 inherited resource methods 或 subresource class；Jersey 自動提供的 HTTP 行為、content negotiation、trailing slash、method-not-allowed 等仍須 oracle 實測，不算成已驗證的額外 endpoint。

表內 `—` 表示 method 沒有額外 `@Path`；`Q` 是 query、`P` 是 path、`B` 是 request body。所有 controller method 都未直接宣告 `@HeaderParam`；全域 `Authorization` 與部分方法的 `Origin` 由 filters 處理，見第 6 節。未宣告 `@Consumes`／`@Produces` 的位置寫「未指定」，不替原版補上 JSON 或 text/plain 契約。沒有明訂 `@DefaultValue` 時，`String`／boxed 值按宣告可為 null，primitive boolean/int 的缺省 false/0 屬框架注入預期，待 oracle 核對。

狀態欄把**本體／mapper 明訂 status**與「值返回」「void」「null 返回」分開；後三者的常見 JAX-RS 200／204 行為僅是待驗預期，本文不把它們冒充 HTTP capture。`Response.sendError` 可能走 servlet error 處理，不能假設與 JSON mapper 同一 body。各列另外受到第 6 節的 filters／mappers 影響。

D16 已確認 Rustgrok 的**管理網頁／有副作用的管理 API 只限主機本機，LAN 提供搜尋／瀏覽**。下列「讀者」「管理變更」「管理讀取」是依實際本體描述操作，不用 HTTP verb 代判，也不把原版 token allowlist 當成 Rustgrok 的可達性設計。無角色／project ACL 的 D02 維持不變。

## 2. 搜尋與內容閱讀

### 2.1 SearchController；class path `search`

| HTTP | method path | 輸入／預設 | 回傳與來源可見 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | — | Q `full`,`def`,`symbol`,`path`,`hist`,`type`: String；`projects`: List<String>；`maxresults`: Integer，缺省 `hitsPerPage * cachePages`；`start`: int=0；`sort`: String=`relevancy`；`maxhitsperfile`: int=0 | JSON `SearchResult`；負數、`start+maxresults` int overflow、invalid query 明訂 400；其餘值返回。[SearchController.java L72–135](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SearchController.java#L72-L135) | 讀者搜尋，但會先呼叫 `suggester.onSearch(projects, query)`；有條件更新人氣統計，不能寫成純讀取。 |

`def`、`symbol`、`projects` 是 REST 的精確拼法，不可替換為 UI 的 `defs`／`refs`／`project` 或 suggester 的 `projects[]`。QueryParameters 的參數字串來源見 [QueryParameters.java L89–138](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/QueryParameters.java#L89-L138)。`sort` 已知值為 `relevancy`、`lastmodtime`、`fullpath`；未知字串由 `SortOrder.get()` 得 null，再交 engine，controller 沒有直接拒絕未知 sort，後續結果待 oracle。見 [SortOrder.java L32–65](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/SortOrder.java#L32-L65)。

不指定 projects 或空 list 時，wrapper 搜尋 `PageConfig` 取得的全部 projects；指定時與該集合按名稱取交集，未直接為未知名稱回 404。結果按 path 分組並保留 `LinkedHashMap` 順序。`SearchResult` 欄位為 `time`（ms）、`resultCount`、`results: Map<String,List<SearchHit>>`、`startDocument`、`endDocument`；每個 hit 為 `line`,`lineNumber`,`tag`，沒有 REST score 欄位。空頁的 `endDocument` 等於 `startDocument`。見 [SearchController.java L120–134](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SearchController.java#L120-L134)、[SearchController.java L166–223](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SearchController.java#L166-L223)。這個 response 不代表 UI／engine 的所有命中資訊或 ranking 契約已完成。

人氣副作用的條件包括 service 已初始化、`allowMostPopular=true`、projects 非 null；suggester 對 query 展開 terms 並增加 per-project counts。缺 projects 的全專案搜尋與指定 projects 的人氣行為不能視為必然相同。見 [SuggesterServiceImpl.java L240–252](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/suggester/provider/service/impl/SuggesterServiceImpl.java#L240-L252)、[Suggester.java L576–592](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/suggester/src/main/java/org/opengrok/suggest/Suggester.java#L576-L592)。

### 2.2 AnnotationController；class path `annotation`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | — | Q `path`: String；`revision`: String，null 或空字串轉 null 傳給 HistoryGuru | JSON `List<AnnotationDTO>`；annotate 為 null 時回空 list；IOException／path 錯誤交共同處理。[AnnotationController.java L65–92](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/AnnotationController.java#L65-L92) | 讀者 annotate；呼叫原 SCM/history 分支，未由 controller 新增 source snapshot。 |

DTO 為 `revision`,`author`,`description`,`version`；`version` 由 `fileVersion + "/" + revisions.size()` 生成，依註解 source line 1 起算的順序輸出。見 [AnnotationController.java L55–88](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/AnnotationController.java#L55-L88)。

### 2.3 HistoryController；class path `history`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | — | Q `path`: String；`withFiles`: boolean，無明訂預設；`max`: int=1000；`start`: int=0 | JSON `HistoryDTO`；HistoryGuru 回 null 時本體回 null；HistoryException／IOException 交 mapper，沒有 controller 明訂負數上限檢查。[HistoryController.java L172–193](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/HistoryController.java#L172-L193) | 讀者 history；`getHistory(file,withFiles,true)`。 |

`HistoryDTO` 為 `entries`,`start`,`count`,`total`；`count` 使用請求的 `max`，不直接等於本頁實際 entries 數。entry 為 `revision`,`date`,`author`,`tags`,`message`,`files`，tags 另由 revision map 補入。見 [HistoryController.java L65–89](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/HistoryController.java#L65-L89)、[HistoryController.java L120–169](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/HistoryController.java#L120-L169)。

### 2.4 DirectoryListingController；class path `list`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | — | Q `path`: String | JSON `List<DirectoryEntryDTO>`；IOException／NoPathParameterException／CacheException；DTO 轉換若遇 IOException 或 ForbiddenSymlinkException，catch 後仍加入 null entry。[DirectoryListingController.java L138–175](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/DirectoryListingController.java#L138-L175) | 讀者目錄列表；`PageConfig`、DirectoryListing、索引 extras 合併，未見顯式管理設定變更；原 IncomingFilter 未 allowlist 此路徑。 |

DTO 為 `path`,`numLines`,`loc`,`date`,`description`,`pathDescription`,`isDirectory`,`size`；目錄 size=null，檔案 size 取當下 `File.length()`。見 [DirectoryListingController.java L70–107](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/DirectoryListingController.java#L70-L107)。這個 REST DTO 與 xref 的 HTML 目錄表是不同可觀察輸出。

### 2.5 FileController；class path `file`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | `/content` | Q `path`: String；以 Accept 協商 `text/plain` variant | `StreamingOutput`，Produces `text/plain`；索引 document 不存在時 `sendError(404)`；genre 非 PLAIN 時 `sendError(406)`；存在則 transfer live file。[FileController.java L83–107](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/FileController.java#L83-L107) | 讀者內容；要求先有相應 index document 且為純文字。 |
| GET | `/content` | Q `path`: String；以 Accept 協商 octet-stream variant | `StreamingOutput`，Produces `application/octet-stream`；transfer FileNotFoundException catch 為 `sendError(404)`；先前 `toFile` 的例外交 mapper。[FileController.java L109–126](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/FileController.java#L109-L126) | 讀者 binary／原始 bytes；沒有 plain variant 的 index／genre gate。 |
| GET | `/genre` | Q `path`: String | `String`，Produces `text/plain`；document 不存在或無可辨識 genre，`sendError(404)`。[FileController.java L128–152](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/FileController.java#L128-L152) | 讀者索引 genre metadata。 |
| GET | `/defs` | Q `path`: String | JSON `List<Object>`，每個 Object 是 Definitions.Tag 的 DTO；無 defs 回空 list；IOException／ParseException／ClassNotFoundException 交共同 mapper。[FileController.java L154–172](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/FileController.java#L154-L172) | 讀者 index definitions。 |

兩個 content overload 都最終以 `FileInputStream` 讀當下檔案，沒有 revision 參數；不能把 REST `file/content` 與 servlet `raw`／`download` 的 SCM 分支混為同一契約。stream 寫出後發生 IO 錯誤的 wire status／部分 body 待 oracle。transfer 見 [FileController.java L66–80](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/FileController.java#L66-L80)。defs DTO 的欄位由 `@DTOElement` 驅動，含 `line`,`symbol`,`type`,`text`,`namespace`,`signature`,`lineStart`,`lineEnd`，見 [DTOUtil.java L40–56](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/util/DTOUtil.java#L40-L56)、[Definitions.java L219–284](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/Definitions.java#L219-L284)。

## 3. SuggesterController；class path `suggest`

整個 class 帶 `@Suggester`，因此所有下列 methods 都受 `SuggestionsEnabledFilter` 影響；關閉 suggester 時 filter 明訂 404，不能只對第一條 GET 套用。見 [SuggesterController.java L83–99](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L83-L99)、[SuggestionsEnabledFilter.java L31–45](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/suggester/provider/filter/SuggestionsEnabledFilter.java#L31-L45)。

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | — | `@Valid @BeanParam SuggesterQueryData`，完整 Q 見下文 | JSON `Result`；無可建立的 suggester query 拋 ParseException；不符配置回 404；validation／parse 由 mapper 回 400。[SuggesterController.java L108–138](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L108-L138) | 讀者補全；`@Authorized` project filter，非人氣初始化或重建。 |
| GET | `/config` | 無 body／query | JSON `SuggesterConfig` 值返回。[SuggesterController.java L162–175](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L162-L175) | 讀者補全設定讀取；上游特別另開此入口供遠端頁面取得。 |
| PUT | `/rebuild` | 無參數；Consumes 未指定 | void；直接 `CompletableFuture.runAsync(suggester::rebuild)`，未回 task UUID／Location。[SuggesterController.java L177–181](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L177-L181) | 管理變更：全體補全重建，response 不代表已建完。 |
| PUT | `/rebuild/{project}` | P `project`: String | void；CompletableFuture；controller 無 project 404。[SuggesterController.java L183–187](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L183-L187) | 管理變更：單 project 補全重建；service 對不存在、未 indexed 或未初始化分支只 log／return。 |
| POST | `/init/queries` | B JSON `List<String>` URLs | void；URISyntaxException 未在 method catch；MalformedURLException log 並繼續；query parse 失敗被忽略。[SuggesterController.java L189–224](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L189-L224) | 管理變更：從 URLs 拆 `project` 與 query fields，呼叫 `onSearch` 初始化／增加人氣。 |
| POST | `/init/raw` | B JSON `@Valid List<TermIncrementData>` | void；bean validation 交 400 mapper。[SuggesterController.java L259–271](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L259-L271) | 管理變更：逐筆 `increaseSearchCount(project,Term(field,token),increment)`；controller 未檢查 service boolean 結果。 |
| GET | `/popularity/{project}` | P `project`: String；Q `field`: String=`full`；`page`: int=0；`pageSize`: int=100；`all`: boolean，無明訂預設 | JSON `List<Entry<String,Integer>>`；非 `QueryBuilder.isSearchField` 的 field 明訂 400；all=true 使用 page=0、size=Integer.MAX_VALUE；無 page/pageSize 非負檢查。[SuggesterController.java L273–308](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L273-L308) | 人氣資料讀取，未見管理寫入；原 IncomingFilter 不 allowlist 此路徑，Rustgrok 的讀者／管理讀取可達性待契約細化。 |

補全 Bean Q 為 `projects[]: List<String>`；`field: String` 必須 nonblank 且符合 `full|defs|refs|path|hist`；`caret: int` 必須 ≥0，無明訂預設；以及 `full`,`defs`,`refs`,`path`,`hist`,`type`: String。`type` 能參與 query，但不在 `field` 可選 regex。見 [SuggesterQueryData.java L37–68](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/suggester/model/SuggesterQueryData.java#L37-L68)、[AuthorizationFilter.java L40–67](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/suggester/provider/filter/AuthorizationFilter.java#L40-L67)。parser 會過濾不存在的 project；`caret > text.length()` 或目標 field text 為 null 時拋 IllegalArgumentException，沒有專屬 400 mapper，來源可見 generic mapper 路徑是 500，仍須實際 provider／注入行為核對，不能自行改成一律400。見 [SuggesterQueryDataParser.java L61–109](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/suggester/parser/SuggesterQueryDataParser.java#L61-L109)。

Result 欄位為 `time`,`suggestions: List<LookupResultItem>`,`identifier`,`queryText`,`partialResult`；LookupResultItem 的 getter 欄位為 `phrase: String`,`projects: Set<String>`,`score: long`，見 [LookupResultItem.java L33–57](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/suggester/src/main/java/org/opengrok/suggest/LookupResultItem.java#L33-L57)。Result 見 [SuggesterController.java L310–374](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L310-L374)。`TermIncrementData` 欄位為 `project`,`field`,`token`,`increment`；field/token `@NotBlank`，increment 的 `@Min(0)` 實際接受零，雖 annotation message 說 positive。未指定值時 int 為 0；project 無此 bean 的 NotBlank。見 [SuggesterController.java L377–420](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L377-L420)。List 元素驗證、未知 JSON properties、Entry／LookupResultItem／完整 SuggesterConfig 的實際 wire schema 仍須獨立 capture，不能由 Java type 推斷全部序列化細節。

配置會移除不在 allowedProjects 的項目，再檢查 minChars、maxProjects、allowedFields、allowComplexQueries；這些是上游配置行為，不表示 Rustgrok 要加回 RBAC／project ACL。見 [SuggesterController.java L140–160](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SuggesterController.java#L140-L160)。單 project rebuild service 的 no-op 分支見 [SuggesterServiceImpl.java L195–217](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/suggester/provider/service/impl/SuggesterServiceImpl.java#L195-L217)。

## 4. Project、repository、group 與 messages

### 4.1 ProjectsController；class path `projects`

此 controller 的管理變更會寫 project／repository／cache／searcher 狀態；不能把它當成只改名冊。`Project` 與 `Repository` 依 GLOSSARY 分開。表中「task→N」是先 submit 回 202，完成後由 status endpoint 回 N，詳見第 5 節。

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| POST | — | B `text/plain` projectName String | `Response` task→201，結果 body=null。[ProjectsController.java L91–106](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L91-L106) | 管理變更：新增 project；重加現有 project 時刷新 repositories；不是執行索引。 |
| DELETE | `/{project}` | P project String | 未知 project 同步 NotFoundException→404；先 disable 再 submit task→204。[ProjectsController.java L164–190](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L164-L190) | 管理變更：移除 index／xref／cache、groups、repository mapping、project 設定並 refresh searcher map；不刪使用者 source。 |
| DELETE | `/{project}/data` | P project String | 未知 project 404；先 disable 再 submit task→204。[ProjectsController.java L217–233](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L217-L233) | 管理變更：保留 project 登記，清除生成資料，並非 source delete。 |
| DELETE | `/{project}/annotationcache` | P project String | Produces JSON；未知 project 404；無 repositories 回 null；否則 task→200，JSON `List<String>`。[ProjectsController.java L267–291](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L267-L291) | 管理變更：移除 annotation cache。 |
| DELETE | `/{project}/historycache` | P project String | Produces JSON；未知 project 404；history disabled 明訂204；無 repositories 回 null；否則 task→200，JSON `List<String>`。[ProjectsController.java L293–320](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L293-L320) | 管理變更：移除 history cache。 |
| PUT | `/{project}/indexed` | P project String；Consumes `text/plain`，method 沒讀 body | 未知 project404；同步 `setIndexed(true)` 後 task→200、body=null。[ProjectsController.java L329–371](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L329-L371) | 管理變更：刷新 repository current version、異步 suggester rebuild、refresh index searchers／last index date。 |
| PUT | `/{project}/property/{field}` | P project,field String；B String value；Consumes 未指定 | void；已知 project 以 ClassUtil 設值並同步相應 repository 欄位；未知 project 只 log；IOException 交 generic mapper。[ProjectsController.java L373–402](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L373-L402) | 管理變更：反射設 property，並非任意 JSON PATCH。 |
| GET | `/{project}/property/{field}` | P project,field String | JSON Object；未知 project404；ClassUtil field 讀取 IOException 交 generic mapper。[ProjectsController.java L404–416](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L404-L416) | 管理設定讀取，未見顯式設定寫入。 |
| GET | — | 無參數 | JSON `List<String>` project names。[ProjectsController.java L418–422](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L418-L422) | project 名冊讀取；原版非 allowlist。 |
| GET | `indexed` | 無參數 | JSON `List<String>`，filter `Project.isIndexed()`。[ProjectsController.java L424–432](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L424-L432) | 可搜尋 project 名冊讀取；原版非 allowlist。 |
| GET | `/{project}/repositories` | P project String | JSON `List<String>` source-root relative repo directory names；未知 project／無 mapping 回空 list。[ProjectsController.java L434–452](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L434-L452) | repository metadata 讀取；未假設 project=單一 Git repo。 |
| GET | `/{project}/repositories/type` | P project String | JSON `Set<String>` repository type；未知 project／無 mapping 回空 set。[ProjectsController.java L454–471](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L454-L471) | repository 類型讀取，未限 Git。 |
| GET | `/{project}/files` | P project String | JSON `Set<String>`，`IndexDatabase.getAllFiles(["/"+project])`；IOException 交 generic mapper；controller 未先查 project。[ProjectsController.java L473–481](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L473-L481) | 索引檔案路徑讀取。 |

新增的 workhorse 會以 sourceRoot/projectName 探 repo，新 project 保持未 indexed；現有 project 的重加會增刪 repository 清單。見 [ProjectsController.java L108–162](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L108-L162)。資料清除操作刪除 dataRoot 中 `index/{project}`、`xref/{project}`；刪檔 IOException 只 log，若 repository list 為 null／空則直接 return，後方 history／annotation／suggester delete 分支不執行。完整 project delete 另移除 HistoryGuru repositories；data-only 不做該移除。見 [ProjectsController.java L192–265](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L192-L265)、[ProjectsController.java L315–327](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ProjectsController.java#L315-L327)。因此不能把 completed task 的成功 status 解讀為每一刪檔必然成功。

`setIndexed(true)` 和 delete 的 `setIndexed(false)` 都發生在 task submit 之前；submit 失敗的 400 不保證先前狀態已回復。這也不是 D12 的「已發布搜尋版本整批切換」證明，`indexed` 完成亦未等待它另啟的 CompletableFuture suggester rebuild。來源同上；相關發布契約仍依 ADR-0003／G1。

### 4.2 RepositoriesController；class path `repositories`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | `/property/{field}` | Q `repository`: String，必須精確比對 `getDirectoryNameRelative()`；P field String | JSON Object；先將 RepositoryInfo 轉 DTO，再 `ClassUtil.getFieldValue`；找不到 repository 明訂404；IOException 交 generic mapper。[RepositoriesController.java L40–68](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/RepositoriesController.java#L40-L68) | repository 設定／metadata 讀取；無管理寫入 endpoint。 |

這個 endpoint 只能取得 DTO 包含的 property；不可假設 RepositoryInfo 所有 Java fields 都已暴露。DTOElement 機制見 [DTOUtil.java L40–56](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/util/DTOUtil.java#L40-L56)；完整允許欄位與型別矩陣仍需逐欄展開及 wire capture。

### 4.3 GroupsController；class path `groups`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | — | 無參數 | JSON `List<String>` names；無 groups 回空 list。[GroupsController.java L49–60](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/GroupsController.java#L49-L60) | group metadata 讀取。 |
| GET | `/{group}/allprojects` | P group String | JSON Response；不存在404；存在200，entity 為 project names List，無成員空 list。[GroupsController.java L62–81](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/GroupsController.java#L62-L81) | 遞迴 group 所含 project 名稱讀取；無寫入。 |
| GET | `/{group}/pattern` | P group String | text/plain Response；不存在404；存在200，entity 為 pattern。[GroupsController.java L83–96](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/GroupsController.java#L83-L96) | group pattern 設定讀取。 |
| POST | `/{group}/match` | P group String；B text/plain projectName String | Response；不存在group404；符合200；不符204；無 response media 宣告。[GroupsController.java L98–118](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/GroupsController.java#L98-L118) | **無設定寫入**：以 group pattern 測試名稱；不是把 project 加入 group。 |

`Group.match(String)` 本體僅回 `compiledPattern.matcher(name).matches()`；見 [Group.java L234–252](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/configuration/Group.java#L234-L252)。這個 POST 是不用 HTTP verb 判管理變更的具體例子。上游 IncomingFilter 對這四條均要求非 allowlist 的 token 路徑，Rustgrok 的管理讀取／metadata 可達性細分仍待契約，不能憑 POST 自行限制 LAN 搜尋所需資訊。

### 4.4 MessagesController；class path `/messages`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| POST | — | B JSON `@Valid Message` | Response201；validation 例外交400 mapper。[MessagesController.java L48–54](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/MessagesController.java#L48-L54) | 管理變更：env.addMessage，新增 UI 訊息及其到期處理。 |
| DELETE | — | Q `tag`: String 必須非null；B text/plain text String | void；tag缺少明訂400；tag可空字串，controller 不驗 nonblank。[MessagesController.java L56–63](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/MessagesController.java#L56-L63) | 管理變更：刪除含 tag 且符合 text 的訊息；text為null／空時刪該tag任意文字。 |
| GET | — | Q `tag`: String 可null | JSON `Set<AcceptedMessage>`；有tag查該tag，否則全部。[MessagesController.java L65–73](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/MessagesController.java#L65-L73) | 訊息讀取；container 若未啟動 expiration timer，讀取時會 `expireMessages()`，不能絕對宣稱沒有資料變更。 |

Message 的 body fields 為 `tags`（缺省主頁tag、NotEmpty）、`messageLevel`（info；success/info/warning/error）、`text`（NotBlank）、`duration`（10分鐘；ISO-8601 duration serializer／正值 constraint）；text輸出使用 HTML encoding。見 [Message.java L50–100](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/messages/Message.java#L50-L100)、[Message.java L239–310](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/messages/Message.java#L239-L310)。duration 的 validator 明確要求非null、非負且非零，見 [PositiveDurationValidator.java L32–43](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/api/constraints/PositiveDurationValidator.java#L32-L43)。`AcceptedMessage` 明訂 JSON getters 包括 `text`,`messageLevel`,`created`,`tags`,`expiration`，內部 message 有 JsonIgnore；另有 `isExpired()`，完整 serializer 可見欄位仍以 wire capture 為準。見 [MessagesContainer.java L243–290](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/messages/MessagesContainer.java#L243-L290)。讀取時可能 expire 的條件見 [MessagesContainer.java L71–113](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/messages/MessagesContainer.java#L71-L113)；tag/text 比對見 [Message.java L153–164](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/messages/Message.java#L153-L164)。

## 5. 設定、系統與非同步工作

### 5.1 ConfigurationController；class path `configuration`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | — | 無參數 | application/xml String=`env.getConfigurationXML()`。[ConfigurationController.java L72–76](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ConfigurationController.java#L72-L76) | 管理設定讀取。 |
| PUT | — | B application/xml，由 request stream 以 UTF-8讀完整字串；Q `reindex`: boolean，無明訂預設 | Response task→201、body=null；request IO error 交 mapper。[ConfigurationController.java L78–94](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ConfigurationController.java#L78-L94) | 管理變更：異步 applyConfig(body,reindex,RESTFUL)，之後 suggester refresh。 |
| GET | `/{field}` | P field String | JSON Object；ClassUtil 讀 configuration 欄位的 IOException 明訂400。[ConfigurationController.java L96–101](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ConfigurationController.java#L96-L101) | 管理設定欄位讀取；可用欄位與字串型別轉換依 ClassUtil。 |
| PUT | `/{field}` | P field String；B String value，Consumes 未指定 | 同步 setConfigurationValueException；IOException 明訂400；之後 Response task→204、body=null。[ConfigurationController.java L103–117](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ConfigurationController.java#L103-L117) | 管理變更：先改欄位，再異步 applyConfig(false,RESTFUL)、suggester refresh。 |
| POST | `/authorization/reload` | 無參數 | Response task→204、body=null，queue名 authorization。[ConfigurationController.java L119–129](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ConfigurationController.java#L119-L129) | 上游管理變更：reload authorization framework；D02 已排除其 mandatory parity，不據此要求帳號／RBAC 或 JVM bridge。 |

`getField`／`setField` 的 400 是本體明訂，與 project／repository property IOException 的 generic500不同。field reflective helpers 見 [ConfigurationController.java L131–164](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/ConfigurationController.java#L131-L164)。`setField` 的同步寫入在 submit 之前，400 missing queue 不表示未改設定；XML set 的 applyConfig 則在 task 裡執行。

### 5.2 SystemController；class path `system`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| PUT | `/includes/reload` | 無參數；Consumes 未指定 | void。[SystemController.java L58–62](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SystemController.java#L58-L62) | 管理變更：reload include files。 |
| POST | `/pathdesc` | B application/json `@Valid PathDescription[]` | void；IOException 交 generic mapper。[SystemController.java L64–71](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SystemController.java#L64-L71) | 管理變更：`EftarFile.create(Set.of(descriptions),dtagsEftarPath)` 寫 path descriptions。 |
| GET | `/indextime` | 無參數 | application/json String：由 ObjectMapper 將最後 index date 寫成 ISO-8601含時區的 JSON；null日期分支交 serializer。[SystemController.java L73–82](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SystemController.java#L73-L82) | index時間讀取，IncomingFilter allowlist；不能以此代表完整 generation publish。 |
| GET | `/version` | 無參數 | text/plain String，格式 `%s (%s)`：version與revision。[SystemController.java L84–89](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SystemController.java#L84-L89) | 版本讀取；原 IncomingFilter **未** allowlist。 |
| GET | `/ping` | 無參數 | String空字串；Produces未指定，無明訂 status。[SystemController.java L91–95](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SystemController.java#L91-L95) | ping讀取，IncomingFilter allowlist；不等於完整健康檢查。 |

PathDescription Java bean 為 `path`,`description`: String，來源沒有這兩個欄位的 validation constraints；見 [PathDescription.java L25–51](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/PathDescription.java#L25-L51)。controller 也沒有 `shutdown`、`reindex` 或排程管理 route；不應從產品計畫倒推成這個版本已有的 REST。

### 5.3 StatusController；class path `status`

| HTTP | method path | 輸入／預設 | 回傳與 status／錯誤 | 實際操作 |
|---|---|---|---|---|
| GET | `/{uuid}` | P uuid String，manager 用 `UUID.fromString` 解析 | Response：找不到404；未完成202；完成則回 apiTask.getResponse 的完成 status／entity。[StatusController.java L50–63](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/StatusController.java#L50-L63) | 管理工作狀態讀取；不是 worker cancel。 |
| DELETE | `/{uuid}` | P uuid String | Response：找不到404；尚未完成400；已完成刪task後200。[StatusController.java L65–81](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/StatusController.java#L65-L81) | 管理變更：刪已完成 task 記錄；不取消正在執行的工作，也不回復工作副作用。 |

非合法 UUID 在 manager 的 `UUID.fromString` 拋例外，controller 未處理；走 generic mapper 的來源預期為500，不能把所有不存在／不合法UUID都寫404。見 [ApiTaskManager.java L64–70](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/ApiTaskManager.java#L64-L70)、[ApiTaskManager.java L118–124](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/ApiTaskManager.java#L118-L124)。

`ApiTaskManager.submitApiTask` 先找 queue，缺少則400；有queue時 submit、保存 UUID→task，回 **202＋Location**。正常 WebappListener 設context，Location 為 `{context}/api/v1/status/{uuid}`；無 context 的測試分支為 `/status/{uuid}`。來源：[ApiTaskManager.java L88–115](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/ApiTaskManager.java#L88-L115)。WebappListener 建 `projects`、`authorization`、`configuration` 各一個 thread pool，threadCount均1；不同queue可獨立執行，不能假設所有管理動作共用全域序列。見 [WebappListener.java L129–136](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/WebappListener.java#L129-L136)。

ApiTask default 完成 status為200；各 controller 可指定201／204。non-null return object 以 JSON entity回傳，null只建status。task future InterruptedException→500；ExecutionException依**exact exception class** map，未登記預設500，entity為 `ex.toString()`，這和共同 JSON `message` mapper不同。本 inventory 中的9個 submit 呼叫沒有傳自訂 exceptionStatusMap。見 [ApiTask.java L63–95](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/ApiTask.java#L63-L95)、[ApiTask.java L149–188](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/ApiTask.java#L149-L188)。`isDone` 包括正常與例外完成，不能把done當success；`isCompleted` 才只在callable順利回傳後設置。見 [ApiTask.java L120–145](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/ApiTask.java#L120-L145)、[ApiTask.java L190–205](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/ApiTask.java#L190-L205)。

共有9個 ApiTask submit 來源：configuration 3個、projects 6個。suggester rebuild 2條使用 CompletableFuture，沒有 status endpoint可追蹤；projects/indexed task內又另啟 CompletableFuture，task完成不包含該重建完成保證。這些是不同 async契約。

## 6. 全域與 name-bound glue

### 6.1 IncomingFilter：先匹配 allowlist，再看其他方法／filters

此 `@Provider @PreMatching` filter 檢查 **`Authorization: Bearer {token}`**：token需在配置集合，且HTTPS或 `allowInsecureTokens=true`。若沒有滿足token條件，只有 URI path在內建 allowlist才放行；否則401。它比對 path字串，沒有依 HTTP verb分類，也未自行檢查 loopback。request為null的直接return是測試分支，不能當部署模式。見 [IncomingFilter.java L51–74](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/filter/IncomingFilter.java#L51-L74)、[IncomingFilter.java L104–132](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/filter/IncomingFilter.java#L104-L132)。

固定allowlist完整為：`search`、`suggest`、`suggest/config`、`history`、`file/content`、`file/genre`、`file/defs`、`annotation`、`system/ping`、`system/indextime`。**不包含 `list`、`system/version`、projects/groups/repositories/messages/status及suggester管理路徑。** 這解釋上游遠端讀取的差異，但不替Rustgrok批准LAN管理或強制帳號／SSO。

### 6.2 Path、project與CORS

`@PathAuthorized` 綁在 annotation、history、list、file四種閱讀controller的相應methods。PathAuthorizationFilter 以request `path`經FileUtil canonical化後找project，再呼叫上游authorization framework；拒絕403，沒有framework的本體分支false；NoPathParameterException／InvalidPathException在filter被捕捉為406。見 [PathAuthorizationFilter.java L45–84](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/filter/PathAuthorizationFilter.java#L45-L84)。FileUtil對path=null拋NoPathParameterException、空字串／canonical路徑離開source root拋InvalidPathException、不存在拋FileNotFoundException；見 [FileUtil.java L40–70](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/util/FileUtil.java#L40-L70)。

所以「漏path一律400」不成立：method mapper路徑是400，但正常帶PathAuthorized的filter可能先回406。對root本身、symlink、Windows path與canonical前綴的具體HTTP結果須oracle；不由本文改寫安全或巢狀repo政策。

suggest root GET的 `@Authorized` 額外讀 `projects[]`，逐project交原authorization framework，拒絕403；沒有framework時本體不拒絕。見 [AuthorizationFilter.java L38–70](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/suggester/provider/filter/AuthorizationFilter.java#L38-L70)。這與PathAuthorizationFilter的null分支不同，是上游證據，D02的RBAC／ACL排除仍有效。

`@CorsEnable` 出現在search、annotation、history、list、file所有methods，以及suggest根GET／config GET；沒有class級擴張到所有管理路徑。CorsFilter只在有Origin時加 `Vary: Origin`；Origin在配置allowedOrigins才加同值 `Access-Control-Allow-Origin`。沒有在此filter宣告Allow-Methods／Allow-Headers／credentials或自行回應preflight。見 [CorsFilter.java L32–58](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/filter/CorsFilter.java#L32-L58)。CORS不能取代D16主機本機管理的實際可達性控制。

### 6.3 Error response

| 來源型別／分支 | 原始來源可見輸出 |
|---|---|
| GenericExceptionMapper `Exception` | 500、application/json、`{message: e.getMessage()}`。[GenericExceptionMapper.java L33–43](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/error/GenericExceptionMapper.java#L33-L43)、[ExceptionMapperUtils.java L39–55](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/error/ExceptionMapperUtils.java#L39-L55) |
| ValidationException | 400、同JSON message envelope。[ValidationExceptionMapper.java L30–36](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/error/ValidationExceptionMapper.java#L30-L36) |
| Lucene ParseException | 400、同JSON message envelope；provider在suggester package但由RestApp遞迴註冊，沒有method name binding，不能只當suggest專用。[ParseExceptionMapper.java L31–40](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/suggester/provider/ParseExceptionMapper.java#L31-L40) |
| WebApplicationException | 直接傳回 `e.getResponse()`；controller的exception訊息不代表一定已放入response body。[WebApplicationExceptionMapper.java L30–36](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/error/WebApplicationExceptionMapper.java#L30-L36) |
| FileNotFoundException | 404、無entity明訂。[FileNotFoundExceptionMapper.java L31–36](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/FileNotFoundExceptionMapper.java#L31-L36) |
| NoPathParameterException | 400、無entity明訂；PathAuthorizationFilter另可能先攔406。[NoPathParameterExceptionMapper.java L30–35](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/NoPathParameterExceptionMapper.java#L30-L35) |
| filter abort／Response status | 依各body／filter，不能假設套上generic JSON envelope。 |
| ApiTask execution failure | status map預設500；ExecutionException entity為字串，非上述message envelope。見第5.3節。 |

MalformedJSON、參數轉型、框架bean validation、Accept與Content-Type不符、servlet sendError、stream輸出後錯誤、provider precedence的實際headers／bodies／status仍NOT_RUN；不靠Java `throws` 清單直接產生HTTP契約。

## 7. D20待固定契約與驗證缺口

下列是由實際source造成的待驗問題，不是重問已定Rust、D02、D04、D16或平台架構，也不批准JVM產品fallback。

| 主題 | 已有靜態證據 | 尚需固定／驗證 |
|---|---|---|
| D16可達性與副作用 | search GET有條件更新人氣；messages GET在timer缺席時expire；groups POST match不改設定；list閱讀但不在上游allowlist | Rustgrok保留讀者搜尋／瀏覽與本機管理的前提下，明確對照附帶統計／cache維護與管理API；loopback／代理／Origin／Host控制與LAN拒絕須另設計與實測。本文不把search擅改為僅本機。 |
| REST精確URL與參數 | 47 verb/path keys、48 methods、content雙media；search `def/symbol/projects`與suggest `defs/refs/projects[]`不同 | Oracle取得context、slash、escaping、重複query、空值、數字範圍、missing值、content negotiation、HEAD／OPTIONS行為。 |
| DTO／欄位契約 | controller DTO、Message、PathDescription、BeanParam與DTOElement機制已定位 | 完整SuggesterConfig、LookupResultItem、RepositoryInfo property允許矩陣、日期／null／set順序／額外getter序列化；不能把Java type當wire schema已凍結。 |
| errors與非同步 | filters、mappers、submit202、完成status、Location、非合法UUID來源分支已讀 | capture各錯誤body及Content-Type；task retention／重啟／刪除、參數錯誤對同步寫入的影響、無追蹤CompletableFuture、部分刪檔失敗的可觀察結果。 |
| D12已發布搜尋版本 | indexed先set flag、task refresh、另啟suggester；configuration apply與queues各自執行 | 原版相關資料可見性與Rustgrok整批切換契約另由G1／ADR-0003；不把這些局部完成status當generation publish證據。 |
| Q9閱讀來源 | REST file/content讀live file，defs讀index；history／annotation委派HistoryGuru | 與JSP／raw／download的live／SCM分支分開對照；source在索引期間／之後變動的HTTP結果、stream時序待oracle。 |
| D02排除 | authorization/reload與upstream filters實際存在 | 對authorization／RBAC mandatory parity記錄已排除理由；不因此刪非auth reader、group metadata、messages、suggester或管理能力。 |

追溯入口：本文屬 **WP-A05 inventory**；相關既有需求為FR-011／FR-012／FR-013／FR-017及NFR-014，測試規格入口為REF-003、API-001～004、UI-001～006、SRV-001，D16可達性另API-002／SEC-005。這些是既有計畫的關聯入口，未改正式traceability，也沒有把任何test標passed。REST inventory只補方法級來源台帳，並未宣告整個WP-A05、D20、G1或產品完成。

## 8. 本輪receipt與限制

- 唯一新增檔案為本文；逐一讀完13個controller method bodies，對照RestApp、task manager／task／WebappListener、主要filters／error mappers、BeanParam及必要DTO／utility source。
- 靜態計數核對為13 classes／48 HTTP methods／47 verb+path keys；逐controller method數依檔名排序為1、5、1、4、4、1、3、13、1、1、2、7、5。Git blob身份核對只驗來源bytes，**不算產品或HTTP測試PASS**。
- 本文涵蓋此REST package的實際宣告，不涵蓋web.xml servlet、JSP直接URL、metrics servlet、CLI／Python clients、完整UI DOM／assets或所有callee的內部SCM／engine分支；那些不能由本篇REST計數代替。
- 未build、未執行上游source／tests／oracle服務，未送出HTTP請求，未改Issues、commit或push；所有runtime／parity／效能／平台驗收仍 **NOT_RUN**。
