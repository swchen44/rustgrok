# 來源索引與研究方法

版本：v0.2；查核日期：2026-10-25（Asia/Taipei）。

使用者已選擇 Rust 核心重寫。OpenGrok／Java／Tomcat 來源用來建立原版參照環境（reference oracle）；Tantivy、Tree-sitter 與其他 Rust 技術仍是研究候選。本索引不代表已批准產品依賴，也不代表已建置或執行產品。各章的 Sxx／Oxx／Txx 是該章自己的來源編號。

本研究採官方上游原始碼、官方版本資料與元件維護者文件。固定 commit 的檔案適合重現本輪判斷；wiki、latest 文件及 branch URL 可能變動，選版與發行時須重新核對。

## 1. 研究版本與讀取紀錄

| 對象 | 固定來源 | 角色與限制 |
|---|---|---|
| OpenGrok 1.14.20 | commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`；[release API](https://api.github.com/repos/oracle/opengrok/releases/tags/1.14.20)、[tag API](https://api.github.com/repos/oracle/opengrok/git/ref/tags/1.14.20) | 功能／UI 研究 oracle；D04 正式驗收版本未批准 |
| Lucene 9.12.3 | commit `f965e930673c1f5cb478dc6a8907f5fc4ef7b539`；[BM25Similarity](https://github.com/apache/lucene/blob/f965e930673c1f5cb478dc6a8907f5fc4ef7b539/lucene/core/src/java/org/apache/lucene/search/similarities/BM25Similarity.java) | 對照原版引擎語意，非 Rust 產品相依 |
| Tantivy 0.27.0 | commit `67889269cae67063c372bc236b102d632a021568`；[tag API](https://api.github.com/repos/quickwit-oss/tantivy/git/ref/tags/0.27.0)、[annotated tag](https://api.github.com/repos/quickwit-oss/tantivy/git/tags/a22c1b184e861d1065b964bb45f013ef8945dae4) | 候選引擎靜態研究，尚未選為產品依賴 |

Tantivy 研究時的 releases/latest 與 tag／docs.rs 最新標示並不一致，因此採明確 tag 與 commit。Cargo.toml 宣告的 Rust 1.88 是該研究版本的最低 Rust 版本，不是本產品已選 toolchain。

機器可讀紀錄：

- [OpenGrok research lock](../upstream/research-lock.json)：原版 release／commit／API 回報的 asset metadata，角色為 reference oracle。
- [Rust research lock](../upstream/rust-research-lock.json)：Tantivy／Lucene pin、額外 OpenGrok 讀碼、文件範圍與尚未決定的產品欄位。
- [Source review manifest](../upstream/source-review-manifest.json)：取回的 OpenGrok 文字範圍與本地文字 hash。

原始碼透過 GitHub 內容介面讀取。本地文字 hash 不等於 upstream archive／registry crate／binary hash。檔案樹盤點不等於讀過全部內容，更不等於完整原始碼審核。

## 2. OpenGrok 固定原始碼

下列檔案均固定於研究 commit，詳細讀碼範圍及個別來源見 [01](01-upstream-research.md) 與 [12](12-rust-compatibility.md)。

| 官方來源 | 支持的範圍 |
|---|---|
| [根 POM](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/pom.xml)、[Indexer POM](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/pom.xml)、[Web POM](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/pom.xml) | modules、Java／Lucene／JGit、WAR／JSP、Windows 排除測試 |
| [Build workflow](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/.github/workflows/build.yml) | 上游 OS／Java CI 矩陣；不能代替本產品驗收 |
| [CustomQueryParser](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/CustomQueryParser.java)、[QueryBuilder](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/QueryBuilder.java) | AND、大小寫、escaping 與欄位 Boolean 組合 |
| [CompatibleAnalyser](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/CompatibleAnalyser.java)、[PathTokenizer](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/PathTokenizer.java)、[HistoryAnalyzer](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/HistoryAnalyzer.java) | 欄位 tokenization／normalization／positions |
| [PlainFullTokenizer.lex](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/jflex/analysis/plain/PlainFullTokenizer.lex)、[PlainSymbolTokenizer.lex](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/jflex/analysis/plain/PlainSymbolTokenizer.lex) | 識別字、數字與標點規則 |
| [SearchEngine](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/search/SearchEngine.java) | 多 project MultiReader、count、relevance／path／date sort |
| [Ctags](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/Ctags.java)、[JFlexXref](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/JFlexXref.java) | Ctags 協定、symbols 與 xref HTML |
| [GitRepository](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java) | JGit history／blame、rename 與 whitespace comparator |
| [Project](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/configuration/Project.java)、[Configuration](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/configuration/Configuration.java)、[IndexDatabase](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/index/IndexDatabase.java) | sourceRoot、XML、commit／locking；非 Rust 已實作機制 |
| [web.xml](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/WEB-INF/web.xml)、[SearchController](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/controller/SearchController.java)、[IncomingFilter](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/api/v1/filter/IncomingFilter.java) | 路由、UI／REST 參數與 token gate |
| [help.jsp](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/help.jsp)、[search.jsp](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/search.jsp)、[style](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/default/style-1.0.6.css)、[utils JavaScript](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/webapp/js/utils-0.0.48.js) | 原 UI、DOM、資產與互動；JSP 動態部分須移植 |
| [LICENSE.txt](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt)、[NOTICE.txt](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/NOTICE.txt) | 預設 CDDL 1.0 only、檔案例外與 notices；發行物仍須逐件核對 |

## 3. OpenGrok 動態官方文件

| 來源 | 用途與限制 |
|---|---|
| [Setup](https://github.com/oracle/opengrok/wiki/How-to-setup-OpenGrok) | 原版 runtime、Ctags、WAR、專案與設定；舊範例以 pinned source 校正 |
| [Per-project workflow](https://github.com/oracle/opengrok/wiki/Per-project-management-and-workflow) | 原 project／index／configuration 工作流程 |
| [Authorization](https://github.com/oracle/opengrok/wiki/Authorization) | AuthN／AuthZ 與 plugins inventory |
| [User Interface](https://github.com/oracle/opengrok/wiki/User-Interface) | 功能盤點，不代替固定版本實跑 |
| [Languages and Formats](https://github.com/oracle/opengrok/wiki/Supported-Languages-and-Formats)、[SCM support](https://github.com/oracle/opengrok/wiki/Supported-Revision-Control-Systems) | 語言／SCM inventory，不自行縮成常用子集 |
| [Testing](https://github.com/oracle/opengrok/wiki/Testing) | 上游測試方法，須與 POM profiles 核對 |

history.jsp、diff.jsp、更多 analyzers／SCM 尚未全部逐一分析。只確認路徑存在的檔案，不得記為已讀碼。

## 4. Rust 搜尋與分析候選

Tantivy 原始碼固定於本輪研究 commit；列入來源不表示採用。

| 官方來源 | 用途 |
|---|---|
| [Tantivy README](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/README.md)、[Cargo.toml](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/Cargo.toml) | 研究版 metadata、宣告 license／MSRV／features、benchmark 限制 |
| [QueryParser](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/query_parser/query_parser.rs)、[query grammar](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/query-grammar/src/query_grammar.rs) | 預設 OR、regex 設定、語法與 adapter 研究 |
| [tokenizer manager](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/tokenizer/tokenizer_manager.rs)、[Token API](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/tokenizer-api/src/lib.rs) | default tokenizer 長度 filter、UTF-8 byte offsets |
| [BM25](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/bm25.rs)、[fieldnorm](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/fieldnorm/code.rs) | 欄位統計與計分／normalization 對照 |
| [RegexQuery](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/regex_query.rs)、[FuzzyTermQuery](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/fuzzy_query.rs)、[PhraseQuery](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/query/phrase_query/phrase_query.rs) | API 能力；不等於 Lucene 語法／命中／排序相容 |
| [IndexRecordOption](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/schema/index_record_option.rs)、[format constants](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/lib.rs)、[index metadata](https://github.com/quickwit-oss/tantivy/blob/67889269cae67063c372bc236b102d632a021568/src/index/index_meta.rs) | postings／positions 與自身 index 格式，非 Lucene index 相容證明 |
| [Lucene 9.12.3 classic syntax](https://lucene.apache.org/core/9_12_3/queryparser/org/apache/lucene/queryparser/classic/package-summary.html)、[BM25 API](https://lucene.apache.org/core/9_12_3/core/org/apache/lucene/search/similarities/BM25Similarity.html) | 版本化 query／scorer 參考，另加 OpenGrok 自訂行為 |
| [Tree-sitter introduction](https://tree-sitter.github.io/tree-sitter/) | CST、incremental parsing、runtime／bindings；僅文件研究，未鎖版本或 grammar |
| [Ctags JSON output](https://docs.ctags.io/en/latest/man/ctags-json-output.5.html) | JSON 協定與 libjansson build 條件；尚未決定採用 |
| [Universal Ctags](https://github.com/universal-ctags/ctags)、[Windows builds](https://github.com/universal-ctags/ctags-win32) | 後續固定 build／hash／license 的官方起點，D19已確認隨包策略，具體build待驗 |
| [Git log](https://git-scm.com/docs/git-log)、[blame](https://git-scm.com/docs/git-blame)、[diff](https://git-scm.com/docs/git-diff) | CLI 候選；不能推論與原版 JGit 等價 |

## 5. Rust runtime、分發與驗證

以下已查閱文件多為動態官方文件／維護者來源，精確產品版本與 build target 待選型後固定。文件 06 與 05 提供各項推論、限制及驗證計畫。

| 來源 | 用途 |
|---|---|
| [Rust platforms](https://doc.rust-lang.org/rustc/platform-support.html)、[Windows MSVC](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)、[macOS targets](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html) | target／minimum OS 查核，cross-compile 不是 native 驗收 |
| [Rust linkage](https://doc.rust-lang.org/reference/linkage.html) | native／system library、CRT 與動態相依 |
| [Cargo vendor](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html)、[build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)、[profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) | 離線建置、native build 與 release profile |
| [Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) | 候選 runtime 的工作排程／取消限制，尚未選版 |
| [npm package.json](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/) | bin、os/cpu、optionalDependencies、bundleDependencies |
| [npm install](https://docs.npmjs.com/cli/v11/commands/npm-install/)、[config](https://docs.npmjs.com/cli/v11/using-npm/config/)、[install-scripts](https://docs.npmjs.com/cli/v11/commands/npm-install-scripts/) | tarball、offline、ignore-scripts 與生命週期政策 |
| [cmd-shim](https://github.com/npm/cmd-shim/blob/main/lib/index.js)、[bin-links](https://github.com/npm/bin-links/blob/main/lib/link-bins.js) | native bin 的 source 依據；branch URL 可變，blob 身分記於 06，仍須測精確 npm 版本 |
| [Node child_process](https://nodejs.org/api/child_process.html) | 薄 launcher 的 argv／stdio／exit 與 Windows signals |
| [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/) | 發布／provenance 候選，npm 仍由使用者發布 |
| [Playwright snapshots](https://playwright.dev/docs/test-snapshots) | 同 OS／browser／fonts 的 UI 比較 |
| [OpenJDK JMH](https://openjdk.org/projects/code-tools/jmh/)、[Criterion inputs](https://bheisler.github.io/criterion.rs/book/user_guide/benchmarking_with_inputs.html) | 微基準方法，不能代替端到端效能 |
| [JVM 穩態量測研究](https://arxiv.org/abs/2209.15369) | warmup／steady-state 假設的原作者研究 |
| [Rust regex complexity](https://docs.rs/regex/latest/regex/struct.Regex.html) | 候選 regex 成本，不代表 Lucene regexp 相容 |
| [Rust profiling](https://nnethercote.github.io/perf-book/profiling.html)、[heap allocations](https://nnethercote.github.io/perf-book/heap-allocations.html) | 原作者效能書的方法，不是本案實測結果 |
| [Miri](https://github.com/rust-lang/miri)、[Rust sanitizers](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html) | 驗證工具候選及平台／toolchain 限制 |

## 6. 原版 oracle 與歷史包裝研究

| 官方來源 | v0.2 角色 |
|---|---|
| [Tomcat 10.1 Introduction](https://tomcat.apache.org/tomcat-10.1-doc/introduction.html)、[Server component](https://tomcat.apache.org/tomcat-10.1-doc/config/server.html) | 原版 oracle 容器配置／生命週期 |
| [Tomcat embedded API](https://tomcat.apache.org/tomcat-10.1-doc/api/org/apache/catalina/startup/Tomcat.html) | ADR-0001 歷史候選，不再是產品主線 |
| [WebappListener](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/WebappListener.java) | 原版 Apache Tomcat 10 檢查與 readiness |

Oracle 的 Java／Tomcat exact builds 尚未固定或實跑。目前標準 Rust profile 不帶 JVM。特定舊 plugin 的 Java bridge 只屬 D20 未批准候選；若日後需要，須新的明確使用者決定、獨立 profile 與測試，且不能取代 Rust 核心。

## 7. 證據狀態與授權界線

1. **使用者決定**：D01 已確認 Rust；原功能與 style 要求維持。
2. **來源事實**：固定 source 與官方 metadata 的靜態研究，讀取範圍有限。
3. **設計提案**：相容 parser／engine adapter、templates、SCM adapter、generation 與 npm 包型。
4. **待執行驗證**：oracle／Rust runtime、differential、三平台、離線與效能全部仍為 NOT_RUN。

CDDL 敘述依 OpenGrok LICENSE.txt 與來源檔紀錄，作為來源盤點及發行工程依據。不能僅由「改成 Rust」推論全產品適用 MIT，也不能在內容未定時宣稱授權符合性已完成。複用／翻譯／修改／獨立新增檔應分別保留 provenance、notices、適用條款與 source 對應，正式發行核對實際 inventory。

本輪 release digest 是 GitHub API 回報值，尚未下載重算；沒有完整 clone／build／run 或產品 benchmark 證據。D19已確認必要工具隨包、D21已確認搜尋優先；工具build、D20相容邊界、代表環境與數值SLO仍待決。

## v0.4新增：Ubuntu版本核對

使用者先指定Ubuntu24.04，中途寫24.02，已在最新回覆明確確認「24.04」。v0.4時CPU尚未指定；v0.5已確認Ubuntu24.04 amd64部署及macOS先開發／完整驗證。[Canonical官方發行列表](https://releases.ubuntu.com/)與[Ubuntu release cycle](https://ubuntu.com/about/release-cycle)提供24.04 LTS及24.04.x發行資料；實際point release隨測試環境記錄，不再列為本輪需求阻擋。查核日期：2026-10-25。此查核不是產品執行證據。

## v0.5新增：GitHub Releases、可見性及平台名稱

查核日期2026-10-25。以下官方資料用來設計交付與平台對照，不是產品測試結果。

| 來源 | 本輪核實事項與用途 |
|---|---|
| [GitHub About releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases) | tag、Release附件、repo read/write access、自動source archives；每Release最多1000 assets，每檔小於2 GiB。用於06/07/16與release manifest模板 |
| [GitHub About repositories](https://docs.github.com/en/repositories/creating-and-managing-repositories/about-repositories) | Public／Private可見性選擇；v0.6使用者已指定Public並完成建立 |
| [Rust平台支援](https://doc.rust-lang.org/rustc/platform-support.html) | x86_64 Linux與macOS targets存在；Mac架構於v0.6確認M1／arm64；具體toolchain、GNU/musl與最低OS仍按產品驗證選型，不以Rust平台級別代替產品PASS |

v0.5建立表單只備妥尚未送出；v0.6使用者確認Public與M1後已完成建立。GitHub metadata回讀：swchen44/rustgrok、id1433282584、public、main，push／admin可用。README首個commit為6764a8efa3f4764133c397fc40fc71cafa330444。完整規劃已提交為526f619089703f710c915af303e54aea7d25226e，46個blob逐一核對一致；後續交接收據更新另依Git history追蹤。沒有產品Release或runtime驗收證據。
