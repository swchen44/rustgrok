# 公開 corpus 來源與授權

這三份檔案原樣取自 OpenGrok 1.14.20，commit `3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3` 的 `opengrok-indexer/src/test/resources/sources/`，僅作公開搜尋測試輸入，並非企業 source。保留原版 [LICENSE.txt](LICENSE.txt)、[NOTICE.txt](NOTICE.txt) 及各檔案適用的來源條件；不能把複製的內容改稱獨立原創 MIT。

| 原始相對路徑 | SHA-256 |
|---|---|
| `c/main.c` | `0afb5b45d1538ee86e23b046100c8e458614ba71e50f16c05bda1a52de0edb17` |
| `python/main.py` | `c30bc720a38c39709e22f6bba1996b58c0a4e2e25d8cc6ea1bff7a348efde3f0` |
| `javascript/function.js` | `7e37130f0b5a078fea055a885632cdc00e1989def53e4fafe8b52a75824f8805` |

原始 Python fixture 以 `python/main.py.fixture` 保存在此目錄，建 corpus 時還原為 `main.py`；它含 Python 2 語法，C fixture 含分析器邊界語法；本測試只索引文字，不執行或編譯這些檔案。此語料含一組普通巢狀Git；更多語言版本、Unicode、多層巢狀、submodule、merge 與其他 SCM 尚未被此小樣本涵蓋。

[固定 corpus manifest](expected-corpus-manifest.json) 是本目錄受版控的可信 corpus 定義，記錄每個 repository 的 source hash、tree、HEAD 與完整 commit chain。建置器驗證生成結果符合該定義；oracle 擷取器不可只信任輸入資料夾自行提供的 manifest。
