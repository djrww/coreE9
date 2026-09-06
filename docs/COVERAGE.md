# 覆蓋率報告與門檻(COVERAGE)

> 採集:`cargo llvm-cov --lcov`(llvm-tools-preview + cargo-llvm-cov 0.9.0;零依賴庫不變)。
> 執行面:全部庫測試(33 具名測試)+ 集成矩陣(`tests/laws.rs`,31 條)。
> 門檻紅線:`tools/cov_gate.py`(CI job `coverage` 強制執行)。

## 一、現況(P4-3,2026-09-07;`cargo llvm-cov --features oracle`)

| 模組 | 覆蓋 | 比例 | 閾值 | 狀態 |
|---|---|---|---|---|
| `span.rs`(§1.2 位址映象) | 26/26 | 100.0% | ≥90% | ✅ |
| `tree.rs`(樹公理 + 驗證器負案例) | 199/202 | 98.5% | ≥90% | ✅ |
| `l9newman.rs`(§4.3 Newman 通道) | 197/202 | 97.5% | ≥90% | ✅ |
| `parse.rs`(§1–2.3 解析器) | 898/922 | 97.4% | ≥90% | ✅ |
| `shrink.rs`(反例最小化) | 74/77 | 96.1% | ≥90% | ✅ |
| `ast.rs`(§3.2–3.3 語義面) | 547/572 | 95.6% | ≥75%(豁免 90%) | ⚠️ 豁免 |
| `lex.rs`(§5.1 詞法) | 192/205 | 93.7% | ≥90% | ✅ |
| `gen.rs`(生成器宇宙 + P4-2 `gen_r0_semantic`) | 410/443 | 92.6% | ≥90% | ✅ |
| `r0.rs`(§7.2 R₀ 載體) | 1339/1443 | 92.8% | ≥90% | ✅ |
| `rep.rs`(§4 重寫系統) | 219/241 | 90.9% | ≥90% | ✅ |
| `edit.rs`(§2.1 編輯單體) | 96/97 | 99.0% | ≥90% | ✅ |

> 覆蓋率跑 `--features oracle`:P4-2 生成器 `gen_r0_semantic` 為庫公開面,
> 由 O-7 律(250 輪)於 coverage 跑中覆蓋;default build 下該函數無測試面,
> 不帶特徵跑會把 gen.rs 拖到 58%(P4-3 CI 實測教訓,2026-09-07)。
>
> `tree.rs` 殘餘 3 行 = 防禦枝不可達實證:`check_tree_axioms` 的 DFS
> 重複彈出(任何重複子邊必先觸發雙父錯誤返回,故 `continue` 不可達)+
> `l7b_evaluate` 不變量失敗枝(L7b 律斷言 bad=0,失敗即模型級 BUG 而非
> 正常路徑)。其餘 L5/L6 驗證器防禦枝由 `tree::validator_negative`
> 手工損壞 NodeView 負案例全數真實覆蓋。

> bin 的 `main`(`cl0r0.rs` / `fuzz.rs` / `l9newman.rs`)不計入:它們是執行驅動,
> 無測試面;其核心邏輯在庫內受測。

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

## 四、CI 紅線

`.github/workflows/ci.yml` → job `coverage`:
`cargo llvm-cov --features oracle --lcov` → `python3 tools/cov_gate.py`。
任何核心模組 <90%(或 ast <75%)⇒ job 失敗 ⇒ PR 不可合併。
(P4-3,2026-09-07:特徵面收緊 —— 生成器入計;`tree::validator_negative`
負案例補入;見 §一 註記。)
