# 基準報告(BENCHMARK)

> 儀器:`benches/hotpaths.rs`(零依賴自研計時器;`cargo bench --bench hotpaths`)。
> 語料:種子固定 —— 500 合法程式 + 500 垃圾輸入 + 200 半截檔案(與 fuzz 同宇宙)。
> 單位:ms/樣本。**判定用 best-of-n(各輪最小值)**,median/MAD 僅供診斷;
> 基線:`bench/BASELINE.json`(**同機刷新**,`--update`)。
> CI 容差 ±25%:名目扣 5% 雜訊下限,再按本趟污染度**單向**放寬(≤10pp);
> `null` 參考內核的漂移須先通過顯著性門檻才用於校正(2026-09-03 修,見下)。

## 2026-09-02:gate 重寫(修「main HEAD 長期紅」)

舊做法有三個缺陷,每個都足以讓一個好 commit 被判回歸:

| 缺陷 | 後果 | 修法 |
|---|---|---|
| 基線是**另一台機器單次採樣** | 跨機漂移 20–35% 被當成代碼回歸(`laminar` ×1.26、`named_sexp` ×1.35) | 基線改由同機 `--update` 產生;CI 加 `null` 漂移校正 |
| 只跑 **5 輪**、且用 **median** 判定 | 同一份代碼三趟的 median 趟間變異 cv = **10–24%**(實測),必然 flapping | 改 **best-of-n(min)**:同三趟 cv 降到 **0.3–3.3%**;預設 n=11 |
| 單樣本 ~µs 被整批計時平均 | 計時器解析度不足 | 每輪仍跑整批語料(1000 樣本 ≈ ms 級),把粒度推到 ms |

判定式(每個內核;2026-09-03 修,見「健檢 P2-4/P2-5」):

```
best   = cur_min / base_min                  # ★ 主判据(best-of-n)
contam = max(floor, cur 的 median/min − 1, base 的 median/min − 1)   # 污染度
base_tol = max(2%, tol − floor)              # 乾淨趟基準有效容差 = 20%
slack  = min(max(0, contam − floor), 10pp)   # 單向放寬,永不變嚴
lim    = tol (--strict)  否則  base_tol + slack
corr   = null 漂移,僅當 |corr−1| > 2 × null 相對噪聲 才啟用(否則 = 1.0)
ratio_corr = best / corr
REGRESSED ⇔ best > 1+lim 且 ratio_corr > 1+lim
improved   ⇔ best < 1−base_tol(改善判定不受本趟污染度影響)
median 超標 → 只印「median 超標(環境污染線索)」,不判紅
```

* **為什麼污染度是「單向放寬」而非「抵免(`tol − band`)」**:舊版寫的是抵免,
  而抵免的方向是**反的** —— 越吵的內核容差越小。實測:最吵的 `newman`
  (band 14%)只拿到 11% 容差,而乾淨的 `l7b`(5%)拿到 20%;最容易被誤紅的
  恰恰是最吵那個。改為單向放寬後,污染只讓門檻變鬆或持平,並設 10pp 上限
  以免疊加成 ±36% 而放過真回歸(合成測試「只有代碼慢 30%」仍判紅)。
* **為什麼 median 不判紅**:median 把搶核/降頻的離群值吃進去。同一份代碼跑三趟,
  median 最大比可達 1.56×而 best-of-n 只有 1.06× —— 用 median 判定必然 flapping。
* `null` 只做**診斷 + 縮放**,不做決策放寬:`--relax-on-drift` 曾實作後刪除,
  因為它同樣會吃掉「全內核一起變慢」的真回歸。
* **固有盲区(誠實申報)**:若某 commit 讓所有內核**與 null 一起**變慢(例如
  分配器/全域 flag 變動),比值法無法區分環境。對策是 `--assume-env-stable`
  (關閉校正,只信 raw),或同機 `--update` 重刷基線。
* **判別力自測**:`tools/bench_gate_selftest.py`(13 個合成情境,斷言 exit code),
  已掛進 CI `bench` job 的第一個檢查 —— gate 不只能變綠,必須仍能變紅。

## 2026-09-03:健檢 P2-4 / P2-5 —— 修判定式本身(不是修基準)

重寫後的 gate 已能變綠,但健檢發現它綠得不穩:兩個 gate(覆蓋率/基準)都
有「**看起來在守、實際沒在守**」的結構問題。基準 gate 的兩處:

### P2-4 污染度算了卻沒用,且方向反了

`band`(現稱 `contam = median/min − 1`,本趟典型輪比最快輪慢多少)在 docstring、
註解、輸出欄位裡都宣稱參與判定,但程式碼真正用的是 `tol − floor`(常數)
—— 印出來的 band 是裝飾。而那個被寫進文件的 `tol − band` 若真的照做,
方向還是錯的。

修法:污染度**單向放寬**,`slack = min(max(0, contam − floor), 10pp)`,
並把有效紅線(`red@` 欄)印出來,讓「這一趟到底卡在哪條線」可被看見。

| 情境(合成) | 舊版 | 新版 |
|---|---|---|
| best 慢 25%,本趟污染度 3%(乾淨) | 紅 | 紅(基準有效容差 20%) |
| **同一個 best(慢 25%),本趟污染度 20%** | **紅** | **綠**(門檻放寬到 30%) |

### P2-5 null 標尺的底噪在挪動紅線

`corr = cur_null_median / base_null_median` 把「整台機器變慢」除掉,但 `null`
是全表最小、最接近計時底噪的內核(實測 median 17 ns,min 15 / max 23 ns,
MAD 1 ns ⇒ 相對噪聲 ~5.9%),純噪聲就能讓它漂移兩位數百分比。而 `corr > 1`
會**單向放寬**門檻 ⇒ 紅線的位置取決於一個純噪聲量的正負號。

同機同碼連續實測的 `corr`:**0.941 / 0.882**(另一趟 CI runner 上為 0.824)。
一旦某趟量到方向相反的同量級漂移(如 +12%),紅線就從 1.20× 被推到
**1.34×** —— 14 個百分點的搖擺,且搖擺的來源是底噪而非代碼。

修法:加顯著性門檻,只有當 `|corr − 1| > 2 × max(cur/base 的 null 相對 MAD, 2%)`
才認定為真環境漂移並啟用校正,否則 `corr := 1.0`。

| 情境(合成) | 舊版 | 新版 |
|---|---|---|
| 代碼慢 28%,null 漂移 +8%(在其 2× 噪聲內) | **綠**(誤放行) | **紅** |
| **同一個慢 28%**,null 漂移 +30%(遠超噪聲) | 綠 | 綠(視為環境,照常校正) |

實跑對照(本沙箱,rustc 1.98.0):`null 參考內核 −11.8% —— 未過顯著性門檻
(|漂移| ≤ 2× 噪聲 = 13.3%)⇒ 不校正`。紅線從「隨 null 噪聲漂移」回到穩定的
1.20×–1.28×(後者為污染度放寬所致,且只朝寬的方向)。

### 實跑(2026-09-03,本沙箱 2 核)

```
kernel                  base       cur    best  median  contam   red@  verdict
lex                   0.0005    0.0005   0.973   0.917   10.0%   1.25×  ok
parse                 0.0039    0.0038   0.982   0.895   13.0%   1.28×  ok
laminar               0.0085    0.0090   1.059   0.969   10.1%   1.25×  ok
named_sexp            0.0089    0.0083   0.932   0.871    7.8%   1.23×  ok
l7b_evaluate          0.0069    0.0068   0.986   0.988    5.0%   1.20×  ok
r0_lex                0.0006    0.0006   0.995   1.003    6.1%   1.21×  ok
r0_parse              0.0026    0.0025   0.978   0.913    9.3%   1.24×  ok
newman_3x4x6          6.1723    4.8823   0.791   0.757    5.0%   1.20×  improved
```

`--strict`(忽略污染放寬)同樣全綠 ⇒ 判定不是靠放寬才過。`newman_3x4x6`
best 0.791× 已達 improved 門檻,但基線本身來自一次較吵的採樣,故**不**
自動 `--update`:基準基線的刷新應由人決定並附解釋。

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

## CI 首跑實測(2026-09-02,PR #8 @ GitHub ubuntu-latest 4 核)

`run #15` 四個 job 全 success(fmt/clippy/test/doc、rocq、coverage、**benchmark gate**)。
job log 里的 gate 表(跨機 2 核基線 → 4 核 runner):

```
kernel            base      cur    best  median   band  verdict
lex             0.0005   0.0005   1.021   0.962  10.0%  ok
parse           0.0039   0.0037   0.938   0.839  13.0%  ok
laminar         0.0085   0.0078   0.911   0.830  10.1%  ok
named_sexp      0.0089   0.0083   0.937   0.872   7.8%  ok
l7b_evaluate    0.0069   0.0068   0.984   0.960   5.0%  ok
r0_lex          0.0006   0.0006   0.960   0.933   5.2%  ok
r0_parse        0.0026   0.0024   0.914   0.840   9.3%  ok
newman_3x4x6    6.1723   6.6230   1.073   1.072   5.0%  ok
---- null 參考內核 -17.6% ⇒ 環境漂移校正係數 0.824
```

判讀:runner 的 `null` 比沙箱基線**快 17.6%**,而多數內核也只快 2–9% ⇒ 兩者一致,
判定 ok。真正值得留意的是 `newman_3x4x6` 在「環境比較快」的情況下**反而慢 7.3%**
(它是多執行緒通道)——舊 gate 用裸 median + 單次採樣會把這類信號整個淹掉。
基線維持沙箱版即可通過 CI(已實證);若要完全同境,在 runner 上跑一次
`--emit-baseline` 並提交(PR 內人工確認,不設自動回寫)。

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

其他旗標:`--noise-floor 0.05`(雜訊帶下限)、`--max-slack 0.10`(污染度單向
放寬上限)、`--null-k 2.0`(null 漂移顯著性倍數)、`--tiny-us 10`(把 <10 µs 的
內核降為 informational,預設 0 = 全部硬判)、`--strict`(忽略污染放寬)、
`--assume-env-stable`(停用 null 校正)、`--skip <kernel>`(harness 側)。

## CI 紅線

`cargo bench --bench hotpaths -- --reps 11 --newman-reps 5` → `tools/bench_gate.py
bench/BASELINE.json --tol 0.25`。回歸(扣雜訊 + 污染單向放寬後,且經顯著的
環境校正後仍超 `red@`)判紅,PR
不可合併;基線更新須由 `--update` 產生並附解釋。原始 `bench.log` 一律上傳為
artifact,失敗時可直接看 null 漂移與 MAD,不必重跑。
