# ADR-0001：沿用原版核心與自帶服務（已取代）

狀態：**SUPERSEDED**。建立與取代日期：2026-10-25。原狀態為 PROPOSED，從未升為 ACCEPTED。

取代文件：[ADR-0002：Rust 核心與相容性](0002-rust-core-and-compatibility.md)。

使用者後續明確選擇「我還是想要改寫rust 因為效能比較好」。Java／Lucene／WAR＋私有 Tomcat 包裝因此不再是產品主線；原版部署只保留作 reference oracle。效能收益仍需實測，原功能與 style 完全相同要求繼續有效。

保留本檔原檔名與歷史提案，供追蹤變更及既有連結。下列歷史 Proposal／Alternatives／Consequences 不是現行執行指示、已批准方案或已完成實作。

## Context（歷史）

使用者要求OpenGrok功能與style一致、npm分發、自帶Web Server、跨macOS/Ubuntu24.04/Windows64及內網使用。npm分發並不單獨限制核心語言。

## Proposal（歷史；已取代）

保留原Java/Lucene/ctags與source.war，Node CLI管理固定私有Tomcat distribution與runtime。優先用原版部署方式，最小化patch；不重寫UI/query核心。

## Alternatives（歷史）

外裝runtime profile可減包體但增加環境前提；embedded Tomcat需完整WAR行為驗證；Node全重寫需要重新建立所有parity契約。Docker/WSL未被批准為Windows支持的替代。

## Consequences（歷史）

維持原版行為的證明成本較小；代價是大runtime、二進位平台矩陣、授權、安全更新與程序管理。任意source mapping、Windows行為與index原子性不因採此方案自動解決。

## Acceptance evidence required（歷史；已失效）

v0.1 曾要求 WP-B01–WP-B06、原版與候選的 JSP/query/xref/history/project/API/stop、native Windows 與 airgap tgz 證據。v0.2 的同編號工作包已改成 Rust 工作；不得以本段覆蓋現行[實作計畫](../04-implementation-plan.md)與[驗證計畫](../05-verification-plan.md)。

目前沒有產品 runtime acceptance 證據。此方案已被使用者決定取代，不會因補齊舊測試而自動恢復。

## Decision history

| 事件 | 狀態 | 依據 |
|---|---|---|
| v0.1 提出沿用原版核心 | PROPOSED／OPEN | 當時等待需求決策與技術證據，未獲採用確認 |
| 使用者選擇 Rust 重寫 | SUPERSEDED | 使用者原文與 D01；現行方向見 ADR-0002 |

目前標準 Rust profile 不帶 JVM。日後特定舊 Java plugin bridge 若有需要，屬 D20 新的明確使用者決定、獨立 profile／測試，不恢復本 ADR，也不能取代 Rust 核心。

## 研究依據

- [固定 OpenGrok POM](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/pom.xml)、[Web POM／WAR](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/opengrok-web/pom.xml)。
- [Tomcat 10.1 Introduction](https://tomcat.apache.org/tomcat-10.1-doc/introduction.html)。
- [原始碼研究與 oracle 角色](../01-upstream-research.md)。
