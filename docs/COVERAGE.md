# 覆蓋率報告與門檻(COVERAGE)

> 採集:`cargo llvm-cov --lcov`(llvm-tools-preview + cargo-llvm-cov 0.9.0;零依賴庫不變)。
> 執行面:全部庫測試(**17** 具名測試)+ 集成矩陣(`tests/laws.rs`,**36** 條)= **53**。
> 門檻紅線:`tools/cov_gate.py`(CI job `coverage` 強制執行)。

## 一、現況(第二迭代,2026-09-02)

| 模組 | 覆蓋 | 比例 | 閾值 | 狀態 |
|---|---|---|---|---|
| `span.rs`(§1.2 位址映象) | 26/26 | 100.0% | ≥90% | ✅ |
| `gen.rs`(生成器宇宙) | 258/260 | 99.2% | ≥90% | ✅ |
| `edit.rs`(§2.1 編輯單體) | 96/97 | 99.0% | ≥85%(豁免 90%) | ⚠️ 豁免 |
| `l9newman.rs`(§4.3 Newman 通道) | 197/202 | 97.5% | ≥90% | ✅ |
| `tree.rs`(樹公理抽取) | 58/60 | 96.7% | ≥90% | ✅ |
| `parse.rs`(§1–2.3 解析器) | 951/983 | 96.7% | ≥90% | ✅ |
| `shrink.rs`(反例最小化) | 74/77 | 96.1% | ≥90% | ✅ |
| `lex.rs`(§5.1 詞法) | 193/206 | 93.7% | ≥90% | ✅ |
| `rep.rs`(§4 重寫系統) | 240/245 | 98.0% | ≥90% | ✅ |
| `r0.rs`(§7.2 R₀ 載體) | 1337/1480 | 90.3% | ≥90% | ✅ |
| `ast.rs`(§3.2–3.3 語義面) | 319/415 | 76.9% | ≥75%(豁免 90%) | ⚠️ 豁免 |

> bin 的 `main`(`cl0r0.rs` / `fuzz.rs` / `l9newman.rs`)不計入:它們是執行驅動,
> 無測試面;其核心邏輯在庫內受測。

### 豁免的書面理由(兩條,各自帶硬門檻,不是共用一個)

* **`ast.rs` ≥75%** —— §3.2–3.3 語義面:
  (a) llvm-cov 對 3 行內的小函數(match 直接返回 `&str`)存在計數器歸屬失真;
  (b) 其餘未覆蓋行對應 CL0 語法不可達的保留槽(`Ctx::Lhs` / killer 防禦分支)。
* **`edit.rs` ≥85%** —— 2026-09-03 由 CI 實測發現:
  本機量 96/97 = 99.0%,GitHub runner 量 86/97 = **88.7%**。同一份碼、同一個
  rustc 1.98.0 / cargo-llvm-cov 0.9.0 / x86_64、同一組 36 個測試全過。
  差的那 10 行(`Edit::is_empty` 53–55、`Edit::shift` 58–66)在 runner 上被
  inline 掉之後行歸因歸給呼叫者,**並非沒執行** —— 已證明:本機只跑
  `tests/laws.rs` 一個二進位,`edit.rs:58` 就有 count=603,且
  `tests/laws.rs:824/1296` 確實呼叫 `e.shift(…)`。
  **真實缺口只有第 93 行這 1 行。** 門檻取 85% 而非 75%,是為了仍抓得到掉
  ≥4 行的真實退化(88.7% → 84.5% 即紅),**豁免不是天窗**;
  `cov_gate_selftest.py` 已就此加 5 個判別力情境(含「掉到 84.9% → 紅」)。

### ⚠️ 覆蓋率數字不可跨平台移植(2026-09-03,已知且無法由配置消除)

本表數字以**本機實測**為準;runner 上量到的值在 2–3 個模組上會不同。已實測排除:

| 懷疑 | 結論 |
|---|---|
| stale profraw | 否 —— `rm -rf target` 重建後一樣 |
| toolchain 漂移 | 否 —— 鎖 `dtolnay/rust-toolchain@1.98.0` 後一樣 |
| 隨機性 | 否 —— fuzzer 是固定種子 `0xC10_2024_0001` |
| 行表不同 | 否 —— 兩邊都是 edit.rs 95 行 / rep.rs 238 行,行號完全相同 |

真正的機制:小函數是否被 inline 會改變**行歸因** —— inline 掉之後,該函數
自己的行顯示為 0(歸給呼叫者)。實測三種組態都無效:
`-Ccodegen-units=1`、`-Ccodegen-units=256`、`-Cllvm-args=--inline-threshold=0`。
也試過 `CARGO_INCREMENTAL=0`(rustc coverage book 建議做法):`rep.rs` 兩邊確實
一致了(235/245),但 `span.rs` 在本機反而由 26/26 掉到 23/26 —— **沒解決
可移植性,反而製造本地紅燈,故不採用**。

| 模組 | 本機(增量開,現行) | GitHub runner |
|---|---|---|
| `edit.rs` | 96/97 = 99.0% | 86/97 = 88.7% |
| `span.rs` | 26/26 = 100.0% | 26/26 = 100.0% |
| `rep.rs` | 240/245 = 98.0% | 235/245 = 95.9% |

⇒ **門檻由 CI 判**;本機量到不同的數字屬已知現象,不是回歸。
小模組(`span.rs` 僅 26 行,3 行就差 11.5 個百分點)本來就落在雜訊裡,
這也是為什麼豁免要寫明理由與硬門檻,而不是直接調低全域門檻。

## 一之二、2026-09-03 健檢修復後的重測(同機 2 核,rustc 1.98.0)

修掉 `r0_lex` 的 UTF-8 邊界 panic、`r0_parse::close()` 的棧空 panic,
以及 `l7b_evaluate` 的 `parse(..).unwrap()` panic 之後重跑:上表 `parse.rs` /
`shrink.rs` / `l9newman.rs` / `r0.rs` 四列已更新為實測值。
其中 `r0.rs` 由 1287/1430(90.0%,**恰好貼線**)升到 1337/1480(90.3%)——
新修的分支本身就有測試覆蓋,門檻的脆弱度因此下降。

## 二、`ast.rs` 豁免的誠實理由(不假裝覆蓋)

1. **計數器歸屬失真**:`llvm-cov` 對 3 行內的小函數(如 `EvKind::label` /
   `Track::label` 的 `match self { … => "…" }` 直返),把執行計數器歸屬到
   函數首行而非各 `arm` 行 —— 測試已斷言其被調用(非空標籤),但行級
   報告仍標為未覆蓋。屬報告工具失真,非代碼未執行。
2. **CL0 語法不可達的保留槽**:`ast.rs` 的 `Ctx::Lhs`(賦值語義只存在於 R₀,
   CL0 語法面無賦值)、`Block` 分支中部分防禦路徑 —— 對 CL0 樹結構恆不觸發,
   為 R₀/LSP 語義保留。規格不因之修改。
3. **語義面曾休眠(本迭代修復)**:`extract` 以 `Root`(語法外殼)進入語義面,
   而遍歷只認 `FnItem`/`Block` ⇒ **合法程式的事實層恆空**(25% 覆蓋率的
   真正原因)。已修復(以 Root 的子節點進入)+ 修復半截程式的 4 處 `unwrap`
   panic(全化:無資料可抽即如實跳過)。25% → 76.9%。

## 三、覆蓋率提升手段(記錄,供下迭代)

- 語義面:30+ 語法樣本矩陣(`test_law_semantic_extract_breadth`)+ 三軌一致性
  (`test_law_semantic_facts_consistent`)+ 衝突矩陣(`test_law_semantic_conflict_matrix`)。
- 解析錯誤回收:構造壞一半矩陣(`test_law_parse_error_paths_total` / 
  `test_law_r0_error_paths_total`)。

## 三之二、門檻自身的可信度:fail-open 修復(2026-09-03 健檢 P2-6)

門檻最危險的失效模式不是「判太嚴」,而是**靜默失效** —— 舊版 `cov_gate.py`
只迭代 lcov 裡**存在**的模組,因此下面四種情形一律印 `coverage gate: ok`
並 exit 0,門檻只在文件裡存在:

| # | 情境 | 舊版 | 新版 |
|---|---|---|---|
| 1 | 報告為空(一個 `SF:` 都沒有) | exit 0 ✅綠 | **exit 1 紅** |
| 2 | 報告只剩被過濾掉的 `src/bin/`(如 `l9newman.rs`) | exit 0 ✅綠 | **exit 1 紅** |
| 3 | 核心模組缺席(如 `r0.rs` 整支沒被量到) | exit 0 ✅綠 | **exit 1 紅** |
| 4 | 兩個不同路徑同名 basename(如 `src/` 與 `src/util/` 都有 `r0.rs`) | 後蓋前 | **exit 1 紅** |

實測對照:空報告下 `git show HEAD~:tools/cov_gate.py` → `coverage gate: ok`,
exit 0;新版 → exit 1。新版並在結尾印出約束規模,讓「到底有幾個模組真的
被門檻看著」不再是一句無法驗證的斷言:

```
coverage gate: ok(已檢查 11 個模組;受門檻約束 11 個 = 10 核心 ≥90% + 1 豁免 ≥75%)
```

情境 2 的由來:`src/l9newman.rs` 與 `src/bin/l9newman.rs` 同名,舊版只因
「先過濾 `/bin/`」才沒被覆蓋掉 —— 那是運氣,不是防護。新版把同名衝突
本身列為紅線,運氣變成顯式檢查。

## 四、CI 紅線

`.github/workflows/ci.yml` → job `coverage`:
`cargo llvm-cov --lcov` → `python3 tools/cov_gate_selftest.py`(門檻自測)
→ `python3 tools/cov_gate.py target/coverage.lcov`。

`tools/cov_gate_selftest.py` 以合成 lcov 驗 9 個情境並斷言 exit code
(上表四條 fail-open 防線 + 門檻本身仍要能判紅),與 `bench_gate_selftest.py`
同一紀律:**gate 不只能變綠,必須仍能變紅**。任何核心模組 <90%
(或 ast <75%)⇒ job 失敗 ⇒ PR 不可合併。

門檻可用 `--core <0–1>` / `--ast <0–1>` 覆寫(實測:`--core 0.95` 立刻判紅
`lex.rs / r0.rs / rep.rs`;`--ast 0.80` 判紅 `ast.rs`)—— 這兩個旗標也是
「門檻真的在算」的現成證據,不是裝飾。
