# 路線樞紐:以 rustc 為權威指標開發語義/語法層(PIVOT-RUSTC-ORACLE)

> 2026-09-06 規劃(聯網文獻比對後,見 §三)。
> 對應決策:**P3 #12 Rocq 定理認證路線 → ⏸ 凍結(資產保留,不刪除);新主線 P4 —— oracle 驅動的 Rust 語義/語法開發。**
> 紀律不變:**律先於碼、機械自証、如實申報**。改變的只是「裁判」:從 Rocq kernel(證明權威)換成 rustc(行為權威)。

---

## 〇、決策摘要(先讀)

1. **樞紐不是背離原規格,而是把規格裡的佔位符激活。** `rep.rs` 的測度本就寫明:
   `μ(P) = (|E_red(P)|, |Err_rustc(P)|)`——「rustc 面為 oracle 範疇,記 0」。
   新路線就是把 `|Err_rustc|` 從恆 0 變成**每次可實測的機器查詢**。判定權仍然不轉移給任何單一工具(見 §四「分歧三分法」),但 rustc 的行為從「口頭引用」升級為「結構化對帳」。
2. **為什麼停 Rocq 主線(誠實清單):**
   - R3 的一般性缺口(窮舉 ≠ 定理)在 `HARD-ITEMS #1` 已預示;捷徑已被否證,主定理 `ct_join_exact` 開放中,且其「適用性封閉性」需自建理論,預計 XL 週期高、失敗風險實在;
   - `HARD-ITEMS #2` 指出的「鏡像漂移」是雙語言永久稅:Rocq 鏡像要人手同步。
     **新路線把「第二實作」從手寫鏡像換成 rustc 本體——rustc 不是鏡像,是權威本尊,且由上游百人維護、自帶百萬級測試與 crater。**
   - Rocq Phase 0–2 資產(鏡像、對帳框架、抽象 Newman、具體 SN)**全數保留**;SPEC-TRACE 的 R3 行改標「⏸ 凍結」,窮舉證據(2,443,506 對全綠等)仍然有效,只是不再宣稱「即將成定理」。
3. **權威性分層(同等權威指標的明確化):**

   | 層 | 指標 | 性質 | 用法 |
   |---|---|---|---|
   | 行為權威 | **rustc 本體**(stable / nightly) | 判決:`Accept` / `Reject{E-code, span}` | 差分主裁判(Tier-A/B) |
   | 同源行為 | **rustc_driver / rustc_interface 內嵌** | 深取數:HIR/MIR/borrowck/Polonius facts | 語義層解剖(Tier-B) |
   | 同源生態 | **ra-ap-rustc_lexer / ra_ap_syntax** | 語法層差分對象 + 增量重析先例 | 語法層對照(Tier-C) |
   | 規範權威 | **FLS(Ferrocene Language Specification,已歸 rust-lang t-spec)/ Rust Reference** | 條文:Legality Rules / Dynamic Semantics | 爭議裁決、文檔引用 |
   | 模型權威 | **a-mir-formality / MiniRust / Oxide** | 可執行形式模型 | 設計對標、方法論先例 |

4. **範圍:** R₀(附錄 B 子集)為受驗對象;CL0 九律載體**原樣保留**——L1–L9 一字不改,新增「外律 O 系」(oracle parity,見 §八)。

---

## 一、現況盤點(截至 2026-09-06,main @ 0537e43)

| 資產 | 規模/狀態 | 在新路線的角色 |
|---|---|---|
| `src/r0.rs`(EBNF + `r0_lex` + `lalr1_clean` + `unsupported` + `r0_parse` 節點級申報) | 2,243 行,已交付 CST | **語法層受驗對象 A**:差分 rustc/ra_ap_syntax |
| `src/ast.rs`(三軌 liveness:Lexical/Nll/Referent + 衝突圖 + 區間圖貪婪著色) | 713 行 | **語義層受驗對象 B**:判決差分 rustc borrowck |
| `src/rep.rs`(修法菜單 + L8/L9)+ `src/l9newman.rs` | 426 + 288 行 | µ 第二分量激活(§六) |
| `src/gen.rs` / `src/bin/fuzz.rs` / `src/shrink.rs` | 340 + 404 + 120 行 | **升級為語義感知生成式差分**(§八 P4-2) |
| `tests/laws.rs` 47 具名測試 + `docs/SPEC-TRACE.md` | 全綠 | 紀律載體不變,擴 O 律行 |
| `rocq/`(Phase 0–2)+ `docs/ROCQ-*.md` + Maude/NaTT 外證 | R3 開放 | ⏸ 凍結歸檔(§七) |
| CI(fmt/clippy/test/doc/coverage/bench/rocq) | 綠 | 增 `oracle` job;rocq job 標 allow-failure/凍結 |
| 工程:CI runner 已是 rustc stable 環境 | — | Tier-A 零新增系統依賴;Tier-B 需 nightly 釘版 |

**既有誠實缺口在新路線的對照:** R₀ 無 MIR 級借用事實、三軌模型無 place 敏感度(x.a 與 x.b 目前同 binding 衝突)、無控制流精確 killer(while/if 分支下 Nll 軌是線性近似)——這些正是「以 rustc 開發語義」第一批要被差分暴露的點(§五、P4-3)。

---

## 二、目標架構

```
cl0r0(core,零依賴,不受影響,publish 面不變)
│
cl0r0-oracle(新 workspace 成員或 feature = "oracle" 模組;dev-dependency 面)
├─ trait Oracle            // fn check(&self, src: &str) -> Verdict
│                          // Verdict = Accept | Reject { code, spans, class }
├─ CliOracle     (Tier-A)  // rustc 子進程 --edition=2021 --error-format=json
├─ DriverOracle  (Tier-B)  // rustc_driver::run_compiler + Callbacks::after_analysis
│                          //   #![feature(rustc_private)];rust-toolchain 釘 nightly
├─ ModelOracle            // 本專案 ast.rs 三軌 → Verdict(受驗對象,同一接口)
└─ harness               // diff(ModelOracle, CliOracle) × corpus → 報告 + triage
corpus/
├─ curated/     // 手寫 + rustc 官方 ui/borrowck 用例精選改寫(合 R₀ 者照收)
├─ generated/   // gen.rs 語義感知升級:生成時即知期望判決(accept/reject by construction)
└─ real/        // 後期:crates.io 熱門 crate 的函數級切片(P4-6)
docs/ORACLE-TRACE.md      // 對帳矩陣:案例 → rustc 判決 → 本模型判決 → 歸因(BUG/MODEL-DIFF/RUSTC-BUG)
tools/oracle_gate.py      // 類比 bench_gate:門檻 + 基線 + --update 重刷紀律
```

**四個關鍵設計裁決:**

1. **比對粒度 = 錯誤碼集合 + 涉事 span,不是診斷文本。** 文本逐版漂移;`E0382/E0499/E0502/E0503/E0505/E0506/E0507/E0596/E0597` 的碼與 span 才是穩定合同。快照紀律沿用 trybuild/ui_test 的 `.stderr + overwrite 重刷` 工作流(與本專案 `bench --update` 同哲學)。
2. **分歧三分法(判定權不轉移的機械化):**
   - `BUG` —— 本模型錯 → 修,並以 fixture 鎖回歸(沿用 shrink + fixtures 防線);
   - `MODEL-DIFF` —— 模型刻意保守/簡化(如 NLL problem-case 3、Polonius 位置敏感接受)→ 入 ORACLE-TRACE 白名單,附規格出處;這是模型的**主體性**,不跟 rustc 走到底;
   - `RUSTC-BUG` —— 上遊議題 → 連結 issue,案例隔離。rustc 的 type checker 曾有實證 unsoundness 研究(2026),權威也要誠實對待。
3. **版本矩陣:** stable 釘「當前 + 前一版」+ nightly 釘一版;每輪報告 per-version agreement。這延續 `EXTERNAL-XCHECK` 的多實作共識哲學——現在的「實作們」= 本模型 + rustc stable + rustc nightly(+ 可選 polonius 引擎),天然三至四實作。
4. **core 零依賴不破壞:** oracle 全部在 dev/feature 面;`cargo publish` 的 cl0r0 面不新增任何依賴。Tier-B 只在 nightly CI job 存在,`continue-on-error` 不入主門檻(承 HARD-ITEMS #4 可重建教訓:`rust-toolchain.toml` 入庫)。

---

## 三、文獻坐標(聯網查證 2026-09-06)

| # | 文獻/資源 | 與本專案的關係 |
|---|---|---|
| 1 | **Oxide: The Essence of Rust**(Weiss, Gierczak, Patterson, Ahmed,OOPSLA 2019,[arXiv:1903.00982](https://arxiv.org/abs/1903.00982)) | **方法論直接先例**:形式模型 + Reducer(Rust→Oxide)+ OxideTC,對 **rustc 官方測試套 200+ 用例**做「tested semantics」驗證。Oxide 的「模型 vs rustc 差分」正是 P4 的學術模板;差別:Oxide 驗型系統判決,我們另驗**區間幾何投影 + 修法菜單** |
| 2 | **Polonius 體系**:Matsakis「Foundations of the Next Generation Borrow Checker」;[rust-lang/polonius](https://github.com/rust-lang/polonius)(datalog 原型,`-Znll-facts`);Rust Project Goals [2025h1「Scalable Polonius」](https://rust-lang.github.io/rust-project-goals/2025h1/Polonius.html)→[2025h2「Stabilizable Polonius」](https://rust-lang.github.io/rust-project-goals/2025h2/polonius.html);Stjerna 碩/博士論文 [Optimising the Next-Generation Borrow Checker for Rust](https://www.diva-portal.org/smash/get/diva2:1981974/FULLTEXT01.pdf)(graded borrow checker、`loan_is_active_at`) | Nll/Referent 軌的算法語義;Tier-B 取 facts 的歷史與現狀;「alpha 版接受 NLL problem-case 3」= 本模型最經典的 MODEL-DIFF 候選 |
| 3 | **rustc 內嵌樣板**:[rustc-dev-guide 官方範例](https://github.com/rust-lang/rustc-dev-guide/blob/main/examples/rustc-driver-example.rs)(`Callbacks::after_analysis` 取 `TyCtxt`,nightly-2025-03-28 實測);[rustc_borrowck::polonius](https://doc.rust-lang.org/stable/nightly-rustc/rustc_borrowck/polonius/index.html)(含 `-Zpolonius=legacy` facts 生成模塊) | Tier-B 的權威接線圖;Clippy/Prusti/MIRAI 同構先例 |
| 4 | **a-mir-formality**([2025h2 目標:Borrow checking in a-mir-formality](https://rust-lang.github.io/rust-project-goals/2025h2/a-mir-formality.html);MiniRust 函數體集成 + Polonius Alpha 模型) | 設計公理對標:「**compiler 應是模型的 sound-but-incomplete 實現**」——本專案對調方向:本模型在 R₀ 片段應是 **rustc 行為的 sound under-approximation**(rustc 接受 ⇒ 我們不報錯 = L7a 的外延) |
| 5 | **MiniRust**([minirust/minirust](https://github.com/minirust/minirust),Jung;操作語義 = 直譯器規格) | 動態語義規範層;遠期若做 UB 面才接觸,先列 Tier-D 引用 |
| 6 | **差分測試系譜**:Csmith;[YARPGen](https://github.com/intel/yarpgen)(OOPSLA 2020 distinguished paper);[RustSmith](https://www.doc.ic.ac.uk/~afd/homepages/papers/pdfs/2023/ISSTA-tool.pdf)(ISSTA'23,Rust 首個端到端差分生成器,含借用/生命週期合法程序生成);[Rustlantis](https://research.ralfj.de/papers/2024-oopsla-rustlantis.pdf)(OOPSLA 2024,MIR 級隨機差分);Program Reconditioning(PLDI 2023) | P4-2 生成式差分的直接參照:RustSmith 證明「生成時保證借用合法」可行;Rustlantis 的 minimization 工作流與 shrink.rs 同構 |
| 7 | **Stacked/Tree Borrows + Miri**:[Stacked Borrows](https://dl.acm.org/doi/pdf/10.1145/3371109)(POPL 2020);[Miri: Practical UB Detection for Rust](https://research.ralfj.de/papers/2026-popl-miri.pdf)(POPL 2026) | 遠期 Tier-E(動態 oracle);近期只需知道「靜態 borrowck 的動態超集驗證」這一方法論存在 |
| 8 | **rustc 可靠性實證**:[Rust's Type Checker Implementation Is Unsound: An Empirical Study](https://arxiv.org/html/2608.28713)(2026;§6 給出「潛在 oracle」分類學,收 a-mir-formality 等) | 分歧三分法(RUSTC-BUG 類)的依據;多版本矩陣的必要性 |
| 9 | **規範層**:FLS 移交 rust-lang([2025h1 目標](https://rust-lang.github.io/rust-project-goals/2025h1/spec-fls-publish.html)、[LWN 報導](https://lwn.net/Articles/1015636/);RFC 3355);Ferrocene ISO 26262/IEC 61508 資格 | 「權威指標」的規範面:條文有段落編號可引用;R₀ 擴張時每一條語法/合法性規則可掛 FLS 條款號 |
| 10 | **語法層生態**:[ra_ap_syntax](https://docs.rs/ra_ap_syntax)(「快速增量重析 + 優雅錯誤處理 + 全保真表示」——與 L1/L3/L4/L7 設計目標逐條同構);[ra-ap-rustc_lexer](https://lib.rs/crates/ra-ap-rustc_lexer)(rustc_lexer 自動發佈,v0.155.0 @ 2026-03 仍在更新);[trybuild](https://github.com/dtolnay/trybuild)/ui_test | Tier-C 差分對象與工具;rowan 紅綠樹是 `reparse`/`ReuseData` 的成熟對照,值得逐條對讀(不一定採用,但必須對表) |

> 檢索備忘:Polonius 2026-07 有「接近穩定化」的產業報導(20,000 crates 實測 +1.4% 編譯時)——Tier-B 的 polonius facts 通道在未來兩三個 Rust 版本內可能變動,**這正是「只作 feature-gated 不入主門檻」的原因**。

---

## 四、Tier-A/Tier-B/Tier-C 接線細節

### Tier-A(首選,一切門檻建立在這層)

```text
命令面:rustc --edition=2021 --crate-type lib --emit=metadata \
            --error-format=json <input.rs>   (,-Zunstable-options 視需要)
輸出面:JSON 陣列 → 抽 {code, span:{line_start,column_start,line_end,column_end}, level}
規範化:錯誤碼集合(去重、排序)+ span 起訖(字節化,與本專案 Span 對齊)
環境:rustup toolchain 釘版(rust-toolchain.toml);CI 用 dtolnay/rust-toolchain@<pin>
```
- 无 rustc_private、无 nightly 依賴 → **stable 即可跑**,CI 主門檻用這層。
- `Accept` 判決以 `--emit=metadata`(不codegen)取,快且足夠。

### Tier-B(解剖刀,feature = "rustc-private")

- 樣板照 rustc-dev-guide:`run_compiler` + `Callbacks::after_analysis(|compiler, tcx| ...)`;
- 取數目標(按需漸進):
  1. `tcx.hir_*()` —— span 對照(語法層差分);
  2. `tcx.mir_borrowck(def_id)` —— 每函數 borrowck 結果(loan/region 事實)→ 與三軌區間**逐 loan 對帳**(這是語義層的「樣本點由行為生成」升級,直接落實 HARD-ITEMS #2 的方案 1);
  3. `-Zpolonius=legacy` facts → rust-lang/polonius datalog 引擎跑出 location-sensitive 判決 → 第四實作意見;
  4. `-Zunpretty=mir` 落盤 → 人審通道 + ORACLE-TRACE 附件。
- 紀律:**Tier-B 不進任何主門檻**;nightly job `continue-on-error`,產物只入 ORACLE-TRACE 附錄。API 逐版碎裂是預期內成本,釘版 + 單一間接層(`DriverOracle` 實現 trait)控制爆炸半徑。

### Tier-C(語法層差分)

- `r0_lex` vs `ra-ap-rustc_lexer`:token 種類/邊界逐一對表(trivia 保留策略、raw string、`>>` 拆分點);
- `r0_parse` 錯誤恢復 vs `ra_ap_syntax`:同一髒輸入,ERROR 節點覆蓋區間對照 → **L7 全化/極大化的外部標尺**(我們不要求一致,要求「差異可解釋、可入表」);
- rowan 紅綠樹 vs `parse.rs` 的 `reparse`/`ReuseData`:增量重析語義對讀報告(為日後若解封 L3/L4 準備彈藥)。

---

## 五、R₀ 語言面與語義層的深化路線(由差分數據驅動)

**原則:擴張順序不靠主觀,靠兩個數據源**——(a) 差分分歧的 MODEL-DIFF 熱點;(b) P4-6 真實語料的 `unsupported` 面實測。

語義層已知必改項(差分將立刻暴露,提前掛號):

| # | 深化項 | 現狀 | 幾何語義的升級 | 預期 rustc 對照 |
|---|---|---|---|---|
| S1 | **place 敏感度** | 衝突以 binding 為單位 | 區間攜帶 place 路徑;x.f1 與 x.f2 不相交 → 衝突圖從「綁定區間圖」變「place 區間圖」(仍是區間圖,區間按 place 分組) | E0506/E0499 細粒度接受 |
| S2 | **控制流精確 killer** | Nll 軌為線性掃描近似 | 區間端點取 CFG 上 liveness(分支/循環合流)而非文本序 | E0502 於 if/while 下的一致性 |
| S3 | **兩階段借用** | 無 | 可變借用區間允許「預留─啟用」兩段式 | rustc two-phase borrows(E0502 例外面) |
| S4 | **NLL problem-case 3** | Referent 軌大概率先拒 | 記 MODEL-DIFF;若日後模型升級至位置敏感,對照 polonius | polonius 接受/NLL 拒 |

語法層擴張候選梯隊(待數據確認):`match` 基礎模式 → `for`/迭代 → 閉包(捕獲)→ 泛型語法面(僅解析)→ `impl`/trait 語法面。每步走四連:EBNF 增補 → `unsupported` 面縮小 → 差分語料擴 → parity 不回退。

---

## 六、修法菜單(rep.rs)的 oracle 化

1. **µ 第二分量實測化:** `|Err_rustc(P')|` 由 CliOracle 現場查詢;L8 遞減律的測試從「(遞減, 0)」升級為「(遞減, 實測 rustc 錯誤數遞減)」。
2. **新增定律級測試(外律 O5):** Guarded 菜單每條規則的 fixture 現場:重寫輸出必須 (A) 通過本模型檢查,且 (B) **rustc 接受**(R₀ 片段內)。「修了真的能編譯」第一次成為機械事實。
3. **合流的外部驗證:** 不同修復順序到達的正規形,逐個過 rustc —— 「唯一正規形」獲得行為層佐證(仍非定理,如實申報)。
4. **誠實邊界(寫進 SPEC-TRACE):** 「rustc 接受」是修復正確性的**必要條件**,不是充分條件(語義保持/行為等價是更強命題,不在本輪宣稱範圍)。

---

## 七、與 Rocq 資產的關係:凍結,不是丟棄

- `rocq/`、`docs/ROCQ-*.md`、Maude/NaTT 外證全部保留;CI `rocq` job 改 `continue-on-error` + 標註凍結版;
- `SPEC-TRACE.md` R3/R5/R6/R7 行標「⏸ 凍結(2026-09-06 路線樞紐);窮舉與外證證據仍有效」;
- 回歸條件(明文化,避免「永遠不做了」或「偷偷又做」):若 P4 差分將 CT 菜單的語義穩定下來、且團隊有 XL 預算,可重啟 HARD-ITEMS #1 的降檔路線(peak decreasingness / 反射式有限證書)——那時鏡像的對帳框架(19 樣本點 + reconcile 腳本)直接復用;
- 沉澱:Rocq 對帳的紀律(樣本點 kernel 複驗、凍結序、故意漂移演練)**原樣遷移**到 ORACLE-TRACE——工具換了,紀律不換。

---

## 八、分階段計劃(P4 系列;完成標準 = 具名測試 + 門檻,律不過碼不合)

| 階段 | 內容 | 量 | 完成線(具名測試/文件) |
|---|---|---|---|
| **P4-0** oracle 腳手架 | `trait Oracle` + `CliOracle`(JSON 解析)+ rust-toolchain 釘版 + CI `oracle` job + `tools/oracle_gate.py` 骨架 | S | ✅ **已交付(2026-09-06,實測全綠)**:`oracle_smoke_e0502`、`oracle_determinism` + 追加 `oracle_accept_clean`/`oracle_reject_uncoded_syntax_error`/`oracle_expectation_grammar`(外律 O 系 ×5);種子語料 8/8;gate 判別力演練 ×4 情境全紅(`docs/ORACLE-TRACE.md`)|
| **P4-1** curated 語料 v1 + `ModelOracle` parity | ~100 案例:手寫 + rustc `tests/ui/borrowck` 精選改寫(合 R₀ 者照收);覆蓋 E0382/E0499/E0502/E0503/E0505/E0506/E0507/E0597;`docs/ORACLE-TRACE.md` 開表;分歧三分法入工具 | M | ✅ **已交付(2026-09-06,實測全綠)**:語料 39 案例(24a/14r/1u,碼由 rustc bootstrap 實測命名);`src/model.rs`(extract_r0 + 三軌判決 + parity 引擎);`oracle_parity_borrow_matrix`(BUG 類 0;MODEL-DIFF 24 項/7 家族 100% 歸檔附出處;**幾何保守下界定律**:借用衝突類碼 ⇒ Lexical 零漏報 9/9);附帶三大發現(CL0 事件真空/借鏈死代碼/Referent 終點 bug,見 ORACLE-TRACE §一)。插入 **P4-1a**:修 CL0 extract 真空 |
| **P4-2** 生成式差分 | `gen.rs` 語義感知升級(生成時即知期望判決,參照 RustSmith 的 by-construction 合法性);`fuzz.rs` 併軌:每輪樣本雙跑;失敗 shrink → `tests/fixtures/`(沿用既有防線) | M | ✅ **已交付(2026-09-07)**:`gen::gen_r0_case`(by-construction 期望 `GenExpect::{Accept,Reject(code)}`,family = Accept / E0503 / E0506 / E0499 / E0502,依 rustc 1.98.1 實測校準);`tests/oracle_laws.rs::oracle_fuzz_agreement`(O-7,400 案例、0 BUG 分歧、accept 137 / reject 263(4 碼全覆蓋)、nll+referent MODEL-DIFF 71 項統計入 REPORT)。`fuzz.rs` 保留為 CL0 律層(非 oracle);shrink 反例入庫循既有 fixtures 防線 |
| **P4-3** 語義深化第一梯 | S1 place 敏感 + S2 控制流 killer(§五);差分暴露的 BUG 修至 0;MODEL-DIFF 表更新 | L | `test_law_semantic_place_orthogonality`(x.f1/x.f2 不衝突)、`test_law_semantic_cfg_liveness`(if/while 下 killer 精確);parity 門檻不回退 |
| **P4-4** Tier-B 內嵌 | `DriverOracle`(feature-gated):HIR span 對照 → `mir_borrowck` 逐 loan 對帳 → polonius facts 第四意見(nightly job,不入主門檻) | M | `oracle_driver_loan_reconcile`(樣本點 ≥ 200,承 HARD-ITEMS #2「行為生成樣本」方案);`docs/ORACLE-TRACE.md` 附錄 |
| **P4-5** 修法 oracle 化 | µ 真值 + 修復接受率(§六) | M | `test_menu_repairs_accepted_by_rustc`(Guarded 全規則 × fixtures 100% rustc-accept);`test_law_L8_red_edge_decreasing` 升級雙分量實測版 |
| **P4-6** 真實語料抽樣 | crates.io top-N:函數級切片 → 是否落 R₀?`unsupported` 面實測報告 → 擴張排序數據化 | L | `docs/R0-COVERAGE-EMPIRICAL.md`(如實申報覆蓋率);擴張梯隊裁決記錄 |
| **P4-7** | R₀ 擴張第一梯(候選:基礎 `match`);Tier-C 語法差分常態化(r0_lex vs ra-ap-rustc_lexer 對表) | M/L | `r0_lex_vs_rustc_lexer`、`r0_recovery_vs_ra_syntax`(差異解釋率 100%,不要求相同);EBNF 增補 + unsupported 縮小 + parity 不回退 |
| 按需 | watch/LSP 演示(原 P2 #9/#10,不受樞紐影響);rocq 回歸(§七條件) | — | — |

**建議節奏(首兩個迭代):**
```
第五迭代:P4-0 + P4-1        → oracle 立起來,curated 百例parity 開表
第六迭代:P4-2 + P4-3 前半   → 生成式差分上線;place 敏感動工
```

---

## 九、風險與緩解(誠實版)

| # | 風險 | 緩解 |
|---|---|---|
| R1 | rustc_private API 逐版碎裂(Tier-B 維護稅) | Tier-A 為一切門檻;Tier-B 單一間接層 + 釘 nightly + job 不入主門檻 |
| R2 | 診斷/flag 漂移(`-Zpolonius*`、JSON 字段) | 比對只用錯誤碼+span;trybuild 式 overwrite 重刷紀律;nightly 探測腳本 |
| R3 | rustc 自身有 bug(2026 實證研究) | 分歧三分法含 RUSTC-BUG;stable×2+nightly 版本矩陣;案例隔離 |
| R4 | 「rustc 同意」被誤當「語義正確」 | MODEL-DIFF 類別保住模型主體性;修法面明寫「必要非充分」;對標 a-mir-formality 公理反向陳述 |
| R5 | 環境可重建(HARD-ITEMS #4 教訓) | rust-toolchain.toml 入庫;oracle 環境併入 `scripts/setup_dev.sh`;CI 與本地同腳本 |
| R6 | 零依賴原則被侵蝕 | oracle 全在 dev/feature 面;core 的 `cargo publish` 面依賴數不變(監督指標:Cargo.lock 的 lib 依賴數 = 0) |
| R7 | 差分門檻重蹈 bench gate 假紅/假綠 | oracle_gate 繼承 bench_gate 的統計紀律:先 null 對照(同輸入同版重跑飄移 = 0 才可信)、best-of-n、基線同境 `--update` |

---

## 十、學術定位(潛在論文敘事,供日後投稿決策)

「**區間幾何借用語義 × rustc 差分驗證 × 可合流修法菜單**」三角:

- Oxide 有 tested semantics(對 rustc 官方套例),但無修法層;
- Polonius/Stjerna 有算法與 graded 檢查,無幾何(區間圖/弦圖/完美性)敘事;
- a-mir-formality 是官方可執行模型,但差分工具鏈與外部用戶生態未成;
- Rustlantis/RustSmith 驗編譯器,不驗「模型」。

本專案可宣稱的位:**對 rustc 行為做差分驗證的幾何語義模型,且修法菜單的產物以 rustc 接受率為機械驗收**。CL0 九律 + ORACLE-TRACE 即證據鏈。

---

## 附:ROADMAP.md 增補段(建議 drop-in)

> **路線樞紐(2026-09-06)**:P3 #12 Rocq 認證主線 ⏸ 凍結(R3 一般性缺口 + 鏡像維護成本;資產保留,見 `docs/PIVOT-RUSTC-ORACLE.md` §七)。新主線 **P4:以 rustc/rustc_driver 為權威指標開發 R₀ 語義與語法層**——`|Err_rustc|` 從「記 0」變實測;九律不變,新增外律 O 系(parity/修復接受率);對帳表 `docs/ORACLE-TRACE.md`。第一迭代 = P4-0(腳手架)+ P4-1(curated 百例)。
