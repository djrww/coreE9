# 基準報告(BENCHMARK)

> 儀器:`benches/hotpaths.rs`(零依賴自研計時器;`cargo bench --bench hotpaths`)。
> 語料:種子固定 —— 500 合法程式 + 500 垃圾輸入 + 200 半截檔案(與 fuzz 同宇宙)。
> 單位:ms/樣本(中位數 × 5 輪;`newman_3x4x6` 為 ms/次)。
> 基線:`bench/BASELINE.json`;CI job `bench` 對照 ±25%(跨環境寬容)。

## 現況(沙箱 2 核 × x86-64,rustc 1.98.0,release fat-LTO)

| 內核 | ms/樣本 | 說明 |
|---|---|---|
| `lex` | 0.0005 | CL0 詞法(平鋪,trivia 保留) |
| `parse` | 0.0038 | CL0 全化解析(含 ERROR 回收) |
| `laminar`(含 parse) | 0.0080 | 樹層狀檢查 |
| `named_sexp`(含 parse) | 0.0077 | 具名投影(§1.3) |
| `l7b_evaluate` | 0.0067 | 迭代淨化(§2.3) |
| `r0_lex` | 0.0006 | R₀ 詞法 |
| `r0_parse` | 0.0029 | R₀ 解析(節點級 unsupported) |
| `newman_3x4x6` | 7.85 ms/次 | 3×4×6 空間(2,400 狀態)機械 Newman 檢查 |

## 解讀

- parse 全鏈(lex→parse→laminar)單樣本總耗 <0.02 ms:1 萬行級檔案
  (約 400 樣本)的完整結構檢查在秒內 —— L7b 迭代淨化(每輪全析)為
  最重操作,已由 `l7b_evaluate` 基準覆蓋。
- `newman_check` 併行分塊(2 執行緒)後,4 事件 × 6 座標全量
  (623,616 狀態 × 635,424 臨界對)在 ~6.7s 完成 —— 基準僅取 3×4×6
  作回歸信號。

## CI 紅線

`cargo bench --bench hotpaths` → `tools/bench_gate.py bench/BASELINE.json --tol 0.25`。
超出 ±25% 視為回歸,PR 不可合併(基準檔更新需附解釋)。
