# 規格 ↔ 測試對帳矩陣(SPEC-TRACE)

> **紀律:律先於碼 —— 每律一個具名測試;律不過,碼不合。**
> 本表即該紀律的字面落實(P1 #8):規格條文 → 具名測試 → 代碼符號,一表窮盡。
> 基線:`v0.1.1` 增量(第一迭代)· 全管線綠(fmt / clippy -D warnings / test / doc -D warnings)。

---

## 〇、測試全量清單(27 具名測試)

**CL0 載體 — 九律 + 編輯單體 + 定理(16,`tests/laws.rs`)**

| 具名測試 | 對應律 |
|---|---|
| `test_law_L1_roundtrip_byte_exact` | L1 無損回環 |
| `test_law_L1_lexical_tiling` | L1(詞法平鋪前提)|
| `test_law_L1_exhaustive_small` | L1(小輸入窮舉)|
| `test_law_L2_determinism` | L2 決定論 |
| `test_law_L5_laminar_nesting` | L5 層狀嵌套 |
| `test_law_L6_projection_is_function_of_surface` | L6 靜態化(具名投影)|
| `test_law_L7a_no_false_errors` | L7a 無假錯誤 |
| `test_law_L7b_structural_maximality` | L7b 結構極大化(迭代淨化)|
| `test_law_L7_error_totalization` | **L7 全化(第一迭代重建)**|
| `test_law_L8_red_edge_decreasing` | L8a 紅邊嚴格遞減 |
| `test_law_L8_measure_is_guaranteeing_termination` | L8b μ 保證終止 |
| `test_law_L9_unique_normal_form` | L9a 唯一正規形 |
| `test_law_L9_critical_pairs_joinable` | L9b 臨界對可合流 |
| `test_law_L9_naive_menu_finds_counterexample` | L9c 機器找反例(通道)|
| `test_theorem_T2_interval_graphs_are_perfect` | T2 區間圖完美性 |
| `test_edit_monoid_laws` | M1/M2/M4/M5 編輯單體 |

**R₀ 載體 — 附錄 B 解析器(8,`src/r0.rs` mod tests)**

| 具名測試 | 對應規格 |
|---|---|
| `r0_lex_tiling` | §5.1 詞法平鋪(對 R₀)|
| `r0_lalr1_clean_checks` | §7.2 LALR(1)-乾淨片段 |
| `r0_unsupported_detects` | §9 如實申報(詞法/詞級)|
| `r0_parse_legal_roundtrip` | §7.2 全語法面 · 無 Error · byte-exact |
| `r0_parse_garbage_totalization` | §2.3 全化(對 R₀)|
| `r0_parse_unsupported_nodes` | §9 節點級 Unsupported(第一迭代)|
| `r0_parse_depth_honest` | §9 機器界如實申報(Depth,永不 panic)|
| `r0_parse_determinism_named_sexp` | L2/L6 決定論 + 具名投影(對 R₀)|

**核心冒煙(3,** `src/lex.rs` `src/parse.rs` mod tests **)**

`lex_lexical_invariants` · `smoke_parse_legal` · `smoke_parse_garbage`

---

## 一、CL0 載體對帳

| 規格章節 | 條文 | 具名測試 | 代碼符號 | 狀態 |
|---|---|---|---|---|
| §1.1 | 樹節點:種類 + 半開跨度 + 直接子節點表 | `smoke_parse_legal` | `parse::Tree` / `parse::Node` | ✅ |
| §1.2 | 位址映象 σ(v) = [a,b) 半開 | `test_edit_monoid_laws`(位移函數) | `span::Span::new` | ✅ |
| §1.3 | 具名投影 π(保留具名、丟匿名/trivia) | `test_law_L6_projection_is_function_of_surface` | `Tree::named_sexp` | ✅ |
| §2.1 | 編輯單體:位移函數、複合、結合律 | `test_edit_monoid_laws` | `edit::{apply, compose, compose_seq}` | ✅ |
| §2.2 | 增量重析(reuse 準則:σ 不相交 ∧ 邊界配置同) | (規格排除 L3/L4;基礎設施在,未激活) | `parse::Tree`(配置快照欄位) | ⬜ 依規格留白 |
| §2.3 | ERROR 全化:任意輸入必回樹 | **`test_law_L7_error_totalization`** | `parse::parse` / `Tree::validate_continuity` | ✅ |
| §2.3 | L7a 無假錯誤(合法程式 0 ERROR) | `test_law_L7a_no_false_errors` | `parse::parse` / `Tree::has_error` | ✅ |
| §2.3 | L7b 極大錯誤跨度互不嵌套 + 迭代淨化 | `test_law_L7b_structural_maximality` | `Tree::maximal_error_spans` | ✅ |
| §3.1 | 跨度嵌套(laminar)+ L5 | `test_law_L5_laminar_nesting` | `Tree::laminar_ok` | ✅ |
| §3.2 | liveness 三軌(lexical / NLL / referent) | `test_theorem_T2_interval_graphs_are_perfect`(承載) | `ast`(三軌 liveness) | ✅ |
| §3.3 | 衝突圖:區間圖 ⊂ 弦圖 ⊂ 完美圖 | `test_theorem_T2_interval_graphs_are_perfect` | `ast`(衝突圖構造) | ✅ |
| §3.4 | 樹 = 1 維 CW 複形,χ = 1 | `test_law_L5_laminar_nesting`(並檢) | `tree`(CW 複形檢驗) | ✅ |
| §3.5 | 空衝突圖 ⇒ 幾何收斂 | `test_theorem_T2_interval_graphs_are_perfect`(並檢) | `ast::red_edges` | ✅ |
| §4.1 | 狀態空間(事件多集 + 借用邊 + 日誌) | `test_law_L8_*` / `test_law_L9_*`(承載) | `rep::AState` / `rep::Ev` | ✅ |
| §4.2 | L8:菜單每規則嚴格遞減 μ | `test_law_L8_red_edge_decreasing` | `rep::Menu` / `rep::Rule` | ✅ |
| §4.2 | L8:μ ⇒ 終止保證 | `test_law_L8_measure_is_guaranteeing_termination` | `rep::Policy`(μ 計算) | ✅ |
| §4.3 | L9:SN + WCR ⇒ CR(Newman)| `test_law_L9_unique_normal_form` + `test_law_L9_critical_pairs_joinable` | `l9newman::newman_check` | ✅ |
| §4.3 | L9 反例通道(機器找反例)| `test_law_L9_naive_menu_finds_counterexample` | `rep::enumerate_states` | ✅ |
| §4.4 | 錨定保持:事實攜帶可回跳 span | `test_law_L6_*`(錨位址並檢)| `ast`(事實層 span)| ✅ |
| §5.1 | 詞法 DFA、平鋪、trivia 保留 | `test_law_L1_lexical_tiling` / `lex_lexical_invariants` | `lex::lex` | ✅ |
| §5.3 | 配置快照(增量重析界)| (與 §2.2 同留白)| `parse::Tree`(cfgs 欄位)| ⬜ 依規格留白 |
| §6.3 | 判定權不轉移(rustc 面記 0)| `test_law_L8_measure_is_guaranteeing_termination` | `rep`(μ 定義)| ✅ |
| 附錄 A | L1–L9 全矩陣 | 上表 16 條 | — | ✅ |
| 附錄 B | R₀ EBNF(見下節) | 下節 8 條 | `r0` | ✅ |
| §9 | 非目標:不重造 rustc;側條件 = 如實申報 | `r0_parse_unsupported_nodes` / `r0_unsupported_detects` | `r0::{unsupported, r0_parse}` | ✅ |

---

## 二、R₀ 載體對帳(§7.2 / 附錄 B)

| 規格條文 | 具名測試 | 代碼符號 | 狀態 |
|---|---|---|---|
| R₀ 詞法 = CL0 詞法(D 子集,註釋/raw string 保留)| `r0_lex_tiling` | `r0::r0_lex` | ✅ |
| LALR(1)-乾淨片段(消除歧義構造)| `r0_lalr1_clean_checks` | `r0::lalr1_clean` | ✅ |
| `r0_parse`:EBNF → CST(`R0Tree { src, nodes }`)| `r0_parse_legal_roundtrip` | `r0::r0_parse` / `R0Tree` / `R0Node` | ✅ |
| span / 連續性 / laminar 同 CL0 紀律 | `r0_parse_legal_roundtrip`(並檢)| `R0Tree::{validate_continuity, laminar_ok}` | ✅ |
| byte-exact roundtrip(零丟失)| `r0_parse_legal_roundtrip` / `r0_parse_garbage_totalization` | `R0Tree::unparse` | ✅ |
| 全化:空串/亂位元/半截/非法 utf8 → 必 Ok | `r0_parse_garbage_totalization` | `r0::r0_parse` | ✅ |
| 節點級 Unsupported(規則/位置,附 note)| `r0_parse_unsupported_nodes` | `R0Node{kind: Unsupported, note, span}` | ✅ |
| 機器界如實申報(Depth 256→64 工程界)| `r0_parse_depth_honest` | `R0ParseIssue::Depth` | ✅ |
| 決定論 + 具名投影 | `r0_parse_determinism_named_sexp` | `R0Tree::named_sexp` | ✅ |

> R₀ 遞歸界:規格首取 256;第一迭代因 2MB 測試線程棧實測不安全,下修 64
> (優先級語法鏈約 15 幀/層),並以 `r0_parse_depth_honest` 如實申報 —— 界本身
> 是機器界,不是「假裝覆蓋」。

---

## 三、誠實留白(不假裝覆蓋)

| # | 缺口 | 狀態 |
|---|---|---|
| 1 | L7 全化無獨立具名測試(曾被子測試合併吸收)| **第一迭代已補**:`test_law_L7_error_totalization`(四源:合法/雙重垃圾/半截/隨機位元),並藉此**發現並修復** CL0 解析器真實缺陷——錯誤回收彈棧未 `finalize`,哨兵跨度 (u32::MAX, 0) 泄漏進 `unparse` 至越界 |
| 2 | R₀ `unsupported` 僅詞法/詞級 | **第一迭代已補**:節點級 Unsupported(note + 精確 span),排除項:trait/impl/use/mod/pub/unsafe/async/match/macro_rules/dyn/enum/type/static/const/extern/where + 閉包/生命週期/屬性/泛型歧義/非法符號 |
| 3 | L9「反例通道」是**機器找反例**,非**證明無反例** | 仍留白(如實) —— 提升通道規模在第二迭代 P0 #4(4 事件 × 6 座標 + 並行)|

**排除項(依規格,非缺口)**:L3(增量重析等價)/ L4(編輯單體相容)——基礎設施
(`reparse` / `compose`)就緒,如需激活只需補等價測試;§9 非目標(不重造 rustc)。

---

## 四、防線(CI,`.github/workflows/ci.yml`)

`push main / PR` → `cargo fmt --all --check` → `cargo clippy --all-targets -- -D warnings`
→ `cargo test --all`(上表 27 具名測試即驗收合同)→ `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`。

任何一條紅線 = PR 不可合併。規格讓步必須走「改規格 + 對帳表更新」而非「放水測試」。
