# 規劃文件的檢查與封裝

此工具使用Python標準函式庫，驗證目前v0.6的38項需求、44個工作包與80項測試規格，產生完整Markdown合訂本、ZIP、SHA-256 manifest及品質報告。它不執行Rust產品測試，也不產生可執行產品。

在repository根目錄執行：

```sh
python3 tools/build_planning_package.py
```

輸出放dist；可用--output-dir指定repository之外的交接目錄。repository內只允許dist作輸出，避免生成檔被再次包入自身。git資料、build輸出與cache不進規劃ZIP。

檢查涵蓋JSON解析、UTF-8／隱形控制字元、Markdown相對連結、需求與測試雙向對照、44個工作包覆蓋與依賴無循環、決策ID及已確認狀態、ZIP與檔案hash一致。產品測試仍必須按tests/matrix.json實作與執行，不能把此工具成功當作產品驗收。

未來若需求、測試數量或版本變更，需連同此規劃版的固定契約更新；不自動把planned改成passed。
