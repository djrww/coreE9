# R₀ 覆蓋面實測(R0-COVERAGE-EMPIRICAL)

> P4-6 交付(2026-09-07)。**只測量、如實申報** —— 不宣稱 R₀「覆蓋」未覆蓋之物。
> 本報告一切數字由 `cargo run --bin r0cov -- <file.rs> …` 實測(引擎 = `r0_parse`;
> unsupported/error 以節點級 span 實測),非人為宣稱。重現命令見文末。

---

## 〇、測量方法

`src/bin/r0cov.rs` 對每個 `.rs` 檔:

1. `r0_parse(src)`(附錄 B EBNF → 表面語法樹,全化,永不 panic);
2. 收集全部 `Unsupported` 與 `Error` 節點的跨度,合併(union)為「有待處理區」;
3. **字節覆蓋率** = `1 − (有待處理區字節 / 全檔字節)`;
4. **範圍內(in_scope)** = 該檔無任何 `Unsupported` 節點**且**無 `Error` 節點
   (即:整檔可被 R₀ + `model::model_check` 完整分析)。

誠實邊界:這裡測的是「R₀ 語法子集的接收面」,不是「語義正確面」。一個檔案
整體落入範圍,只代表 R₀ 能把它無需申報地解析成樹;語義是否與 rustc 一致,
是 parity 門檻(O 系)的範疇。

---

## 一、真實世界語料抽樣(stdlib)

樣本:4 個 rust-lang 標準庫原始檔(2026-09-07 自 `rust-lang/rust` master 拉取),
共 16,413 行 / 575,472 字節。

| 檔案 | 行數 | unsupported 節點 | error 節點 | 字節覆蓋率 | 範圍內 |
|---|---|---|---|---|---|
| `alloc/src/string.rs` | 3,679 | 103 | 6 | **2.36%** | ✗ |
| `core/src/option.rs` | 3,061 | 105 | 8 | **22.74%** | ✗ |
| `core/src/slice/mod.rs` | 5,852 | 58 | 2 | **0.06%** | ✗ |
| `std/src/fs.rs` | 3,821 | 8 | 6 | **1.17%** | ✗ |
| **彙總** | **16,413** | **274** | **22** | **4.85%** | **0 / 4** |

**判讀(如實)**:R₀ 對真實標準庫的覆蓋率**極低**。4 檔全部在範圍外,整體字節
覆蓋率僅 **4.85%**。這不是 bug —— 而是 `R0_EBNF` 的側條件**如實**地把絕大多數
真實 Rust 構造排除出子集:泛型實參/`<…>`、宏 `!`、閉包 `|…|`、`match` 模式、
`trait`/`impl`/`use`/`mod`/`pub`/`unsafe`/`enum`/`type`/`const`/`async`/`dyn`、
生命週期 `'a` 等(見 `src/r0.rs` 的 `R0_EBNF` 與 `EXCLUDED_ITEM_KWS`)。

`unsupported` 節點(共 274 個)正是 R₀ 對「我為何不處理這裡」的**結構化自白**;
`error` 節點(22 個)為語法層的「此處無法起始」殘骸。兩者都以節點級 span 如實
申報,不假裝覆蓋 —— 這是 §9「如實申報」在真實語料上的第一次落地。

---

## 二、對照組:curated 語料(P4-1 受驗對象)

37 個含語義的 `accept_*`/`e_*` 案例 + 1 個 `uncoded_syntax_error`(控制 1 個語法錯誤)。

| 彙總 | 數值 |
|---|---|
| 檔案數 | 39 |
| 行數 | 291 |
| 字節 | 3,789 |
| unsupported 節點 | **0** |
| error 節點 | 1(`uncoded_syntax_error.rs`) |
| 範圍內 | **38 / 39** |
| 整體字節覆蓋率 | **99.34%** |

**判讀**:curated 語料**刻意**落在 R₀ 子集內(borrow 現場的幾何骨架),因此幾乎
全數在範圍內 —— 這是 R₀ 的**設計目標面**;而真實標準庫是 R₀ 的**非目標面**。

---

## 三、結論與擴張排序(數據化,呼應 PIVOT §五)

| 組合 | 文件數 | 範圍內 | 覆蓋率 |
|---|---|---|---|
| 真實 stdlib | 4 / 4 | 0 / 4 | 4.85% |
| curated 語料 | 39 | 38 / 39 | 99.34% |

**兩條結論(皆為實測,非猜測):**

1. **R₀ 是「子集載體」而非「Rust 子語言」。** 它在設計上只接收獲選的借用現場
   骨架;對真實 Rust,`unsupported` 面**如實**把約 95% 的字節排除在子集外。
   「擴張 R₀ 以覆蓋真實 Rust」的投入將沿此面迅速增大 —— 這正是 PIVOT §八
   P4-7 的決策所述:每個構造走「EBNF 增補 → unsupported 縮小 → 差分語料擴 →
   parity 不回退」四連,且須由**此處的 unsupported 直方圖**主導擴張優先序。

2. **在當前 R₀ 子集內,「覆蓋」已達標。** curated 語料 99.34% 覆蓋、38/39 範圍內,
   唯一的範圍外是刻意的語法錯誤案例(語義面如實降級,與 P4-1 parity 一致)。

**對後續迭代的一句話**:擴張優先序不靠主觀,靠 `r0cov` 的 unsupported 直方圖 +
   P4-2 生成式差分的 MODEL-DIFF 熱點。待擴張候選(依頻率排序,由①的排除面
   隱含):`match` 基礎模式 → `impl`/trait 語法面 → 泛型語法面 → 閉包(捕獲)。

---

## 四、重現

```sh
cargo build --bin r0cov
# 真實語料(2026-09-07 自 rust-lang/rust master)
curl -sSL https://raw.githubusercontent.com/rust-lang/rust/master/library/core/src/slice/mod.rs -o core_slice.rs
curl -sSL https://raw.githubusercontent.com/rust-lang/rust/master/library/alloc/src/string.rs -o alloc_string.rs
curl -sSL https://raw.githubusercontent.com/rust-lang/rust/master/library/std/src/fs.rs -o std_fs.rs
curl -sSL https://raw.githubusercontent.com/rust-lang/rust/master/library/core/src/option.rs -o core_option.rs
cargo run --bin r0cov -- alloc_string.rs core_option.rs core_slice.rs std_fs.rs
# curated 對照
cargo run --bin r0cov -- corpus/curated/*.rs
```

> 版本見證:rustc 1.98.1(48a229cea 2026-09-01);`r0cov` 內核為 `r0_parse`。
