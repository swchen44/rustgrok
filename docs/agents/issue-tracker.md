# Issue tracker: GitHub

本 repo 的工作與規格以 swchen44/rustgrok 的 GitHub Issues 追蹤，使用 gh CLI。

## 操作

- 建立：gh issue create --repo swchen44/rustgrok --title "..." --body-file <file>
- 讀取：gh issue view <number> --repo swchen44/rustgrok --json number,title,body,labels,comments
- 列出：gh issue list --repo swchen44/rustgrok --state open
- 更新標籤：gh issue edit <number> --repo swchen44/rustgrok --add-label "<label>"
- 關閉：gh issue close <number> --repo swchen44/rustgrok

多行內容寫入暫存檔，再使用 --body-file。

## 工作追溯

工作包權威清單為 docs/04-implementation-plan.md。
Issue 保留 WP-ID、需求 FR/NFR IDs、決策 Dxx、測試 IDs、
依賴、產物與實際證據；各類 ID 不互相替代。
依工作類型使用 .github/ISSUE_TEMPLATE/ 中的既有範本。

產品決策同步記入 docs/08-decision-register.md；
符合 ADR 條件的架構決策另記入 docs/adr/。

## Pull requests as a triage surface

PRs as a request surface: no.

## Skills 用語

「publish to the issue tracker」代表建立 GitHub Issue。
「fetch the relevant ticket」代表讀取對應 GitHub Issue。
