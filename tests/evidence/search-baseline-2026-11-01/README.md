# 固定原版搜尋基準：兩次公開小樣本執行

2026-11-01 在 macOS 15.7.7／24G720、arm64，以 OpenGrok 1.14.20／固定 commit、Temurin JDK 21.0.13+9、Tomcat 10.1.60 及本機 Universal Ctags 6.2.1 執行兩次。兩個 project 下五個 Git repository，包含一個普通巢狀 Git；C fixture 同時出現在兩個 project，且有固定 rename history。完整 JDK archive 為 199,754,598 bytes，SHA-256 `bf036b35b071bfb2f54f2c0af8ee8d88afa95ee6eb3dce02c0f94cfc2524d04a`；`javac` 為 Mach-O arm64。各 run 的 result 記載其他工具、config、輸入及 raw log hashes。

每個 run 保存原版全文、路徑、定義、參照及跨 project 搜尋的原始 JSON、Git manifest、實際設定與原始 log。兩次 corpus manifest 完全相同，五個查詢均 HTTP 200；兩輪後均確認 corpus 未改變。原版 index log 各顯示找到五個 Git repo，包括 `alpha/component-c/nested-js`；此結論僅限此配置及佈局。

[repeatability.json](repeatability.json)保留兩次原始hash、兩次有序路徑及比較。只排除 JSON 最上層可變的 `time` 後，五個查詢的命中內容相同；**全文 alpha、定義 alpha、跨 project 全文三個查詢的結果路徑順序不同**，另外兩個順序相同。原因尚未確認，API 沒有暴露 score；不能假定同分，不能排序掩蓋或宣稱排序已通過。所有原始 responses 未更動。第二輪刻意提供無效 `GIT_DIR` 及不可用 proxy，工具隔離這些環境；這不是產品網路安全驗收。

Index log 顯示 `p4` 不可用、native-access／vector 警告；Tomcat 關閉時記錄 background resource releaser 與 JGit／Chronicle ThreadLocal 警告。兩次程序均已結束；Perforce、其他 SCM 及 shutdown 的完整行為尚未驗證。

此 corpus 僅2 project／5 Git repo／3 種優先語言，不是約500 MB代表工作量，不涵蓋全部 query、語言、SCM、多層巢狀、submodule、merge、UI 或產品相容／效能驗收。沒有把 REF-002、PRJ-001、SRC-001 整項標 passed。

重跑需預先取得並驗證本機原版 release、JDK、Tomcat 與 Ctags；工具會再次核對 archives、OpenGrok JAR／WAR、JDK java／javac 及 public fixture bytes。選兩個**不存在**的目錄，例如：

```sh
python3 tools/build_public_corpus.py --output /tmp/rustgrok-public-corpus
python3 tools/capture_opengrok_baseline.py --corpus /tmp/rustgrok-public-corpus --output /tmp/rustgrok-oracle-capture
```

Oracle 固定只綁 loopback；inputs、index、runtime與證據輸出只使用指定目錄。完整JDK與相關工具僅供原版參照，不成為Rust產品依賴。要檢查可重現性，在兩個新的輸出目錄各跑一次；只排除最上層 `time` 仍須保留 `results` 的原始 key 次序與所有匹配行次序，差異必須保留。

[第三次原始擷取](run-3/result.json)用最後版本工具與第一輪相同corpus，在無效GIT_DIR／不可用proxy下重跑五項HTTP查詢，均為200且事後corpus不變；各次有序路徑及差異保留於repeatability紀錄，未依結果排序。這只驗工具及固定原版的有限案例。

後續依審查補強：`run-verified-1`／`run-verified-2` 在啟動前逐一核對 Tomcat 已解壓 `bin`、`conf`、`lib` 共74個檔案與官方已驗 archive 的bytes及owner executable mode，並拒絕額外檔案／symlink。第二輪另以無效 Git、proxy 與四項 Java options 環境重跑；兩輪各五項HTTP 200、事後 corpus未變。有序路徑及原始輸出均保留在兩輪目錄與repeatability紀錄，除最上層 `time` 以外不做其他正規化；JDK archive及java／javac本體已驗，JDK其餘解壓library尚未逐檔覆核。

範圍界線：原版 Ctags 的每個預設 option file／有效外部設定尚未完整盤點；已記錄二進位hash／版本及 OpenGrok XML，但不同主機重跑前仍需核對 Ctags 實際設定。這裡保存HTTP狀態及原始body，沒有保存完整HTTP response headers；兩者都不代表完整HTTP契約已驗收。
