# cl0r0 專案現況分析與後續開發路徑

[![CI](https://github.com/djrww/coreE9/actions/workflows/ci.yml/badge.svg)](https://github.com/djrww/coreE9/actions/workflows/ci.yml)

> 基線:`v0.1.1`(main @ 003e37c)· rustc 1.98.0 · 5,583 行 · 零依賴 · 第一迭代已完成(見 §四「建議節奏」)

---

## 一、現況盤點(六維快照)

| 維度 | 狀態 | 證據 |
|---|---|---|
| 代碼質量 | ✅ 測試/靜態全綠(bench 面 ⏸ 暫時清退,2026-09-07)| `cargo test` **53 全綠**(除錯批次新增 6 個 R₀ 回歸,見 DEBT-REVIEW 2026-09-07)· clippy `-D warnings` 0 · fmt clean · rustdoc `-D warnings` 0 · coverage gate ok · oracle gate 39/39(P4-1 parity)· **bench gate:⏸ 清退**(2026-09-07;方法論留 git 史:`best-of-n + null 校正` 仍為未來重啟的判據) |
| 九律覆蓋 | ✅ 主體完成 | L1–L9 + M1/M2/M4/M5 + T2 + R₀ 語義面 + 反例縮小:**47 具名測試**(見 SPEC-TRACE §〇);L9b′ 平行的精確交換已入陣 |
| R₀ 載體 | ⚠️ 部分 | 詞法 `r0_lex` + `lalr1_clean` + `unsupported` 齊;**缺完整 `r0_parse`(CST 樹產出)** |
| 重寫系統 | ✅ 完整 | §4.2 良基測度 μ(L8)+ §4.3 臨界對/L9 Newman 通道(含 Naive 反例對照) |
| 增量層 | ⚠️ 停滯 | `reparse`/`ReuseData`/`ReparseOut` 已實現;L3/L4 依規格約定**排除**在律級斷言外 |
| 工程化 | ⚠️ 弱 | 無 CI、無覆蓋率門檻、無基準測試、發布為手動 |

**已知的誠實缺口(應寫入規格對帳,不應假裝覆蓋):**
1. `test_law_L7_error_totalization`(任何輸入皆出樹的全化律)在 L7 段重寫時被合併吸收 —— 目前僅由 `parse` 設計 + `smoke_parse_garbage` + fuzz 隱式覆蓋,**無獨立具名測試**。
2. R₀ 無解析器:`R0TokKind` → 樹的通道缺失;`unsupported` 目前是詞法級,而非「節點級精確」。
3. L9 反例通道(Naive/Raw)是「機器找反例」而非「機器證明無反例」—— 與規格 §9 判定權不轉移一致,但報告措辭需保持如實。

---

## 二、後續開發路徑(按優先級)

### P0 — 理論閉環(把九律做滿、把缺口補平)

| # | 工作項 | 內容 | 工作量 |
|---|---|---|---|
| 1 | **重建 L7 全化具名測試** | `test_law_L7_error_totalization`:任意輸入(含非法字節/垃圾桶/深嵌套)→ `parse` 必回 `Ok`,且 ERROR 節點 span 連續性成立 | S |
| 2 | **R₀ 完整解析器 `r0_parse`** | 依附錄 B/§7.2:EBNF → CST(`R0Tree { src, nodes }`),同 CL0 的 span/連續性/laminar 紀律;`unsupported` 精確到節點(規則/位置),為 §9「如實申報」提供樹級證據 | L |
| 3 | **fuzz 最小化(shrinking)** | 失敗時自動縮小到最小反例(現僅報統計);反例進 `tests/fixtures/` 防回歸 | M |
| 4 | **L9 並行化 + 更大空間** | `enumerate_states` 常數現為 3 事件/4 座標;以 Rayon 或分塊並行推 4 事件 × 6 座標,強化 Newman 通道規模 | M |

> 註:L3(增量重析等價)/ L4(編輯單體相容)依你的規格決定**排除**;若未來解除,`reparse`/`compose` 基礎設施已就緒,只需補等價測試即可激活 —— 這是低成本的「可選後門」,不需現在做。

### P1 — 工程化(讓自証成為持續紀律,而非一次性成果)

| # | 工作項 | 內容 | 工作量 |
|---|---|---|---|
| 5 | **GitHub Actions CI** | push/PR 自動跑:`fmt --check` → `clippy -D warnings` → `cargo test` → `RUSTDOCFLAGS=-D warnings cargo doc` → 九律矩陣即驗收合同;附 badge | S |
| 6 | **覆蓋率門檻** | `llvm-cov`(llvm-tools 已裝)產出報告 + 閾值(建議核心模塊 ≥ 90%);覆蓋率表進 `docs/` | S |
| 7 | **基準測試** | `criterion`(或維持零依賴自研計時器)針對熱點內核:`parse`/`lex`/`laminar_ok`/`newman_check`;進 CI 做 ±5% 回歸門 | M |
| 8 | **規格 ↔ 測試對帳矩陣** | `docs/SPEC-TRACE.md`:規格章節(§1.2 位址映象 / §2.3 L7 / §3.1 L5 / §4.2 L8 / §4.3 L9 …)→ 具名測試 → 代碼符號,一表窮盡;「律不過,碼不合」的字面落實 | S |

### P2 — 應用化(讓庫被真正用起來)

| # | 工作項 | 內容 | 工作量 |
|---|---|---|---|
| 9 | **watch 層原型** | 編輯單體 M5 的實用演示:並行編輯去抖歸併(`compose_seq`)+ `reparse` 增量重析,一個 `bin/demo` 展示「一次按鍵 = 一次增量重析」 | M |
| 10 | **最小 LSP(僅診斷)** | 已有 span 族 + ERROR 節點:輸出 `textDocument/publishDiagnostics` 已足夠 demo;含 cli `bin/cl0r0 --check file` | M |
| 11 | **cargo publish / 對外庫化** | 若願公開:`cl0r0` 作為庫(crate metadata + 文檔首頁示例),版本語義 0.2 起 | S |

### P3 — 理論深化(學術級)

| # | 工作項 | 內容 | 工作量 |
|---|---|---|---|
| 12 | **形式化證明** | Lean/Coq/Rocq 重證核心:Newman 引理(現為測試級)、L7b 迭代淨化於切縫終止、T2 區間圖完美性;與 Rust 側對帳 | XL | **進行中(路線 A)**:`docs/ROCQ-PLAN.md`(計劃)+ `docs/ROCQ-TRACE.md`(對帳)。**Phase 0–2 ✅**(2026-09-02):鏡像(Mirror.v,D1–D6)+ 對帳框架(19 樣本點 kernel 複驗,抓出並修正 R4 runtime 順序分歧)+ 抽象 Newman(R1/R4)+ **具體 SN(R2:CommutativeTrim × Guarded 之 step_ct 強正規化,µ=|E_red|)**。**Phase 3(R3 具體 WCR → 任意狀態合流)進行中(2026-09-02)**:文獻搜查 + 探針前測見 `docs/R3-RESEARCH.md`(精確交換在 2,443,506 對上全綠;Guarded≡Raw;紅邊單調);Rocq 輔助層 `rocq/theories/WCRUtil.v`(`trim1` 成對歸納三引理 + 修剪語義的 vm_compute 事實)已入庫並掛進 `make`;**原計劃的「start 不變 ⇒ 他人 cut 不變」捷徑已被否證**(`ct_pred` 含 `istart a <? iend b`),主定理 `ct_join_exact`/`R3_ct_wcr`/`R4_ct_confluent` 仍為開放,詳 `docs/ROCQ-TRACE.md` §一-R3、`docs/ROCQ-PLAN.md` §三-R3 |
| 13 | **定律語義化報告** | 把 fuzz 的統計型檢查升級為「生成式證明」:每輪記錄證人,匯出 `docs/REPORT.md` 機器可讀 | L | **已裁決(非 Rocq 前置)**:擱置;理由見 ROCQ-PLAN §5.1;極小版可排 Phase 5 後 |

---

## 三、建議節奏

```
第一迭代 ✅(110b1ff):P0 #1 #2 + P1 #5 #8   → 補平誠實缺口,CI 固化成紀律
第二迭代 ✅(0950906):P0 #3 #4 + P1 #6 #7   → 反例最小化 + 規模擴張 + 門檻
第三迭代 ✅(b904921):P3 #12 Phase 0–2   → 鏡像 + 抽象 Newman + 具體 SN + 對帳框架
第四迭代 ⏸(2026-09-06 凍結):P3 #12 Phase 3(R3 WCR)→ 前測/Rocq 輔助層/外部
                  交叉驗證已入庫;因「窮舉≠定理」一般性缺口 + 鏡像維護成本,
                  主線樞紐至 P4(見 §五;資產保留,rocq job 轉 continue-on-error)
第五迭代 ✅(2026-09-06):P4-0 oracle 腳手架 → Tier-A 判決通道(CliOracle,
                  feature 隔離、零新依賴)+ 外律 O 系 ×5 + 種子語料 8/8
                  + oracle_gate(兩趟全等/基線對帳;判別力演練 ×4 情境全紅)
第六迭代 ✅(2026-09-06):P4-1 語義接線 + parity → `src/model.rs`
                  (extract_r0:R₀ 樹→事實層→三軌紅邊)+ 語料 8→39(碼由
                  rustc bootstrap 實測命名)+ PARITY-REGISTRY(24 分歧/7 家族
                  全歸檔)+ 主律 oracle_parity_borrow_matrix(含幾何保守下界
                  定律:借用衝突類碼 ⇒ Lexical 零漏報,9/9);
                  **三大發現**(ORACLE-TRACE §一):CL0 語義面事件真空 /
                  借鏈死代碼 / Referent 終點下限錯
第六迭代b ✅(2026-09-06):P4-1a CL0 事件真空修復 → ast.rs 同構修復
                  (decl_site + 遍行期作用域 + 借用事件/借鏈 span)+ 兩具名測試
                  補反真空斷言(實測校準);47/47 綠,T2 首食真數據,parity 不變
第七迭代 ✅(2026-09-07):P4-2 生成式差分 → `gen::gen_r0_semantic`(by-construction
                  期望判決,參照 RustSmith 合法性-by-construction)+ `fuzz` 併軌
                  雙跑 + 外律 O-7 `oracle_fuzz_agreement`;2000 輪 0 失配/0 範圍外
                  (rustc 1.98.1;統計見 ORACLE-TRACE §七)
按需            :P2 #9 #10                → 增量編輯器 demo / LSP,對外可用
```

第八迭代 ✅(2026-09-07):P4-3 語義深化 → `ast::intervals` 點事件化(Move/Read/
Deref)+ linked borrow end = 引用最後使用 + call-arg dies_at = 呼叫右括號末 +
S2 回邊活性(僅引用在迴圈內被用時跨越)+ place 敏感(field 鏈)+ 型別面
(TypeClass Int/Ref/Unknown;Ref 讀 = Move+consumes → dead-use 紅邊)。
具名律 O-8/O-9 雙錨;gate 41/41(註冊表 24→4,餘 F-G 範圍外);fuzz 2000 輪
gate over/under = (0,0)/(0,0)。詳 `docs/ORACLE-TRACE.md` §八。

**原則**:每一步都以「新增/強化某條具名測試」為完成標準 —— 律不過,碼不合。

---

## 五、路線樞紐(2026-09-06):P3 #12 ⏸ 凍結 → 新主線 P4(rustc 為權威指標)

> 完整藍圖與文獻坐標見 **`docs/PIVOT-RUSTC-ORACLE.md`**;對帳表見
> **`docs/ORACLE-TRACE.md`**。本節只留決策摘要。

**為何樞紐**:R3 的一般性缺口(窮舉 ≠ 定理,捷徑已被否證)+ 鏡像漂移的雙語言
永久稅(HARD-ITEMS #2),使 Rocq 主線的 XL 投入風險與回報不再成正比。反之,
`rep.rs` 的測度本就寫著 `μ = (|E_red|, |Err_rustc|)` 而 rustc 面「記 0」——
新主線就是把這個佔位符**激活成每次可實測的機器查詢**。判定權仍不轉移
(分歧三分法:BUG / MODEL-DIFF / RUSTC-BUG),只是裁判從 Rocq kernel
(證明權威)換成 rustc 本體(行為權威)。

**凍結不丟棄**:`rocq/`、ROCQ-*.md、Maude/NaTT 外證全部保留;rocq CI job 轉
`continue-on-error`;回歸條件見 PIVOT §七(差分穩定 CT 語義 + XL 預算時重啟)。

**P4-0 已交付(2026-09-06,實測全綠)**:`src/oracle.rs`(feature = "oracle",
零新依賴)· `tests/oracle_laws.rs` 外律 O 系 ×5 · `corpus/curated/` 種子 8 案例
8/8 · `tools/oracle_gate.py` + `corpus/BASELINE.json` · CI `oracle` job ·
`rust-toolchain.toml`。九律載體一字未動(`cargo test --all` 47 條全綠)。

**P4 後續**(PIVOT §八):P4-1 curated 百例 + `ModelOracle` parity + 版本矩陣 →
P4-2 生成式差分 → P4-3 語義深化(place 敏感 / CFG killer)→ P4-4 Tier-B
rustc_driver 解剖 → P4-5 修法 oracle 化(µ 第二分量實測)→ P4-6 真實語料 →
P4-7 R₀ 擴張 + Tier-C 語法差分。
