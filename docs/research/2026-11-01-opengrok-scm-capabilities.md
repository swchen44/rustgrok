# OpenGrok 1.14.20：15個SCM adapter能力與依賴

日期：2026-11-01。固定commit：`3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3`。**STATIC_SOURCE_REVIEW；oracle runtime、商用SCM環境、原生平台、CLI／Java library與Rust相容測試均NOT_RUN。** 本文讀取完整15個註冊adapter，並追讀Repository／HistoryGuru及必要中間基底與RCS/SCCS helper；不是逐個SCM已實際可用的證明。

來源為 `~/Library/Caches/rustgrok/oracle/opengrok-1.14.20/source`。完整source下載／blob核對由主控本輪完成；本筆記另將所讀來源與[固定tree manifest](../../upstream/opengrok-source-tree.json)逐檔核對Git blob身份。註冊母表為 [RepositoryFactory.java L54–79](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryFactory.java#L54-L79)，共15 entries；不是Git-only。

## 1. 閱讀矩陣前的共同契約

- **發現不等於working**：factory先檢查enabled、巢狀時isNestable，再isRepositoryFor、建立instance與setDirectoryName；isWorking=false時記warning，但這段不因此直接丟掉已辨識instance。之後繼承project／configuration的history、cache、rename、merge等設定。見 [RepositoryFactory.java L185–266](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryFactory.java#L185-L266)、[RepositoryInfo.java L408–433](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryInfo.java#L408-L433)。
- **History**：HistoryGuru先找檔案／目錄所屬repo、試history cache；fallback才檢查historyEnabled、isWorking、fileHasHistory及remote/UI條件，亦受PathAccepter與fetchHistoryWhenNotInCache影響。不能把這些前置或cache命中全壓成一個SCM布林值。見 [HistoryGuru.java L425–443](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/HistoryGuru.java#L425-L443)、[HistoryGuru.java L532–598](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/HistoryGuru.java#L532-L598)。
- **Annotation**：先試annotation cache；fallback另檢查非目錄、index中的genre/analyzer annotations能力、isWorking、fileHasAnnotation及path接受條件，再呼叫adapter.annotate。這條路徑並非直接重用historyEnabled的全部gate。見 [HistoryGuru.java L252–308](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/HistoryGuru.java#L252-L308)、[HistoryGuru.java L740–808](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/HistoryGuru.java#L740-L808)。
- **Revision**：HistoryGuru.getRevision找所屬repo，再呼叫getHistoryGet；Repository提供File／InputStream wrappers，但核心OutputStream版本由adapter覆寫。無repo回null／false；不能假設每個adapter失敗都用同一種status。見 [HistoryGuru.java L611–632](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/HistoryGuru.java#L611-L632)、[Repository.java L233–266](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/Repository.java#L233-L266)。
- **Diff**：各adapter沒有統一的public getDiff契約。Web的文字diff由兩側path/revision各取內容，再由PageConfig使用Diff.diff計算；下文「R／D」表示原始碼存在revision retrieval，因而有Web pair-content差異的來源路徑，不表示執行各SCM的diff CLI或所有檔案／revision均成功。見 [PageConfig.java L303–378](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/src/main/java/org/opengrok/web/PageConfig.java#L303-L378)及[既有閱讀研究](2026-10-26-opengrok-source-view-consistency.md)。
- **繼承預設**：Repository.supportsSubRepositories=false、isNestable=false；一般getHistory(file,since)是全量歷史後裁切。只有明確override者才能宣稱其他行為。目錄history=false會使Repository.createCache不走整repo目錄history產生路徑，但不代表檔案history不存在。見 [Repository.java L166–194](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/Repository.java#L166-L194)、[Repository.java L385–403](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/Repository.java#L385-L403)、[Repository.java L500–516](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/Repository.java#L500-L516)。
- **Command resolution**：先Configuration repoCmd，以class canonical name為key；未設定則system property／fallback command。checkCmd只驗指定命令exit0，不是server登入、repo可讀、所有子命令及版本語意皆符合。見 [Repository.java L550–581](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/Repository.java#L550-L581)。

表中的H為fileHasHistory、A為fileHasAnnotation、HD為hasHistoryForDirectories的**實際source條件**；true只表示候選資格。N寫作「可向內掃描／自身可被nested發現」，仍受depth、nestingMaximum及factory類型順序控制。多repo/project歸屬參見[巢狀研究](2026-10-26-opengrok-nested-git.md)。

## 2. Git／Mercurial／Repo與本機歷史系統

### GitRepository

| 項目 | 固定source行為 |
|---|---|
| 繼承／發現／N | extends RepositoryWithHistoryTraversal；候選目錄直接有存在的.git即可（directory或submodule .git file）。N=true／true。 |
| H／A／HD | 三者均回true；未檢查每個檔案tracked狀態。isWorking無條件true，並有bare檢查TODO，不是repo有效性驗證。 |
| History／annotation | RevWalk／HistoryCollector；file path filter、rename-follow、merge設定、tags與per-partes cache各有分支。BlameCommand指定startCommit、followFileRenames、WS_IGNORE_ALL；revision=null先找該path最近commit。 |
| R／D | JGit TreeWalk取blob；失敗且尚未寫出內容時有rename原名fallback。不能用裸git show/blame預設當等價。 |
| 依賴 | adapter直接使用JGit，沒有git CLI command requirement；Rust後端仍未選定。 |

來源：[GitRepository.java L99–118](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java#L99-L118)、[GitRepository.java L138–318](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java#L138-L318)、[GitRepository.java L330–454](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java#L330-L454)、[GitRepository.java L508–588](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java#L508-L588)、[GitRepository.java L733–752](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/GitRepository.java#L733-L752)。JGit版本見[indexer pom L237–240](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/pom.xml#L237-L240)：`7.2.1.202505142326-r`。Git/Mercurial的多參數getHistory與cache中間層來自 [RepositoryWithHistoryTraversal.java L103–123](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryWithHistoryTraversal.java#L103-L123)；不是把所有邏輯歸到直接Repository base。

### MercurialRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N | .hg directory；自身isNestable=true；向內掃描預設true，但property或environment `org.opengrok.indexer.history.mercurial.disableForest` 可停用。 |
| H／A／HD／working | H/A/HD=true；working以lazy／cached `hg`（無arguments）exit0判斷，並非逐檔驗history存在。 |
| History／annotation | extends RepositoryWithHistoryTraversal；log模板、branch revset、--follow／--no-merges及range有專門分支，file與directory的since/till處理不同。annotation用hg annotate模板，並需要HistoryGuru回傳history以建立author映射；拿不到history可回null。 |
| R／D | `hg cat -r`；失敗未輸出時追log --follow原名後重試。 |
| 依賴 | `org.opengrok.indexer.history.Mercurial`／fallback `hg`；CLI與版本輸出語意、branch／rename／merge需實跑。 |

來源：[MercurialRepository.java L57–120](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/MercurialRepository.java#L57-L120)、[MercurialRepository.java L188–276](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/MercurialRepository.java#L188-L276)、[MercurialRepository.java L289–441](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/MercurialRepository.java#L289-L441)、[MercurialRepository.java L454–598](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/MercurialRepository.java#L454-L598)。

### RepoRepository（Android repo容器，不是Git adapter的別名）

| 項目 | 固定source行為 |
|---|---|
| 發現／N／working | .repo directory；N=true／false（isNestable繼承base）；isWorking設command後直接true，不測repo executable。 |
| H／A／HD | 全false。 |
| History／R／D／annotation | getHistory、getHistoryGet、annotate皆直接throw UnsupportedOperationException；parent／branch／currentVersion回null。用子Git repositories提供history，不能將repo容器列成完整history/blame實作。 |
| 依賴 | property `org.opengrok.indexer.history.repo`、fallback `repo` 被設定，但該adapter沒有用它實作上述操作；容器辨識本身不證明需要執行repo。 |

來源：[RepoRepository.java L38–122](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepoRepository.java#L38-L122)。

### BitKeeperRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N | .bk directory；N=false／false繼承base。 |
| H／A／HD／working | H執行 `bk files basename` 並要求stdout trim後等於basename；A呼叫H；HD=false。working執行--version並解析版本。 |
| History／annotation | 明確覆寫getHistory(file,since)，用log dspec與-r範圍；用檔案revision，而非自行改成changeset ID。annotate -aur，可帶-r；非零status丟IOException。 |
| R／D | get -p，可帶-r；exit非零回false。 |
| 依賴／特例 | property `org.opengrok.indexer.history.BitKeeper`／`bk`；tags依版本7.3選不同dspec，buildTagList卻直接加字串bk而非RepoCommand。這是source差異，不能默默修正後稱oracle相同。 |

來源：[BitKeeperRepository.java L57–152](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/BitKeeperRepository.java#L57-L152)、[BitKeeperRepository.java L219–344](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/BitKeeperRepository.java#L219-L344)、[BitKeeperRepository.java L357–434](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/BitKeeperRepository.java#L357-L434)。

### BazaarRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N／working | .bzr directory；N=false／false繼承base；`bzr --help` exit0。 |
| H／A／HD | H/A/HD=true；H不是逐檔tracked探測。 |
| History／annotation | 覆寫getHistory(file,since)：log，directory加-v，since組-r range，tags另建；blame --all --long，可帶-r，非零status丟IOException。 |
| R／D | cat -r；走Runtime.exec及process stream／cleanup，不是共同Executor路徑，這段沒有同樣的exit-code驗證。 |
| 依賴 | property `org.opengrok.indexer.history.Bazaar`／`bzr`；實際CLI版本、退出與timeout行為待驗。 |

來源：[BazaarRepository.java L51–95](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/BazaarRepository.java#L51-L95)、[BazaarRepository.java L99–222](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/BazaarRepository.java#L99-L222)、[BazaarRepository.java L229–264](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/BazaarRepository.java#L229-L264)。

### MonotoneRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N／working | _MTN directory；N=false／false繼承base；`mnt --help` exit0。 |
| H／A／HD | H/A/HD=true。 |
| History／annotation | 覆寫getHistory(file,since)，log --to、--no-graph／--no-merges／--no-format-dates；annotate可帶-r，quiet選項依deprecated property分支。 |
| R／D | cat -r的Executor/output stream實作；是否能在實際Executor生命週期取得內容仍需native oracle，不由source構造推成成功。 |
| 依賴／特例 | `org.opengrok.indexer.history.Monotone`／`mnt`；`org.opengrok.indexer.history.monotone.deprecated`預設false，其getQuietOption回傳單一字串 `--quiet --quiet`；不自行改成兩個argv。 |

來源：[MonotoneRepository.java L54–116](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/MonotoneRepository.java#L54-L116)、[MonotoneRepository.java L128–208](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/MonotoneRepository.java#L128-L208)。

### SubversionRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N | .svn directory；N=false／false繼承base；檔頭明示nested repositories尚未支援，不能把每層.svn都當應自動巢狀發現。 |
| working／remote | lazy `svn --help`；setDirectoryName另跑info --xml，解析URL/root並決定working；URL非file或缺失可標remote。僅help可用不足以保證setDirectoryName後working。 |
| H／A／HD | H/A/HD=true；H有尚未廉價驗tracked的TODO。 |
| History／annotation | 覆寫since overload與last-current metadata路徑；log --non-interactive --xml -v，branch/range及rename目錄ls有分支。annotate --xml／-r；使用配置中的SCM認證參數。 |
| R／D | cat --non-interactive -r；檔名加@作peg-revision escape，成功status才回true。 |
| 依賴 | `org.opengrok.indexer.history.Subversion`／`svn`；working copy、repository／server與配置認證需oracle。SCM server認證不等於加回產品RBAC。 |

來源：[SubversionRepository.java L60–105](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/SubversionRepository.java#L60-L105)、[SubversionRepository.java L174–287](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/SubversionRepository.java#L174-L287)、[SubversionRepository.java L350–460](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/SubversionRepository.java#L350-L460)。

## 3. SCCS／RCS／CVS與Razor

### SCCSRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N | Codemgr_wsdata或小寫codemgr_wsdata directory，或SCCS directory；N=false／false繼承base。 |
| H／A／HD | H查SCCSHistoryParser對應s-file存在；A直接true；HD=false。A=true不能當未版本控制檔也能annotate。 |
| History／annotation | getHistory交SCCSHistoryParser；未覆寫since overload，沿base全量後裁切。annotation先prs取得authors，再get -m -p，可帶-r。 |
| R／D | SCCSget以sccs get -p取revision。 |
| 依賴／working | `org.opengrok.indexer.history.SCCS`／`sccs`；先help help，失敗再--version。需原生SCCS實作／格式驗證，不把不同平台版本預設視為相同。 |

來源：[SCCSRepository.java L56–194](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/SCCSRepository.java#L56-L194)、[SCCSget.java L34–52](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/SCCSget.java#L34-L52)。

### RCSRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N | RCS directory至少有一個名稱符合.*,v的entry（source刻意未逐entry驗regular file）；N=false／false繼承base。 |
| H／A／HD | H查RCS/basename,v存在；A委派H；HD=false。 |
| History／R／D | history用RCSHistoryParser／JRCS Archive；revision用RCSget／Archive.getRevision；不是呼叫rcs/co CLI。since未override，沿base裁切。 |
| Annotation／working | annotate用外部blame，可帶-r；isWorking仍以blame command exit0決定，因而HistoryGuru gate可能使內部Java history也受其working值影響。 |
| 依賴／source特例 | property `org.opengrok.indexer.history.RCS.blame`／`blame`；isBlameWorking的getCommand第一參數實際是MercurialRepository.class，而instance ensureCommand以RCS class運作。只記原始差異，不假定配置永遠無影響。 |

來源：[RCSRepository.java L54–109](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RCSRepository.java#L54-L109)、[RCSRepository.java L113–197](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RCSRepository.java#L113-L197)、[RCSHistoryParser.java L72–101](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RCSHistoryParser.java#L72-L101)、[RCSget.java L31–76](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RCSget.java#L31-L76)。JRCS rcs/diff在[indexer pom L179–187](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/pom.xml#L179-L187)均為0.4.2。

### CVSRepository

| 項目 | 固定source行為 |
|---|---|
| 繼承／發現／N | extends RCSRepository；但發現override為CVS directory，N=false／false繼承Repository。 |
| H／A／HD／working | H/A override回true；HD未override，繼承RCS=false；working override用cvs --version，不沿RCS blame gate。 |
| History／annotation | CVSHistoryParser與cvs log；branch設定存在時用-r1.1:branch，否則-b。annotation override為cvs annotate，revision或branch各有參數。since仍沿Repository全量後裁切，不能誤認沿RCS parser做history。 |
| R／D | override為 `cvs up -p -r`，revision含冒號先裁前段；不使用RCSget。這是輸出到stdout的up -p，不代表授權產品更新checkout。 |
| remote／依賴 | CVS/Root第一行不以/開頭則remote=true；`org.opengrok.indexer.history.cvs`／`cvs`。getRCSFile仍有本機CVS資料映射override，但上述H／history／R均不能據此簡化成「只需JRCS」。 |

來源：[CVSRepository.java L47–137](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/CVSRepository.java#L47-L137)、[CVSRepository.java L172–290](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/CVSRepository.java#L172-L290)。

### RazorRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N／working | .razor directory（可由symlink指向group）；N=false／false繼承base。constructor設working=true，isWorking繼承RepositoryInfo，不測vendor CLI。 |
| H／A／HD | H查映射到History的檔案；A要求映射RCS或SCCS archive是檔案；HD=false。 |
| History／annotation | RazorHistoryParser直接解析映射metadata；annotate按archive型態委派新RCSRepository或SCCSRepository，無符合者回null；rename／delete有TODO。不能把委派存在當所有映射／CWD已可用。 |
| R／D | 先試Archive/BINARY中的@rev.Z，實際用GZIPInputStream；再試RCSget，最後SCCSget。source明示UNIX compress解碼TODO，不能把.Z副檔名視為已支援標準compress。 |
| 依賴／環境 | 真正Razor group/archive可讀與唯讀checkout＋.razor映射；JRCS／GZIP或sccs／blame分支。不是執行razor log/annotate CLI。商用archive取得、格式與symlink平台條件需另外驗證。 |

來源：[RazorRepository.java L75–131](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RazorRepository.java#L75-L131)、[RazorRepository.java L150–216](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RazorRepository.java#L150-L216)、[RazorRepository.java L223–347](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RazorRepository.java#L223-L347)、[RepositoryInfo.java L316–326](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryInfo.java#L316-L326)。

## 4. 需vendor workspace／server的adapters

下列是**需實際vendor工具／workspace／server與必要授權或測試環境才能驗證的群組**；不是宣告其所有client都收費或可以再分發。精確商用授權、免費額度、再散布權與平台build未在本輪查證，D19隨包不會自動賦予這些權利。

### AccuRevRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N／working | directory→isInAccuRevDepot：以accurev info解析Depot、Basis、Top並保存；已有depotName時直接假定在depot。N=false／false繼承base；working是info exit0。 |
| H／A／HD | H/A/HD=true。 |
| History／annotation | AccuRevHistoryParser；hist對檔案加-k keep，path轉depot-relative。annotate -fvu，可帶-v；since未override，沿base裁切。 |
| R／D | 先stat -fe取得element ID，再cat -v revision -e ID；不是直接以檔名cat，移動／evil twins等source註解有邊界。 |
| 依賴／前置 | `org.opengrok.indexer.history.AccuRev`／`accurev`；constructor設定command。檔頭明示服務host需有效AccuRev登入；info parser檢查not logged in。workspace／depot、symlink及server狀態必須實跑，不能由info成功外推所有權限可用。 |

來源：[AccuRevRepository.java L44–105](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/AccuRevRepository.java#L44-L105)、[AccuRevRepository.java L108–234](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/AccuRevRepository.java#L108-L234)、[AccuRevRepository.java L260–382](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/AccuRevRepository.java#L260-L382)、[AccuRevRepository.java L429–456](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/AccuRevRepository.java#L429-L456)。

### ClearCaseRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N | view.dat／.view.dat存在，或directory名為vobs，或canonical path等於cached cleartool lsvob -s列表（ignore case）。N=false／false繼承base。 |
| H／A／HD／working | H/A/HD=true；working是cleartool -version。vob列表另外cached，不能當每次自動refresh。 |
| History／annotation | lshistory，directory加-dir，自訂-fmt；ClearCaseHistoryParser；since未override，沿base裁切。annotation用annotate -nheader -out - -f -fmt，revision依source作獨立argv。 |
| R／D | get -to temp `filename@@rev`，檢查status，再讀temp及清理。 |
| 依賴 | `org.opengrok.indexer.history.ClearCase`／`cleartool`；須有效view/VOB、vendor client與server／license環境；command -version不代表VOB內容已可讀。 |

來源：[ClearCaseRepository.java L55–195](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/ClearCaseRepository.java#L55-L195)、[ClearCaseRepository.java L199–272](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/ClearCaseRepository.java#L199-L272)。

### PerforceRepository

| 項目 | 固定source行為 |
|---|---|
| 發現／N | isInP4Depot；directory先p4 dirs *，未命中再files，stdout含//判為存在。N=false／false繼承base。 |
| H／A／HD／working | H/A/HD=true；lazy `p4 -V`，只驗client命令，discovery另需連到workspace/depot。 |
| History／annotation | PerforceHistoryParser，since overload明確override；annotation `p4 annotate -qci`，內部rev轉成@changeset。%、#、*、@有專用filename escape。 |
| R／D | `p4 print -q`＋@revision；source保留properly evaluate return code的TODO，不能以boolean true當所有p4 failures均被辨識。 |
| 依賴 | `org.opengrok.indexer.history.Perforce`／`p4`；workspace mapping、server、tickets／權限、vendor client再分發及原生platform build待驗。 |

來源：[PerforceRepository.java L55–150](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/PerforceRepository.java#L55-L150)、[PerforceRepository.java L160–265](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/PerforceRepository.java#L160-L265)。

### SSCMRepository（Surround SCM）

| 項目 | 固定source行為 |
|---|---|
| 發現／N／remote | .MySCMServerInfo regular file；N=false／false繼承base；constructor明設remote=true。 |
| H／A／HD／working | H=true，HD=false；A解析.MySCMServerInfo底部對應filename的version，必須可解析為整數且>1。working是sscm version。 |
| History／annotation | SSCMHistoryParser＋明確since overload；history帶SCMBranch／SCMRepository；數字since組-v後續範圍。annotate帶-b／-p及-aV:revision，再解析輸出。 |
| R／D | sscm get下載至temp directory，帶-b/-p、-v、-q/-tmodify/-wreplace；檢查status後讀檔並清理。 |
| 依賴 | `org.opengrok.indexer.history.sscm`／`sscm`；vendor client、server與.MySCMServerInfo內容／格式、認證和原生build待驗。HD=false不能因getHistoryLogExecutor含directory分支就改寫為true。 |

來源：[SSCMRepository.java L61–174](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/SSCMRepository.java#L61-L174)、[SSCMRepository.java L178–313](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/SSCMRepository.java#L178-L313)、[SSCMRepository.java L345–366](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/SSCMRepository.java#L345-L366)。

## 5. 繼承差異與條件的交叉核對

| adapter群組 | 實際差異 |
|---|---|
| Git、Mercurial | 直接extends RepositoryWithHistoryTraversal，再往上是RepositoryWithPerPartesHistory／Repository；history traversal／collector及chunked cache與其他13種不同。僅這兩者覆寫isMergeCommitsSupported=true。 |
| CVS | extends RCSRepository；但命令、working、discovery、file flags、history、annotation、revision retrieval皆有override。HD=false與預設nested flags仍繼承。 |
| Razor | 未覆寫isWorking；constructor設定true後沿RepositoryInfo讀值。不能錯列為通過某個CLI探測。 |
| 其他一般Repository子類 | nested兩flag預設false；部分覆寫since history（Bazaar、Monotone、BitKeeper、Subversion、Perforce、SSCM），其餘沿base全量再裁切。Repo則full history方法就throw，不能假稱有可用incremental fallback。 |
| capability=true占位者 | 多個adapter的H／A直接true，只代表「讓操作試著執行」；不是查證每個檔案在SCM內。SSCM、BitKeeper、RCS、SCCS、Razor的file-specific條件需獨立測。 |

繼承及預設的來源是各class宣告／override與 [RepositoryInfo.java L136–162](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryInfo.java#L136-L162)、[Repository.java L166–194](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/Repository.java#L166-L194)、[Repository.java L500–516](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/Repository.java#L500-L516)、[RepositoryWithHistoryTraversal.java L43–123](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-indexer/src/main/java/org/opengrok/indexer/history/RepositoryWithHistoryTraversal.java#L43-L123)。

**remote不是所有network I/O的完整標記**：CVS由Root判斷、Subversion由info URL判斷、SSCM constructor設定；其他adapter可能透過vendor client聯絡server但未在constructor設定remote。因此不能把HistoryGuru RemoteSCM gate當成所有外網／內網連線的完整防線，也不在本文擅改它。

## 6. D19與驗證阻擋

- **D19已決必要helper隨包，精確工具/build未決**。原版JGit／JRCS僅描述oracle實作，沒有批准Rust產品帶JVM。替換為Git CLI、git2、gix或自製RCS parser都仍需同fixture differential；不得把command有同名功能就當等價。
- **需vendor環境的項目**：AccuRev的登入workspace／depot、ClearCase view/VOB、Perforce server/client mapping、SSCM metadata/server、Razor group/archive與格式。client授權／可再分發條件、可用原生OS tuple及測試帳號尚無實證，不先刪掉這些能力。
- **所有SCM均需原生runtime驗證**：先macOS arm64，後Ubuntu24.04 amd64完整原生重驗，Windows11 x64亦需自身證據；現有或歷史工具是否有對應build／CLI輸出、locale、編碼、特殊路徑、timeout及錯誤行為為UNKNOWN。
- **D02沒有產品RBAC**不等於SCM server不需登入或票證；來源讀取權與產品讀者／管理角色系統分開。D16本機限定管理邊界亦不由SCM capabilities變更。
- **未讀完／未驗的邊界**：本文完整讀15個adapter檔，但沒有宣稱所有HistoryParser／AnnotationParser與第三方library implementation皆完整逐行審查；僅對RCS/SCCS必要helper補查。其他parser精細語法、null／空輸出／錯誤分支的實際結果、商用格式兼容、binary diff皆待oracle。
- **既有追溯**：WP-A05／WP-B05／WP-D03；FR-006、FR-008、FR-011、NFR-014。既有REF-003、PRJ-005、SCM-001～003、UI-004、OS-001／WIN-001繼續使用，沒有新增Test IDs或把planned改passed。

## 7. 本輪receipt

只新增本筆記；不執行SCM executable／Java產品，不安裝、不建立Issue、不commit／push、不改其他inventory或machine manifest。固定source的可讀／hash核對是來源證據，**不等於SCM runtime或Rust產品PASS**。
