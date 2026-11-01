# G0 執行紀錄

更新：2026-11-01。使用者要求「status and continue」後，繼續不受 OPEN 項阻擋的來源與 oracle 工作。實驗範圍依 [G0／G1 基線](18-g0-g1-entry-review.md)，其餘決策維持原狀。

## 工作與追蹤

| WP | GitHub Issue | 狀態與完成邊界 |
|---|---|---|
| WP-A04 | [#1](https://github.com/swchen44/rustgrok/issues/1) | in_progress；固定上游實際下載、hash 與授權／資產台帳。Rust 候選未凍結，完整逐檔 provenance 未完成 |
| WP-A05 | [#2](https://github.com/swchen44/rustgrok/issues/2) | in_progress；固定 source tree 的 inventory 入口與列舉口徑；尚非完整 feature→Rust→test 結案 |
| WP-A06 | [#3](https://github.com/swchen44/rustgrok/issues/3) | in_progress；獨立 Mac arm64 oracle bootstrap；完整 golden、token／AST 診斷與多平台證據尚未完成 |
| WP-A01 | [#4](https://github.com/swchen44/rustgrok/issues/4) | needs-info；Q15管理網頁具體操作集合待答，其餘已決事項不重問 |

## 執行順序與共用產物

1. WP-A04 固定 source commit／tree、下載 bytes／hash 與來源授權文件；WP-A05 依同一 source 身分列舉，不改用最新分支。
2. WP-A06 只執行已驗 hash 的 OpenGrok、Java 及 servlet container；Ctags／SCM 記錄實際路徑、版本與 binary hash。與 Rust 產品的工具選型分開。
3. WP-A07 的完整 deterministic corpus 與 WP-A08 的差異 harness 仍在後續。臨時 bootstrap 輸入不冒充代表工作量、正式 corpus 或 golden。

目前 80 項完整測試規格仍 planned；下載檢核、靜態 inventory 或有限 oracle smoke 結果分別記錄，不能把部分前置步驟標成 REF-001／REF-002／REF-003 整項 passed。

## 已取得的證據

- [固定 source tree](../upstream/opengrok-source-tree.json)：commit 與 tree 身分核對，1,816 tracked entries；包含 Git blob SHA-1，後續完整checkout成功，全部1,816檔已逐一比對Git blob並保存SHA-256，總11,245,647 bytes，git fsck通過。
- [部分功能母表](../upstream/feature-inventory.json) 與 [盤點筆記](research/2026-11-01-opengrok-inventory-entrypoints.md)：12 servlet 宣告、13 mapping rows、13 REST controller 檔、54 analyzer 註冊 entries、15 SCM adapter entries、39 個指定副檔名的 source assets。54 包含 default FileAnalyzerFactory；這些計數不等於語言數、endpoint 數或已完成相容的功能數。
- [Oracle 取得紀錄](../tests/evidence/oracle-acquisition-2026-11-01.json) 與 [獨立 oracle lock](../upstream/oracle-artifacts.lock.json)：Tomcat 10.1.60 的 14,435,753 bytes archive 已重算 SHA-512 並與 Apache 公布值一致，LICENSE／NOTICE hash 已保存。六個固定 source 摘錄已核對 Git blob 身分，含上游 LICENSE／NOTICE；完整逐檔授權仍未完成。
- Compact Temurin21.0.13+9 JRE的47,935,740 bytes archive已驗官方SHA-256，實際`java --version`／`--list-modules` exit0；binary為Mach-O arm64，且沒有`jdk.compiler`。這只完成工具取得與版本探測，該JRE階段當時仍缺完整JDK；後續JDK已驗archive、java／javac並執行有限原版索引／HTTP，見下述後續證據。
- 已逐項展開 [54 analyzer entries](research/2026-11-01-opengrok-analyzer-inventory.md)、[REST methods](research/2026-11-01-opengrok-rest-inventory.md) 與 [15 SCM adapters](research/2026-11-01-opengrok-scm-capabilities.md)。REST 共有48個HTTP-annotated Java methods、47個verb/path keys；`file/content`兩種media variants共用route key。這些均為靜態source結果；UI、query細語意、工具build／再散布及runtime另驗。
- [有限 bootstrap 輸入](../tests/evidence/oracle-bootstrap-inputs-2026-11-01.json)：2個project、3個Git repo、3份未修改的公開上游C／Python／JS fixture，總2,418 source bytes；固定Git身分／時間的一次初始commit，保留每份origin及hash。已完成有限索引探測，尚未HTTP查詢，沒有rename／merge／nested完整驗收，不是WP-A07正式corpus或約500 MB的代表工作負載。

初次完整 source checkout 發生 `HTTP/2 stream 3 was not closed cleanly: CANCEL`、`early EOF`，HTTP/1.1 重試已成功，source阻擋解除；release archives另列下載狀態。大型 archive 下載速度慢，改為保留分段已取得 bytes 續傳；未完成整檔 hash 核對前不解壓執行。這是取得資料的阻擋，沒有修改 D04 基線或放寬驗收。

## 接續與未決事項

本輪開始前的local HEAD為 `234044d`；新的docs／metadata提交另以Git history核實。取得產物使用者目錄為 `~/Library/Caches/rustgrok/oracle/`：

- `opengrok-1.14.20/source/` 是已完整驗byte hash的上游checkout，detached在D04 commit；`verified-source-excerpts/`保留初輪六份摘錄。
- `toolchains/apache-tomcat-10.1.60/` 來自已驗官方SHA-512的archive；OpenGrok／compact JRE／完整JDK archives均已完成整檔檢核，以[oracle lock](../upstream/oracle-artifacts.lock.json)的expected bytes／hash為準，不能依檔案存在或parts總量判定成功。
- 本輪下載工作使用cache內的 `acquire-ranges.py` 與 `acquire-jdk-ranges.py`；`parts/{opengrok,jre,jdk}/` 保留續傳資料。接手時先確認現有下載程序及檔案狀態，避免同一個parts／archive有兩個writer；程序、tool session ID與cache均不是正式依賴鎖。
- 原版[Dockerfile](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/Dockerfile#L109-L113)有 `jdk.compiler` module flags，並使用 `tomcat:10.1.60-jdk21`。完整oracle以固定JDK21為前提；先取得的較小JRE僅供版本／取得探測，不藉此假稱完整suggester或compiler-based token／AST診斷可用。完整JDK archive及java／javac已驗hash並用於有限原版查詢；其餘解壓library未逐檔驗hash。
- 完整JDK與loopback原版索引／HTTP已用固定公開corpus取得有限基準；CLI／guru原始輸出及失敗已保存，原CLI失敗是否在完整JDK下重現仍待核對。不要以小輸入、版本輸出或部分頁面成功取代REF-001／REF-002整項驗收；沒有建立Rust核心或產品發行物。

Q15已向使用者提出本機管理網頁的具體操作集合，尚未收到答覆；**保持OPEN，不採用預選項當答案**。D16的主機本機管理／LAN搜尋瀏覽範圍已確定，不重問；Q15不阻擋獨立oracle或靜態功能盤點。

## 儲存與發布邊界

大型 oracle archives、source checkout、JDK／Tomcat、索引及本機執行 logs 位於 `~/Library/Caches/rustgrok/oracle/`。repo 只保存公開 metadata、必要小型原始輸出與重跑方式，不含企業 source／index／secrets。此 cache 可移除後重新取得，不屬於產品 runtime 或分發內容。

本輪尚未 push、建立產品 Release 或發布 npm。既有遠端提交的歷史 receipt 保留；後續來源查核不宣稱修改已同步遠端。

## 已驗 release 與有限 CLI 探測

[Release inventory](../upstream/opengrok-release-inventory.json)記錄OpenGrok archive的92,575,612 bytes／SHA-256，與官方`f33d78f950ba7ed974caa9153120e4f26f9c7f4d63c1f48b7a0350e12d8e346d`一致；88個tar檔案均有實際hash。WAR有300個members／273個files、113個`WEB-INF/lib/*.jar`；39個選定source assets與WAR內bytes全部相同，這不證明DOM／UI已相容。

[CLI probe](../tests/evidence/oracle-cli-probe-2026-11-01.json)使用已驗的compact JRE：`--version`及`--help guru`exit0，版本回報1.14.20與D04 commit；`--help repos`／`ctags`exit1，原始stderr顯示`Indexer.env`在`exitWithHelp`為null，隨後shutdown亦失敗。成功及失敗的stdout／stderr均保存於[原始輸出](../tests/evidence/oracle-cli-2026-11-01/guru.stdout.txt)同目錄，未修改原版。完整JDK重核仍待完成，不能把失敗歸因於JRE或把部分CLI成功外推為完整oracle、SCM或analyzer驗收。

原版archive未含名為`NOTICE.txt`的member；來源root NOTICE另有已驗bytes。這只是inventory觀察，完整依賴／授權、source對應及應交notices仍待逐項確認；沒有因此省略NOTICE或宣告發行授權通過。

## 2026-11-01 限定 JRE 索引探測

[原始證據](../tests/evidence/oracle-index-probe-2026-11-01.json)：以已驗OpenGrok release、Temurin 21 JRE及本機Universal Ctags 6.2.1，對公開2 project／3 Git repo／2,418 bytes輸入執行原版索引器，exit 0；log顯示3個repository、兩個project完成索引及寫入設定。輸入的3個Git HEAD與worktree事後均未變動。記錄的warning含native-access及vector module；此執行未驗HTTP、查詢結果、完整JDK或代表規模，不能標REF-001／REF-002或產品測試passed。

## 2026-11-01 限定 JRE Web 探測

[原始證據](../tests/evidence/oracle-web-jre-probe-2026-11-01.json)：使用相同小型索引，Tomcat只綁定127.0.0.1；首頁、alpha搜尋API及raw檔案均HTTP 200。搜尋`main`回傳C與JavaScript兩個路徑，raw內容SHA-256與公開fixture相符。程序結束後listener已關閉。

Tomcat log顯示`ClassNotFoundException: com.sun.tools.javac.api.JavacTool`，兩個project的most popular completion被停用；關閉時另有resource-releaser thread與JGit ThreadLocal警告。這次JRE缺`jdk.compiler`，不能當完整oracle驗收；本段記錄JRE時的事實；後續完整JDK有限搜尋基準已另行擷取，人氣建議依使用者要求暫緩驗收。

## 後續完整 JDK 與 G1 搜尋實驗

完整JDK 21.0.13+9 archive為199,754,598 bytes，SHA-256 `bf036b35b071bfb2f54f2c0af8ee8d88afa95ee6eb3dce02c0f94cfc2524d04a`，與官方metadata一致；java及javac為macOS arm64並已用於[有限原版基準](../tests/evidence/search-baseline-2026-11-01/README.md)。JDK其他解壓library尚未逐檔核對。

Issue #6的[實驗與差異](../tests/evidence/search-spike-2026-11-01/README.md)記錄公開corpus七個query、原版score/explain及Rust候選；包含不相容的排序結果。它不構成完整oracle或產品驗收。
