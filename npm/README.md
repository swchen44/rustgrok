# npm 分發規劃

正式方案見 [分發與離線設計](../docs/06-packaging-and-offline.md)。候選 npm/cli 為薄 JS 入口；npm/platforms 產生各 OS/CPU 的預編譯 Rust binary 套件。各平台直接 native bin 也可作實驗候選。

此處尚無 package.json、tgz 或正式 scope/name，不建立可誤發布的占位套件。npm 負責安裝與啟動，搜尋、索引與server在Rust。使用者自行發布npm。離線ZIP直接執行native binary，與是否安裝Node分開驗收。

D19已選A：對應平台payload需含必要Ctags／SCM工具及完整依賴；不把公司已裝工具當默認前提，不隱性連外下載。精確工具版本／能力仍在G1驗證。

發行包放使用者指定新repo rustgrok的GitHub Releases；npm registry發布仍由使用者執行。repo名稱不自動決定npm scope/name。先在Mac完整驗證，再Ubuntu24.04 amd64重驗；完整依賴閉包不能只靠Release上並列TGZ宣稱完成。
