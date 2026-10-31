# OpenGrok 1.14.20：WP-A05 inventory 的列舉入口與計數口徑

日期：2026-11-01。**狀態：STATIC_SOURCE_REVIEW；產品、上游tests、oracle索引／HTTP與視覺驗收均NOT_RUN；有限CLI探測另記。** 本文建立可重跑的完整列舉入口及初步結構計數，不宣稱逐feature相容完成、逐檔授權完成或完整release inventory已驗證。

固定來源：OpenGrok 1.14.20，commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`；Git tree `e332eb280a5362d5ff639c0263acb1e0168b227e`。本機source路徑為 `~/Library/Caches/rustgrok/oracle/opengrok-1.14.20/source`。初輪只核對HEAD／tree；後續HTTP/1.1重試成功，已逐一比對全部1,816檔的Git blob並記錄SHA-256，總11,245,647 bytes；git fsck與clean worktree檢核通過。這不代表release archive或runtime已驗證。

## 1. 可作為完整列舉母表的來源

[opengrok-source-tree.json](../../upstream/opengrok-source-tree.json) 保存 **1,816 tracked entries**，含路徑、mode、object type及Git blob SHA-1；來源是exact commit的 `git ls-tree -rz HEAD`，未用會觸發lazy blob下載的 `-l`。此manifest可用於檢查漏列、分群與upstream路徑追溯；初輪此manifest只有tree身分；後續已補全部blob bytes／SHA-256檢核欄位，**仍不等於license逐檔已讀或runtime相容**。

本輪另成功查官方[recursive tree API](https://api.github.com/repos/oracle/opengrok/git/trees/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3?recursive=1)，回報 `truncated=false`、2,259 tree entries，其中1,816 blobs；與本機tracked列表計數一致。API物件列表及Git blob身份均不是runtime測試。

[官方release metadata](https://api.github.com/repos/oracle/opengrok/releases/tags/1.14.20)列出1個上傳asset：

| asset | API metadata |
|---|---|
| `opengrok-1.14.20.tar.gz` | 92,575,612 bytes；asset ID634373031 |
| SHA-256 | `f33d78f950ba7ed974caa9153120e4f26f9c7f4d63c1f48b7a0350e12d8e346d`，**API回報值，本輪未獨立重算** |
| 來源 | [官方Release 1.14.20](https://github.com/oracle/opengrok/releases/tag/1.14.20)／[asset URL](https://github.com/oracle/opengrok/releases/download/1.14.20/opengrok-1.14.20.tar.gz) |

GitHub自動產生的source tarball／zipball是另一種來源，不能把「1個上傳asset」當成只有一種source下載格式。release的 `target_commitish=master` 也不能取代固定commit；本文source結論只依上述commit。

## 2. Routes與Web行為

### 2.1 Descriptor、JSP與servlet入口

精確入口是：

- `opengrok-web/src/main/webapp/WEB-INF/web.xml`：servlet宣告、mapping、filter mapping、error pages及welcome入口。
- `opengrok-web/src/main/webapp/*.jsp`、`*.jspf`、`WEB-INF/tags/*.tag`：直接JSP URL、include的UI／表單與事件契約；JSP數不等於route數。
- `opengrok-web/src/main/java/org/opengrok/web/GetFile.java`、`PageConfig.java`、`servlet/MetricsServlet.java`：raw/download、request參數／redirect／reading與額外servlet入口。
- `opengrok-indexer/src/main/java/org/opengrok/indexer/web/Prefix.java`、`QueryParameters.java`：URL prefix與UI query參數常數。不可只掃API annotations。

固定[web.xml](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/WEB-INF/web.xml)可按XML元素列出：**12 servlet declarations、13 servlet-mapping rows、6 filter declarations**。13條mapping的pattern為：

`/search`、`/opensearch`、`/history/*`、`/xref/*`、`/raw/*`、`/download/*`、`/s`、`/diff/*`、`/more/*`、`/rss/*`、`/error`、`/enoent`、`/eforbidden`。

這是descriptor列數；`/search`與`/s`有alias關係，不可當13種功能；亦未含所有JSP直接URL、welcome路由或JAX-RS方法。source tree另有 **15 .jsp、8 .jspf、4 .tag**。每種檔案與mapping應保存來源path、servlet/JSP、參數、verb、read／mutation、副作用、status／redirect及對應test。見[PageConfig.java](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/PageConfig.java)、[QueryParameters.java](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/web/QueryParameters.java)。

### 2.2 REST入口

`opengrok-web/src/main/java/org/opengrok/web/api/v1/RestApp.java` 使用 `api/v1` application path，並遞迴註冊 `org.opengrok.web.api.v1`；不能僅靠web.xml判斷REST是否存在。見[RestApp.java L29–39](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/RestApp.java#L29-L39)。

完整source入口目錄：`opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/`，本tree有 **13個controller .java**：Annotation、Configuration、DirectoryListing、File、Groups、History、Messages、Projects、Repositories、Search、Status、Suggester、System（各加 `Controller.java`）。

後續計數須以 `application prefix + class @Path + method @Path + HTTP verb` 作endpoint key；再帶入 `@QueryParam/@PathParam/@HeaderParam/@DefaultValue`、request/response型別與exception mapper。filter／suggester provider及非同步ApiTask亦需掛回同一endpoint；annotation行數不是功能數。入口包括 `api/ApiTask.java`、`ApiTaskManager.java`、`api/error/`、`api/v1/filter/`、`api/v1/suggester/`。本輪**未宣稱13 controllers等於13 endpoints，也尚未完成所有方法展開**。

原版管理API的observable契約仍盤點，但Rust產品可達範圍按D16：管理網頁／有副作用API只限主機本機；LAN搜尋／瀏覽。不因source含authorization filters就加回角色／project ACL需求。

## 3. Queries與analyzers

### 3.1 Query清單不能靠一個field常數數量

必要入口：

| 路徑（相對source root） | 列舉方式 |
|---|---|
| `opengrok-indexer/src/main/java/org/opengrok/indexer/search/QueryBuilder.java` | public setter、query field、field-specific escaping、Boolean組合與parser入口逐項列舉；內部stored fields另列。 |
| 同目錄 `CustomQueryParser.java` | operators、預設AND、query override及Lucene委派邊界；不能採其他引擎defaults當成原語意。 |
| `opengrok-indexer/src/main/jflex/search/{Full,Path,Default}QueryEscaper.lex`、`QueryEscaper.lexh` | 規則及include列舉，不只Java wrapper。 |
| `opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/CompatibleAnalyser.java`、`HistoryAnalyzer.java`及相關tokenizer | 每field分析流程、case、positions／offsets與語言分支。 |
| `opengrok-web/src/main/webapp/search.jsp`、`minisearch.jspf`、API的SearchController、UI QueryParameters | 網頁與API命名、type/filter／sort／paging／project選取及錯誤契約分開。 |
| `suggester/` module與Web的SuggesterController／query parser | autocomplete與query history獨立盤點，不能因主query可用而漏掉。 |

[QueryBuilder.java L54–79](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/QueryBuilder.java#L54-L79)含多個搜尋／stored常數，但 `getSearchFields` 對應list只有full/defs/refs/path/hist；另外仍有type相關constant/setter。因此「5」不是所有查詢能力數，也不是刪除type的依據。field escaping與parser入口見[L458–493](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/QueryBuilder.java#L458-L493)；預設AND見[CustomQueryParser.java L44–46](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/CustomQueryParser.java#L44-L46)。

### 3.2 Analyzer完整註冊入口

主母表是 `opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/AnalyzerGuru.java` 的static `AnalyzerFactory[] analyzers`，不是只搜尋C/Python/JavaScript名稱。該陣列 **54個註冊entries＝47個 `new` expressions＋6個具名factory的 `.DEFAULT_INSTANCE`＋1個 `DEFAULT_ANALYZER_FACTORY`**；排除default變數後才是53個具名factory entries。`DEFAULT_ANALYZER_FACTORY` 本身由 `new FileAnalyzerFactory()` 建立，位於同一陣列且同樣送入 `registerAnalyzer`，不是在53個entries以外另行提供、未經註冊的fallback。另有動態findFactory註冊分支。source tree中 `analysis/**/*AnalyzerFactory.java` 共 **55檔**，包含AnalyzerFactory與FileAnalyzerFactory兩個base，不能當55種語言。見[AnalyzerGuru.java L254–325](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/AnalyzerGuru.java#L254-L325)、[L867–889](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/AnalyzerGuru.java#L867-L889)。

每個factory至少展開names／prefixes／suffixes／magic／Matcher、genre、contentType、hasAnnotations、實際analyzer／xref及Ctags語言映射；registration也直接依這些欄位建map。見[AnalyzerFactory.java L34–70](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/AnalyzerFactory.java#L34-L70)、[FileAnalyzerFactory.java L51–73](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/FileAnalyzerFactory.java#L51-L73)、[AnalyzerGuru.java L411–445](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/AnalyzerGuru.java#L411-L445)。

JFlex來源 `opengrok-indexer/src/main/jflex/` 有 **97 .lex＋52 .lexh**；包含tokenizer、xref與共同include，不是97種語言。archive、binary、image、document等factory亦不可誤分類為程式語言或無聲略過。

已確認優先深測的入口是 `analysis/c/CAnalyzerFactory.java`、`CxxAnalyzerFactory.java`、`analysis/python/PythonAnalyzerFactory.java`、`analysis/javascript/JavaScriptAnalyzerFactory.java`；實際factory map仍須完整展開，不能把C優先解讀成原C++或其他語言被移除。本輪未逐一讀完54個註冊entries（53個具名factory加default）的所有body；JavaScript factory的單次raw取得未完成，未據此捏造suffix／magic清單。

## 4. SCM與必要工具

完整SCM母表：`opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryFactory.java` 的repositories陣列，**15個註冊adapter**，順序及cheap／expensive detection分組均有語意。項目為Git、Mercurial、Repo、BitKeeper、Bazaar、Monotone、Subversion、SCCS、Razor、RCS、CVS、SSCM、AccuRev、ClearCase、Perforce。tree中 `history/*Repository.java` 有16檔，因為含Repository基底，不能當16個SCM。見[RepositoryFactory.java L54–79](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryFactory.java#L54-L79)。

逐adapter以 `isRepositoryFor/isWorking/supportsSubRepositories/isNestable`、history／annotate／revision/diff能力、remote模式、配置、helper command／library、timeout／錯誤處理與platform constraints建矩陣；不能因Git使用JGit或Ctags存在就跳過其餘。Git多repo與nested已另有[固定source研究](2026-10-26-opengrok-nested-git.md)。

工具入口：

- `analysis/Ctags.java`、`CtagsReader.java`、`CtagsValidator.java`、`util/CtagsUtil.java`：Universal Ctags能力、I/O、版本檢查與mapping。
- `analysis/document/MandocRunner.java`、`MandocAnalyzer*.java`：document analyzer工具分支，不能只列Git/Ctags。
- `history/*Repository.java`、`configuration/Configuration.java`、`index/Indexer.java`及各module pom：命令設定、選項與Java依賴區分。

後續的 `--help guru`／repository help與實際helper能力輸出需由獨立oracle取得；本輪不執行，不能將本機預裝工具版本當成已選bundled版本。

## 5. Plugins：先辨識實際框架，避免把檔數當功能數

固定tree的 `plugins/` 共 **49 tracked files、26個src/main/java檔**；其[README](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/plugins/README.md)明確描述authorization plugins與supporting pieces。因此這個module檔數不是非auth功能總數，D02已免除角色／RBAC／project ACL及authorization plugin mandatory parity，不能靠原版有這些class又加回需求。

需同時列舉：

- `plugins/src/main/java/opengrok/auth/`、`plugins/pom.xml`及sample配置：實際implementation、abstract/support class與配置分開。
- `opengrok-indexer/src/main/java/org/opengrok/indexer/authorization/{IAuthorizationPlugin,AuthorizationPlugin}.java`。
- `opengrok-indexer/src/main/java/org/opengrok/indexer/framework/{PluginFramework,PluginClassLoader}.java`：class／JAR discovery及interface驗證邊界，不能只數module的.jar。

框架會列舉jar entries及class names再嘗試載入，source層的interface／supporting code也要留來源與排除理由。見[PluginFramework.java L209–226](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/framework/PluginFramework.java#L209-L226)、[L292–302](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/framework/PluginFramework.java#L292-L302)。

本輪沒有證明「所有可擴充能力只有auth plugins」，也未盤點未來配置下動態載入集合；analyzer assignment／suggester等擴充面按其自身入口另列。Q6無既有客製JAR是使用者輸入，不是免除原內建非auth能力研究或批准Java bridge。

## 6. Assets與release對照

source母表：`opengrok-web/src/main/webapp/` 共 **70 tracked files**。若用副檔名allowlist `.css/.js/.png/.svg/.gif/.woff2` 列靜態資產，得到 **39檔：11 CSS、8 JS、15 PNG、2 SVG、2 GIF、1 WOFF2**。另列manifest.json、META-INF/context.xml、WEB-INF/web.xml及JSP／tags；39不是全webapp，也不是完整WAR內容數。

UI handlers由 `js/utils-0.0.48.js`、`js/diff-0.0.5.js`、`js/repos-0.0.3.js`，以及JSP/JSPF內inline handlers／forms／DOM ids共同構成；其餘第三方JS／CSS不可只因不是自寫handler就刪除。每個asset應保存source path、Git blob、實際bytes SHA-256、license／notice與被哪個template引用；後續已在[部分功能母表](../../upstream/feature-inventory.json)補上39個source assets的實際bytes／SHA-256，仍不足以宣稱逐檔license已完成。

release布局的可查依據是[distribution/assembly.xml](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/distribution/assembly.xml#L6-L65)：

| source/build入口 | 預期release路徑（需實際archive確認） |
|---|---|
| indexer jar | `lib/opengrok.jar` |
| Web WAR | `lib/source.war` |
| plugins jar | `share/lib/plugins.jar` |
| Python tools tar | `tools/opengrok-tools.tar.gz` |
| runtime JAR fileSets | `lib/*.jar` |
| README/LICENSE及logging設定 | `doc/` |

應先對最終tar.gz獨立重算SHA-256，然後列tar members，再分別列WAR中的web assets／JSP／WEB-INF/lib與plugin JAR class entries，對上source母表；source paths、生成class／JFlex輸出及第三方dependency entries分開計數。assembly宣告不是實際release bytes證明；不能從source的70或39推算WAR總檔數。

## 7. 可立即接續的WP-A05工作與追溯

正式[tests/traceability.json](../../tests/traceability.json)目前把WP-A05掛到 **FR-006、FR-008、FR-010、FR-011、NFR-014**。以下補充功能主題的既有Req／Test入口，不修改該映射，也不以WP-ID代替需求或測試ID：

| inventory子表／精確source入口 | 主要Req IDs | 既有Test IDs |
|---|---|---|
| routes/API：web.xml＋RestApp＋13 controllers＋JSP/Prefix/QueryParameters | FR-011、FR-012、FR-013、FR-017、NFR-014 | REF-003、API-001～004、UI-001～006、SRV-001；管理可達邊界另API-002／SEC-005 |
| query：QueryBuilder／CustomQueryParser／3個QueryEscaper.lex／CompatibleAnalyser | FR-001、FR-002、FR-003、FR-022 | QRY-001～006、RST-002、RST-004、RST-007 |
| analyzers：AnalyzerGuru陣列＋54 registered entries（含default）＋JFlex tree＋Ctags入口 | FR-001、FR-004、FR-008、FR-012 | REF-003、SRC-001、RST-003、RST-005、RST-007、UI-002／003 |
| SCM／tools：RepositoryFactory陣列＋15 adapters＋Ctags／Mandoc | FR-006、FR-008、FR-011、NFR-014 | PRJ-005、SCM-001～003、UI-004、REF-003；平台OS-001／WIN-001另驗 |
| plugins／標準配置：plugins module＋framework／authorization＋Configuration | FR-011、FR-020、NFR-010、NFR-014 | REF-003、API-001、RST-011、REL-004；D02排除項記理由，不以skip算PASS |
| suggester：suggester module＋Web SuggesterController／parser/provider | FR-010 | QRY-007、OS-001（現行追溯）；其他source依賴記gap |
| assets：webapp tree＋WAR members＋assembly | FR-012、NFR-009、NFR-010、NFR-014 | REF-003、UI-001～006、RST-005、REL-001／004 |

每列後續可產出 `feature key → source path/line/commit → config／condition → Req IDs → Test IDs → status/gap`；route另附verb/path/schema/status，analyzer附辨識規則，SCM附capability/helper，asset附hash/license。這是接續實作格式建議，不宣稱子表已完整生成。

## 8. 阻擋與本輪receipt

計數修正與自查：最初用只匹配 `new XxxAnalyzerFactory(...)`／`XxxAnalyzerFactory.DEFAULT_INSTANCE` 的pattern，漏掉陣列第一項 `DEFAULT_ANALYZER_FACTORY`；現改為擷取完整陣列、去除註解後逐個comma分隔expression計數。以cache的verified-source-excerpts重新核對Git blob身份：AnalyzerGuru `a6921f776ace01b57e22ad8a586240d1288c38c4`、RepositoryFactory `aaffb5ce5e829647eb7835592260356a4c03c35a`、web.xml `d61c889237f5e0de51070be4fcb5da7612d4521f`，均與tree manifest一致。重新計得54 analyzer entries、15 SCM entries；web.xml改用具namespace的XML parser，仍為12 servlet／13 mappings／6 filters。其餘path-based計數從manifest重算仍相同，沒有用class名稱regex推算語言或feature數。default宣告見[AnalyzerGuru.java L165–167](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/AnalyzerGuru.java#L165-L167)。這些source bytes與結構重核仍不代表runtime PASS。

- **不阻擋結構盤點**：exact tree、主要註冊表與archive assembly已有來源，可以直接生成上述子表；不需重問Rust／Git-only／OpenGrok版本／Windows11 x64等已決事項。
- **後續解除source bytes阻擋，release對照仍待完成**：HTTP/1.1 checkout成功且全部tracked bytes已驗；初輪尚未materialize的13 controllers現已可讀。此筆記的入口盤點沒有因此自動成為完整method或runtime驗收；後續逐項研究另記。
- **阻擋runtime acceptance**：獨立oracle服務、固定JDK／servlet container／helper builds、平台、corpus與配置未實跑；`--help guru`、HTTP／DOM／語言／SCM parity均NOT_RUN。對應WP-A04／WP-A06及後續G1。
- **尚未完成的A05工作**：完整method＋parameter／error展開、54個註冊entries（含default）的所有辨識規則、每SCM capability與helper profile、動態擴充集合、逐asset bytes／source／license對應。
- **非本輪完成宣告**：1,816 entries／54個註冊factory entries（其中53個具名）／15 adapters等是明確口徑的結構計數；不是完整產品功能或逐檔授權已驗證。
- 網路失敗保留：Python urllib因本機CA chain失敗，改用正常TLS的curl；一輪tree API下載逾時未解析成功，不計為證據；使用另一輪完整API回覆與本機exact Git tree交叉核對。沒有關閉TLS驗證。

本輪只新增此筆記；沒有啟動產品、改主控文件、改測試狀態、建立Issue、commit或push。

## 9. 同日後續證據

前述「尚未展開」描述入口調查當時的邊界。後續已完成[54個analyzer註冊列](2026-11-01-opengrok-analyzer-inventory.md)、[13個REST controller的48個HTTP methods](2026-11-01-opengrok-rest-inventory.md)及[15個SCM adapters](2026-11-01-opengrok-scm-capabilities.md)的逐項靜態筆記。source全部bytes與39個指定source assets的SHA-256已保存於母表。這些後續證據仍不完成runtime、動態factory、JSP／UI／query細語意、實際release內容或逐檔授權驗收；整體狀態見[G0執行紀錄](../19-g0-execution-log.md)。


後續OpenGrok release已獨立驗hash，archive／WAR／39個選定source asset byte對照見[實際release inventory](../../upstream/opengrok-release-inventory.json)；`--version`／`--help guru`成功，repos／ctags help因env為null失敗，見[CLI原始證據](../../tests/evidence/oracle-cli-probe-2026-11-01.json)。這些不改變完整runtime與產品測試未通過的邊界。
