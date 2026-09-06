# 規格 ↔ 測試對帳矩陣(SPEC-TRACE)

> **紀律:律先於碼 —— 每律一個具名測試;律不過,碼不合。**
> 本表即該紀律的字面落實(P1 #8):規格條文 → 具名測試 → 代碼符號,一表窮盡。
> 基線:第三迭代(+ Rocq Phase 0–2)· fmt / clippy -D warnings / test(47)/ doc -D warnings / cov gate **綠**;
> bench gate 於 2026-09-02 **重寫後轉綠**(本沙箱實跑;舊基線是單機單次採樣所致之假紅,修法與統計依據見 `docs/BENCH.md`)。本表對「綠」的定義仍是**實跑結果**,不是文件宣稱。

---

## 〇′、Rocq 形式化對照(Phase 0/1,詳見 docs/ROCQ-TRACE.md)

> ⏸ **2026-09-06 路線樞紐**:P3 #12 主線凍結(一般性缺口 + 鏡像維護成本;
> 決策與回歸條件見 `docs/PIVOT-RUSTC-ORACLE.md` §七)。Phase 0–2 定理與
> 全部窮舉/外證證據**仍有效**;rocq CI job 轉 `continue-on-error`。
> 語義/語法層開發主線轉 P4(rustc 為行為權威),對帳表見 `docs/ORACLE-TRACE.md`。

| 定理 | 內容 | 狀態 |
|---|---|---|
| R1 | 抽象 Newman:`sn r -> wcr r -> confluent r`(Huet 式,構造) | ✅ 已證 |
| R4 | `newman_unf`(唯一正規形)+ `exists_normal_form` | ✅ 已證 |
| R2 | 具體 SN:`sn step_ct`(CommutativeTrim × Guarded,µ=|E_red|)| ✅(Phase 2)|
| R3 | 具體 WCR(`ct_join_exact` → `wcr step_ct`) | ⏸ 凍結(2026-09-06):前測全綠證據保留(精確交換 2,443,506/2,443,506 @ 未過濾 4×6);「∀ 狀態 WCR」不再宣稱即將閉合 |
| R5/R6/R7 | L7b 終止 / T2 / 反例存在 | ⬜ Phase 4–6 |

配套:內核鏡像 `rocq/theories/Mirror.v`(D1–D6 決策)+ 對帳框架
`tools/rocq_reconcile.py`(Rust 實例 ↔ Rocq 計算,kernel 複驗 19 樣本點;
已抓出並修正 R4Runtime 尾插/頭插分歧)+ CI job `rocq`。

## 〇、測試全量清單(47 具名測試)

**CL0 載體 — 九律 + 編輯單體 + 定理 + 語義面(31,`tests/laws.rs`)**

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
| `test_shrink_finds_minimal_error_trigger` | **反例縮小 — 最小錯誤現場(第二迭代)**|
| `test_law_regression_fixtures` | **fixtures 回歸防線(第二迭代)**|
| `test_law_L8_red_edge_decreasing` | L8a 紅邊嚴格遞減 |
| `test_law_L8_measure_is_guaranteeing_termination` | L8b μ 保證終止 |
| `test_law_L9_unique_normal_form` | L9a 唯一正規形 |
| `test_law_L9_critical_pairs_joinable` | L9b 臨界對可合流 |
| `test_law_L9_naive_menu_finds_counterexample` | L9c 機器找反例(通道)|
| `test_law_L9_scaled_space_joinable` | **L9 規模擴張 4×5 + 並行(第二迭代)**|
| `test_law_L9b_parallel_moves_exact_swap` | **L9b′ 精確交換/側條件冗餘/紅邊單調(第四迭代,Rocq R3 前測)**|
| `test_law_L9_scaled_space_counterexample_found` | **並行版「機器找反例」仍在(第二迭代)**|
| `test_theorem_T2_interval_graphs_are_perfect` | T2 區間圖完美性 |
| `test_edit_monoid_laws` | M1/M2/M4/M5 編輯單體 |
| `test_edit_unit_operations` | **§2.1 編輯邊緣操作(第二迭代)**|
| `test_law_span_geometry` | **§1.2 半開區間代數(第二迭代)**|
| `test_law_parse_error_paths_total` | **解析錯誤回收矩陣(第二迭代)**|
| `test_law_r0_error_paths_total` | **R₀ 錯誤回收矩陣(第二迭代)**|
| `test_law_rep_menu_algebra` | **菜單 × 政策代數(第二迭代)**|
| `test_reuse_data_consistency` | **§2.2/§5.3 增量工具契約(第二迭代)**|
| `test_law_semantic_conflict_matrix` | **§3.3 相容性矩陣(第二迭代)**|
| `test_law_semantic_facts_consistent` | **§3.2–3.3 語義面一致性(第二迭代)**|
| `test_law_semantic_extract_breadth` | **語義面語法矩陣(第二迭代)**|

**R₀ 載體 — 附錄 B 解析器 + R₀ 語義斷言(10)**

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
| `test_law_r0_kind_label_exhaustive` | **R0Kind 標籤完備(第二迭代)**|
| `test_law_r0_unsupported_keyword_matrix` | **§9 排除關鍵字矩陣(第二迭代)**|

**核心冒煙 + 反例縮小(7,** `src/lex.rs` `src/parse.rs` `src/shrink.rs` mod tests **)**

`lex_lexical_invariants` · `smoke_parse_legal` · `smoke_parse_garbage` ·
`shrink_finds_exact_minimal` · `shrink_handles_unicode_boundaries` ·
`shrink_monotone_l7_style` · `shrink_empty_and_trivial`

**外律 O 系(oracle 通道自證 + 生成式差分,9,** `tests/oracle_laws.rs`,feature = "oracle" **;P4-0,2026-09-06;P4-2,2026-09-07;P4-3,2026-09-07)**

| 具名測試 | 對應律(oracle 外延) |
|---|---|
| `oracle_smoke_e0502` | 判決抽取(E0502 + 主 span 落涉事行;§1.2 位址語義) |
| `oracle_accept_clean` | L7a 無假錯誤(乾淨 R₀ 程式 → Accept) |
| `oracle_reject_uncoded_syntax_error` | L7 全化(任何輸入皆有判決;摘要行結構化排除) |
| `oracle_determinism` | L2 決定論(兩趟判決全等) |
| `oracle_expectation_grammar` | 語料約定本身(檔名 → 期望) |
| `oracle_parity_borrow_matrix` | **P4-1 主律**:rustc × 三軌 parity(39 案例);分歧 = 註冊表精確相等(24/24,BUG 候選 0,stale 0);**幾何保守下界定律**(rustc 借用衝突類碼 ⇒ Lexical 必拒,9/9);blanket(範圍外家族 under 霈屬已申報集合) |
| `oracle_fuzz_agreement` | **P4-2 主律**:生成式差分 agreement(N 輪 by-construction 樣本 × rustc 判決 × 三軌;期望失配 0、逐樣本下界律、全樣本 in-scope);失敗 ddmin 縮小入 `tests/fixtures/` |
| `test_law_semantic_place_orthogonality` | **P4-3 主律(一)**:place 敏感度正交性(雙錨:模型 × rustc)—— `&mut s.a` × `&mut s.b` 不同 place ⇒ Nll/Referent 放行 ∧ rustc 接受;同 place 雙 `&mut` ⇒ 雙軌必拒 ∧ rustc 拒絕。Lexical 軌保持綁定粒度(幾何保守下界,如實過報) |
| `test_law_semantic_cfg_liveness` | **P4-3 主律(二)**:CFG 精確 killer(雙錨)—— (a) if/else 分支不相交 ⇒ 放行 ∧ rustc 接受;(b) while 回邊(條件重複求值)⇒ 雙軌必拒 ∧ rustc 拒絕;(c) 迴圈後借用 ⇒ 放行 ∧ rustc 接受 |

> O 系測的是 **oracle 通道與 parity 引擎本身**;「本模型 vs rustc」的分歧歸檔
> 見 `docs/ORACLE-TRACE.md` §三 + §八 + `corpus/PARITY-REGISTRY.json`(4 項 / 1 家族 F-G 範圍外;P4-3 前 24 項 / 7 家族,20 項隨模型修正轉 stale 移除)。
> ✅ 2026-09-06 P4-1a:CL0 載體語義面事件真空(ORACLE-TRACE §一 發現 #1)
> 已修復 —— `ast::extract` 以 decl_site + 遍行期作用域棧正確產事件;
> `test_law_semantic_facts_consistent` / `test_law_semantic_extract_breadth`
> 均補**反真空非空斷言**(門檻實測校準),真空復發時必紅。

---

## 一、CL0 載體對帳

| 規格章節 | 條文 | 具名測試 | 代碼符號 | 狀態 |
|---|---|---|---|---|
| §1.1 | 樹節點:種類 + 半開跨度 + 直接子節點表 | `smoke_parse_legal` | `parse::Tree` / `parse::Node` | ✅ |
| §1.2 | 位址映象 σ(v) = [a,b) 半開 | `test_edit_monoid_laws`(位移函數) | `span::Span::new` | ✅ |
| §1.3 | 具名投影 π(保留具名、丟匿名/trivia) | `test_law_L6_projection_is_function_of_surface` | `Tree::named_sexp` | ✅ |
| §2.1 | 編輯單體:位移函數、複合、結合律 | `test_edit_monoid_laws` | `edit::{apply, compose, compose_seq}` | ✅ |
| §2.2 | 增量重析(reuse 準則:σ 不相交 ∧ 邊界配置同) | `test_reuse_data_consistency`(工具契約;L3/L4 等價仍依規格排除) | `parse::reparse` / `ReuseData` | ⚠️ 工具綠/等價留白 |
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
| §4.3 | L9b′ 精確交換 + 側條件冗餘 + 紅邊單調(Rocq R3 的鏡面)| `test_law_L9b_parallel_moves_exact_swap` | `rep::{apply, Menu::applicable, AState::red_edges}` | ✅(Rust 窮舉)/ ✅ Raw 版(`R3_ct_wcr_raw`,2026-09-03)/ 🔶 Guarded 版 WCR 仍待證 |
| §4.3 | L9 反例通道(機器找反例)| `test_law_L9_naive_menu_finds_counterexample` | `rep::enumerate_states` | ✅ |
| §4.4 | 錨定保持:事實攜帶可回跳 span | `test_law_L6_*`(錨位址並檢)| `ast`(事實層 span)| ✅ |
| §5.1 | 詞法 DFA、平鋪、trivia 保留 | `test_law_L1_lexical_tiling` / `lex_lexical_invariants` | `lex::lex` | ✅ |
| §5.3 | 配置快照(增量重析界)| `test_reuse_data_consistency` | `parse::Tree`(cfgs 欄位)| ⚠️ 工具綠 |
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
| 1 | L7 全化無獨立具名測試(曾被子測試合併吸收)| **已補**(迭代 1 + 2):`test_law_L7_error_totalization` + `test_law_regression_fixtures` + `test_law_semantic_extract_breadth`(語義面不 panic);修復:哨兵跨度泄漏、`ast::extract` Root 外殼未穿透、半截 4 處 `unwrap` |
| 2 | R₀ `unsupported` 僅詞法/詞級 | **已補**:節點級 Unsupported + `test_law_r0_unsupported_keyword_matrix`(16 關鍵字逐一申報) |
| 3 | L9「反例通道」是**機器找反例**,非**證明無反例** | 已強化(非消除):並行 + 4 事件 × 6 座標(623,616 狀態 × 635,424 臨界對,0 違反);「證明無反例」仍是 P3 #12(形式化)範圍 |
| 4 | 覆蓋率:核心已 ≥90%;`ast.rs` 76.9% 依豁免 | **明示豁免**(行映射失真 + R₀ 保留槽;見 docs/COVERAGE.md)—— 硬門檻 75%,不假裝覆蓋 |

**排除項(依規格,非缺口)**:L3(增量重析等價)/ L4(編輯單體相容)——基礎設施
(`reparse` / `compose`)就緒,如需激活只需補等價測試;§9 非目標(不重造 rustc)。

---

## 四、防線(CI,`.github/workflows/ci.yml`)

`push main / PR` → `cargo fmt --all --check` → `cargo clippy --all-targets -- -D warnings`
→ `cargo test --all`(上表 46 具名測試即驗收合同)→ `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
→ **coverage gate**(核心 ≥90%,ast ≥75% 豁免,`tools/cov_gate.py`)
→ ~~**bench gate**(`hotpaths` ±25%,`tools/bench_gate.py`)~~ ⏸ 2026-09-07 暫時清退(方法論留 git 史,見 DEBT-REVIEW)。

任何一條紅線 = PR 不可合併。規格讓步必須走「改規格 + 對帳表更新」而非「放水測試」。
