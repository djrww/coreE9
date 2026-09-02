cl0r0 v0.1.1 — 發行說明
============================================================
CL0 / R₀ 雙載體 · 機械自証代碼庫(九條定律 L1–L9 + T2 + 編輯單體)

構建
----
- 工具鏈   : rustc 1.98.0 (stable, 2026-08-18)
- profile  : [profile.release] lto="fat", codegen-units=1, opt-level=3
- 優化鏈   : 第一輪 instrumented 構建(-Cprofile-generate)
             → 訓練(19 profraw:fuzz×10 + cl0r0×5 + laws×2 + lib + l9newman 自訓練)
             → llvm-profdata merge(171 KB,291+ 函數熱點)
             → 第二輪 -Cprofile-use 構建(fat LTO 全模塊)
- 二進制   : cl0r0 4.18 MB / fuzz 4.14 MB / l9newman 4.02 MB(含 debug 符號)

驗證矩陣(v0.1.1,全部機械通過)
----------------------------
- cargo test            : 21 passed / 0 failed(九律 L1–L9 + M1/M2/M4/M5 + T2 具名測試)
- cargo clippy -D warnings : 0 警告
- rustfmt --check       : clean
- rustdoc missing_docs  : 0(全量文檔,見 docs/)
- fuzz(種子 0xC1020240001): 總失敗數 0
- l9newman(CommutativeTrim/Guarded):
    10800 狀態窮舉 / 3102 臨界對 / L8 違反 0 / 不可回合 0
    → SN ∧ WCR ⇒ CR ⇒ 唯一正規形(機械驗證通過)
- l9newman(Naive/Raw 對照):L8 違反 39 / 不可回合 33 —— 如實申報反例

文件
----
- cl0r0      : 九律演示(樹性質 / 具名投影 / 三軌 liveness / 修法菜單)
- fuzz       : 屬性測試自動機(可重現種子)
- l9newman   : 機械 Newman 通道(§4.2–4.3)
- SHA256SUMS : 二進制校驗和
- docs/      : rustdoc 全量文檔(52 頁)

============================================================
「律不過,碼不合」—— 本發行的每一個二進制都通過具名測試矩陣。
