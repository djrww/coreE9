# 基準報告(BENCHMARK)

> 儀器:`benches/hotpaths.rs`(零依賴自研計時器;`cargo bench --bench hotpaths`)。
> 語料:種子固定 —— 500 合法程式 + 500 垃圾輸入 + 200 半截檔案(與 fuzz 同宇宙)。
> 單位:ms/樣本。**判定用 best-of-n(各輪最小值)**,median/MAD 僅供診斷;
> 基線:`bench/BASELINE.json`(**同機刷新**,`--update`)。
> CI 容差 ±25% 扣 5% 雜訊下限,並以 `null` 參考內核做環境漂移校正。

## 2026-09-02:gate 重寫(修「main HEAD 長期紅」)

舊做法有三個缺陷,每個都足以讓一個好 commit 被判回歸:

| 缺陷 | 後果 | 修法 |
|---|---|---|
| 基線是**另一台機器單次採樣** | 跨機漂移 20–35% 被當成代碼回歸(`laminar` ×1.26、`named_sexp` ×1.35) | 基線改由同機 `--update` 產生;CI 加 `null` 漂移校正 |
| 只跑 **5 輪**、且用 **median** 判定 | 同一份代碼三趟的 median 趟間變異 cv = **10–24%**(實測),必然 flapping | 改 **best-of-n(min)**:同三趟 cv 降到 **0.3–3.3%**;預設 n=11 |
| 單樣本 ~µs 被整批計時平均 | 計時器解析度不足 | 每輪仍跑整批語料(1000 樣本 ≈ ms 級),把粒度推到 ms |

判定式(每個內核):

```
best = cur_min / base_min                    # ★ 主判据(best-of-n)
corr = cur_null_median / base_null_median    # null = 環境標尺(僅診斷 + 縮放)
eff  = max(2%, tol − floor)                  # floor=5% ⇒ 有效容差 20%
REGRESSED ⇔ best > 1+eff 且 best/corr > 1+eff
median 超標 → 只印「median 超標(環境污染線索)」,不判紅
```

* **為什麼用「抵免」而非「疊加」**:疊加會把 ±25% 擰成 ±36%,合成測試顯示
  真回歸(只有代碼慢 30%)會被放過 —— 那比誤紅更危險。
* **為什麼 median 不判紅**:median 把搶核/降頻的離群值吃進去。同一份代碼跑三趟,
  median 最大比可達 1.56×而 best-of-n 只有 1.06× —— 用 median 判定必然 flapping。
* `null` 只做**診斷 + 縮放**,不做決策放寬:`--relax-on-drift` 曾實作後刪除,
  因為它同樣會吃掉「全內核一起變慢」的真回歸。
* **固有盲区(誠實申報)**:若某 commit 讓所有內核**與 null 一起**變慢(例如
  分配器/全域 flag 變動),比值法無法區分環境。對策是 `--assume-env-stable`
  (關閉校正,只信 raw),或同機 `--update` 重刷基線。
* **判別力自測**:`tools/bench_gate_selftest.py`(9 個合成情境,斷言 exit code),
  已掛進 CI `bench` job 的第一個檢查 —— gate 不只能變綠,必須仍能變紅。

## 現況(沙箱 2 核 x86-64,rustc 1.98.0,`[profile.bench]` fat-LTO opt3;= `bench/BASELINE.json`)

| 內核 | **best(min,判定用)** | median(診斷) | MAD | n × 每輪樣本 |
|---|---|---|---|---|
| `null`(環境參考) | 0.02 µs | 0.02 µs | 0.00 µs | 9 × 1000 |
| `lex`(CL0 詞法) | 0.48 µs | 0.53 µs | 0.03 µs | 11 × 1000 |
| `parse`(全化解析) | 3.91 µs | 4.42 µs | 0.30 µs | 11 × 1000 |
| `laminar`(含 parse) | 8.54 µs | 9.41 µs | 0.30 µs | 11 × 500 |
| `named_sexp`(含 parse) | 8.86 µs | 9.55 µs | 0.42 µs | 11 × 500 |
| `l7b_evaluate`(迭代淨化) | 6.95 µs | 7.19 µs | 0.24 µs | 11 × 200 |
| `r0_lex` | 0.60 µs | 0.63 µs | 0.03 µs | 11 × 1000 |
| `r0_parse` | 2.58 µs | 2.82 µs | 0.18 µs | 11 × 1000 |
| `newman_3x4x6`(2,400 狀態) | 6172.27 µs | 6463.25 µs | 290.97 µs | 5 × 1 |

## 解讀

- parse 全鏈(lex→parse→laminar)單樣本總耗 <0.02 ms:1 萬行級檔案
  (約 400 樣本)的完整結構檢查在秒內 —— L7b 迭代淨化(每輪全析)為
  最重操作,已由 `l7b_evaluate` 基準覆蓋。
- `newman_check` 併行分塊(2 執行緒)後,4 事件 × 6 座標全量
  (623,616 狀態 × 635,424 臨界對)在 ~6.7s 完成 —— 基準僅取 3×4×6
  作回歸信號。

## 常用指令

```bash
# 跑一輪(本機 n=11,newman n=5)
cargo bench --bench hotpaths -- --reps 11 --newman-reps 5 | tee /tmp/bench.log

# 判定(與 CI 同式)
python3 tools/bench_gate.py /tmp/bench.log bench/BASELINE.json --tol 0.25

# 自測 gate 判別力
python3 tools/bench_gate_selftest.py

# 同機刷新基線(發版或換機器/換 rustc 後必做)
python3 tools/bench_gate.py /tmp/bench.log bench/BASELINE.json --update
```

其他旗標:`--noise-floor 0.05`(雜訊帶下限)、`--tiny-us 10`(把 <10 µs 的
內核降為 informational,預設 0 = 全部硬判)、`--strict`(忽略雜訊帶抵免)、
`--assume-env-stable`(停用 null 校正)、`--skip <kernel>`(harness 側)。

## CI 紅線

`cargo bench --bench hotpaths -- --reps 11 --newman-reps 5` → `tools/bench_gate.py
bench/BASELINE.json --tol 0.25`。回歸(扣雜訊、經環境校正後仍 >1.25)判紅,PR
不可合併;基線更新須由 `--update` 產生並附解釋。原始 `bench.log` 一律上傳為
artifact,失敗時可直接看 null 漂移與 MAD,不必重跑。
