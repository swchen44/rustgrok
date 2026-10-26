# Rustgrok 授權範圍

政策確認日期：2026-10-26，決策D17／Q14。此repo採逐檔來源與授權管理，沒有將整個產品或現有規劃包統一改為MIT。

| 內容 | 已接受政策 |
|---|---|
| 可辨識為獨立原創的新增Rust／JS程式碼 | 採[MIT](https://opensource.org/license/mit)；標明實際權利人、適用條款及provenance。尚無產品程式碼，不能僅依副檔名認定原創。 |
| 沿用、修改或移植的OpenGrok內容 | 固定基線1.14.20預設[CDDL 1.0 only，個別檔案例外](https://github.com/oracle/opengrok/blob/3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3/LICENSE.txt)。依實際內容保留適用條件、原notice、修改者及必要source交付，不因翻成Rust改掛MIT。 |
| 其他第三方元件／資產／工具 | 保持各自適用條件，依實際採用及分發內容建立來源、license、notices、SBOM與source對照。 |
| 現有規劃文件、Python工具或尚未分類內容 | Q14僅決定新增獨立原創Rust／JS；不以此自動重新授權其他既有檔案。 |

逐檔台帳需保存origin URL、固定commit／hash、原授權／notice、修改與對應source。實際發行前須隨包提供適用授權全文、notices與所需來源；本政策不是完整component inventory或artifact授權檢查已完成的證據。

決策及來源見[決策表](docs/08-decision-register.md)與[命名／授權查核](docs/research/2026-10-26-naming-and-license-boundaries.md)。
