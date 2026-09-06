# P4 剩餘階段落地計劃(4.3 / 4.4 / 4.5 / 4.7)

> 2026-09-07 · 對應 `docs/PIVOT-RUSTC-ORACLE.md` §八與 §五。
> 先前完成:**P4-0**、**P4-1**、**P4-1a**、**P4-2(生成式差分,O-7)**、**P4-6(r0cov 覆蓋率實測)**。
> 本計劃只列**尚未交付**的 4.3 / 4.4 / 4.5 / 4.7,含:目標、依賴、風險、驗證標準、建議順序。
>
> **原則(全程不變):** 每步以「新增/強化某具名測試」為完成標準;「律不過,碼不合」;
> 任何改變 parity 解釋的工作,先重跑 `oracle_gate`,再決定重註冊或修模型;切勿硬做破壞綠門檻。

---

## 0. 現狀錨點(供對照)

- 受驗對象:`src/model.rs`(`extract_r0` → `ast::Facts` 三軌紅邊)+ `src/ast.rs`(`Event`/`conflicts`/`red_edges`/`intervals`)。
- 修法菜單:`src/rep.rs`(抽象區間層 `AState`,**無源碼錨**);`src/l9newman.rs`(Newman 通道)。
- 語法層:`src/r0.rs`(`R0_EBNF`/`r0_lex`/`r0_parse`/`unsupported`);覆蓋率工具:`src/bin/r0cov.rs`。
- parity:24 條已歸檔分歧(`corpus/PARITY-REGISTRY.json`,7 家族);門檻 O-6 + O-7。
- 已知模型邊界(見 `model.rs` 模組文檔 S1–S6):place 不敏感、NLL 無 CFG killer、
  兩階段借用缺、NLL problem-case 3、call-arg 借用無借鏈、不可變性屬型別面。

---

## 1. P4-3 語義深化第一梯(place 敏感 S1 + 控制流 killer S2)

**目標**(PIVOT §五 + §八):
- `test_law_semantic_place_orthogonality`(`x.f1`/`x.f2` 不衝突);
- `test_law_semantic_cfg_liveness`(if/while 下 killer 精確);parity 門檻不回退。

**這是最「傷筋動骨」的一段:** 目前衝突以 `binding` 為單位。要 place 敏感,得讓
事件攜帶 place 路徑、衝突圖在 place 粒度判。**風險**:curated 語料雖多數無字段,
但字段/索引案例一旦出現,24 條註冊分歧的解釋可能變,需「重跑 gate → 判定是
MODEL-DIFF(重註冊)還是 BUG(修模型)」。

### 建議切成可持續綠的 4 片(每片對應一具名測試,綠了才進下一片)

**Slice A — place 資料底板(不改語義)**
- 新增 `ast::Place`(遞歸:根部 `Binding` + 欄位名 `Vec<String>`;或做輕量
  `PlacePath = (binding, Vec<Segment>)`),`Event` 加 `place: Place`,現有
  產生點(`extract_r0`/CL0 `ast::extract`)一律填**根部** place。
- 語義零變化(place 恆為根 ⇒ 衝突判據不變)。
- 完成測試:`test_law_semantic_event_carries_place`(事件帶 place;字段訪問
  `x.f` 攜帶 `f` 片段)。此片**純資料**,應 0 行為改變,gate 必綠。

**Slice B — 字段感知事件 + place 重疊判據**
- `extract_r0`/`walk_flat` 對 `x.f` 產生 place = 根 `x` + 片段 `f`(現已
  偵測 `prev_is_dot`,只需累積片段而非跳過)。
- 定義 `Place::overlaps`(一為另一的前綴 ⇒ 重疊;`x` 與 `x.f` 重疊,`x.f1`
  與 `x.f2` 不重疊)。
- `conflicts`/`red_edges`:同一 binding、place **overlap**、且相容性違反 ⇒ 紅邊。
- 完成測試:`test_law_semantic_place_orthogonality`(`&mut x.f1` + `x.f2` 讀
  → 不衝突;`&mut x.f1` + `x.f1` 讀 → 衝突)。
- **注意**:curated/generated 語料目前皆無字段寫/borrow,故 parity 應不變;
  若 `r0cov`/新增案例暴露新分歧,走「模型主體性」判定歸入 MODEL-DIFF。

**Slice C — place 敏感借鏈 + 兩階段借用佔位**
- 借鏈 `BorrowLink` 帶 place;`intervals` Referent 軌對 place 細化。
- 完成測試:`test_law_semantic_borrow_place_link`(`let r = &mut x.f1;` 借鏈
  落在 place `x.f1`)。
- 兩階段借用(E0502 的 Two-phase borrow 例外)此片**只掛號**(S3),不宣稱。

**Slice D — 控制流精確 killer(S2)—— 獨立、較大**
- 目前 NLL 軌是「線性掃描找下一個 Decl/Move」。改為取 CFG 上的 liveness
  (if/while/loop 合流點)。需要 R₀ 控制流節點 → 基本區塊 → def/use。
- 完成測試:`test_law_semantic_cfg_liveness`(while 回邊、if 分支下 killer 精確)。
- 風險:會改變 NLL 軌對現有 while/if 語料的判決 → 重跑 gate,判定 MODEL-DIFF
  或修模型。此片建議作為 P4-3 的**收尾獨立里程碑**。

**工作量**:A=B+ C = M;D = L。**先做 A–C(place),單獨成一 commit;D(CFG)另成 commit。**

---

## 2. P4-4 Tier-B 內嵌(`DriverOracle`)

**目標**(PIVOT §四 Tier-B + §八):`feature = "oracle"` 之外再開 `feature = "rustc-private"`;
`run_compiler` + `Callbacks::after_analysis` 取 `tcx.hir_*` / `mir_borrowck` / `polonius` facts;
完成測試:`oracle_driver_loan_reconcile`(樣本點 ≥ 200,承 HARD-ITEMS #2)。

**‼️ 環境硬約束:** 需要 **nightly + `rustc_private`**。本環境僅 stable,`rustc_driver`
不可用。**此階段硬體上無法在本環境實作/驗證。** 可交付的部分(誠實標記 blocked):

- 在 `src/oracle.rs` 增加 `DriverOracle` `trait` 的**型別骨架**(`feature = "rustc-private"`,
  方法簽名 + 文件注,`#[cfg]` 隔離;不編譯實作)以鎖定接口;
- `rust-toolchain.toml` 預留 nightly 釘版註記;
- CI 加 `oracle-tierb` job(僅 nightly,`continue-on-error`),初期即紅即擋。
- **驗證標準**:骨架 `cargo check --features "oracle rustc-private"` 在 nightly 上通過;
  stable 上此 feature 不存在(不被編譯)。
- **建議**:此段屬「解剖刀」,不進任何主門檻;在 P4-3 語義穩定後、且有 nightly CI
  再啟動,才不會被 API 逐版碎裂拖垮。

---

## 3. P4-5 修法菜單 oracle 化(µ 第二分量 + 修復接受率)

**目標**(PIVOT §六 + §八):µ = (|E_red|, |Err_rustc|);`test_menu_repairs_accepted_by_rustc`
(Guarded 全規則 × fixtures 100% rustc-accept);`test_law_L8_red_edge_decreasing` 升級雙分量實測版。

**核心缺口 / 依賴:** 修法菜單目前在**抽象區間層**(`AState`),**沒有對應回源碼的錨**。
要問 rustc「修完能不能編譯」,必須先把「一筆修法 → 對源碼的編輯/代碼建議」作映射。
**這是本階段唯一的硬前置,屬 XL。**

**前置工程(必需才有意義):**
- 為 `AState`/`Rule` 增加**源碼錨**:`rep::export_repair(state, rule) -> Option<SourceRepair>`;
  `SourceRepair` 描述「對 `src` 的編輯」(如縮短借用區 → 把 `&mut x` 移到作用域末、
  或插入 `{ }` 縮小作用域)。要讓 R₀ 源碼能對應到 `AState`,需 `extract_r0` 反饋
  「事件 ↔ 源碼 span ↔ storage」映射(`Event` 已帶 span;再結合成 `storage` 分組)。
- 這是把「幾何手術」翻譯回「文字修改」的橋,亦是 P2 watch/編輯器層共同地基。

**建議也切片:**
- **Slice 1(橋骨架 + 單一規則 R1Shorten)**:`rep::repair_to_edit(state, R1Shorten)` →
  對應的最小文字編輯(縮短一個借用事件的存活區 = 在源碼加一個作用域/重排)。
  完成測試:`test_repair_R1_shorten_has_source_edit`(R1 規則產一念合法的源碼編輯)。
- **Slice 2(接受率)**:給一批 R₀ fixtures(借用衝突現場),跑 `Menu::CommutativeTrim`
  得正規形,把其文字修復丟 `CliOracle`,斷言 **100% rustc-accept**(修了真的能編譯)。
  完成測試:`test_menu_repairs_accepted_by_rustc`。
- **Slice 3(µ 第二分量實測)**:`rep.rs::measure` 由「記 0」改為實測
  `|Err_rustc(P')|`;`test_law_L8_red_edge_decreasing` 升級雙分量版。
- 誠實邊界(寫死):「rustc 接受」是修復正確性的**必要非充分**條件(語義保持是更強命題)。

**工作量**:前置橋 XL;最終可交付最小閉環 `test_menu_repairs_accepted_by_rustc`
(Gated 單一規則 × fixtures)= L。

---

## 4. P4-7 R₀ 擴張第一梯(基礎 `match`)+ Tier-C 語法差分

**目標**(PIVOT §八 P4-7):EBNF 增補基礎 `match` → `unsupported` 面縮小 → 差分語料擴 →
parity 不回退;完成測試 `r0_lex_vs_rustc_lexer`、`r0_recovery_vs_ra_syntax`(差異解釋率 100%)。

**自主性最高、風險較可控的一段**(新增文法/解析,獨立可測,不動受驗模型的 parity 解釋)。
分兩子項:

**4a — R₀ `match` 子集(純語法面)**
- 依附錄 B 側條件擴 `R0_EBNF`:加入 `match_expr = "match" expr "{" { arm } "}"`,
  `arm = pattern "=>" expr ("," | ";")`,pattern 先收縮到**基礎**:
  `IDENT | 字面量 | "_" | 元組`(不含範圍/守衛/OR 模式,守衛等列 unsupported)。
- `r0_lex` 增 token(如 `=>`、`|`,需與閉包 `|` 區分 —— 側條件:無閉包,`|` 為
  模式分隔);`r0_parse` 增 `MatchExpr`/`MatchArm` 節點;`unsupported` 對超出子集的
  pattern 如實申報。
- 完成測試:`test_law_r0_match_parse`(match 正析 + 無損回環 + `unsupported` 對
  守衛/OR 的如實申報)+ `r0cov` 覆蓋率在自製 match 樣本上不再下滑。
- **依賴**:`r0cov` 已就緒,可直接看 unsupported 面縮減。

**4b — Tier-C 語法差分(外部對照)**
- `r0_lex` vs `ra_ap_rustc_lexer`(`r0_lex_vs_rustc_lexer`):token 種類/邊界對表
  (trivia 保留、raw string、`>>`/`>>=` 拆分點 —— 側條件內無泛型,故 `>>` 恆為移位)。
- `r0_parse` 恢復 vs `ra_ap_syntax`(`r0_recovery_vs_ra_syntax`):同一髒輸入,ERROR
  覆蓋區間對照;不要求一致,要求「差異可解釋、可入表」。
- 此子項需外接 dev-dependency(`ra-ap-rustc_lexer`/`ra_ap_syntax`),**僅 dev/feature 面**,
  不進 core 發佈面(零依賴原則);CI 獨立 job(可選 `continue-on-error`)。
- **依賴**:需網路可拉取 dev-dependency;本環境可嘗試,但屬外部工具鏈,建議獨立 job。

**工作量**:4a = L(新增文法/解析器/unsupported);4b = M(依賴外部 crate + 對表)。

---

## 5. 建議執行順序(優先矩陣)

| 序 | 階段 | 子項 | 依賴 | 風險 | 工作量 | 本環境可行性 |
|---|---|---|---|---|---|---|
| 1 | **P4-3** | Slice A(place 資料底板) | 無 | 低(純資料) | S | ✅ |
| 2 | **P4-3** | Slice B(place 判據)+ `place_orthogonality` | A | 中(可重註冊 MODEL-DIFF) | M | ✅ |
| 3 | **P4-3** | Slice C(place 借鏈) | B | 中 | M | ✅ |
| 4 | **P4-7** | 4a:基礎 `match` + `unsupported` 縮小 | `r0cov` | 低–中 | L | ✅ |
| 5 | **P4-5** | Slice 1(源碼修復橋 + R1) + Slice 2(接受率) | `repair_to_edit` | 高(L 前置) | L | ✅(需大量工作) |
| 6 | **P4-3** | Slice D(CFG killer)`cfg_liveness` | B/C | 高(改 NLL 判決) | L | ✅ |
| 7 | **P4-7** | 4b:Tier-C 差分(外部 crate) | dev-dep + 網路 | 中 | M | ⚠️ 部分(需拉外部 crate) |
| 8 | **P4-4** | Tier-B 骨架 + nightly CI | **nightly + rustc_private** | 高(API 碎裂) | M | ❌ 本環境穩定,僅骨架 |

**建議**:先做 1–3(P4-3 place,成一個 commit),再做 4(P4-7 match,成一個 commit);
P4-5(5)依賴源碼橋,建議在 place 落地後接;C D(6)獨立;P4-7(7)與 P4-4(8)在
有 nightly/外部依賴的環境再啟動。

---

## 6. 每一片的「律不過碼不合」驗證清單(統一)

- [ ] 新增/強化**具名測試**(上列 `test_law_*` / `test_menu_*` / `test_oracle_*`);
- [ ] `cargo test --all`(47 基線)不回退;`cargo test --features oracle`(65)不回退;
- [ ] `cargo clippy --all-targets --features oracle -- -D warnings` 0;
- [ ] 若涉語義/parity:重跑 `python3 tools/oracle_gate.py corpus/curated corpus/BASELINE.json --parity`,
      確 **BUG 候選 0**;分歧若為 MODEL-DIFF → 入 `PARITY-REGISTRY` 附出處,若 BUG → 修模型;
- [ ] 若涉語法面:`r0cov` 在自製樣本覆蓋率不倒退;
- [ ] 每成一步即 `git commit` + `push g1`(保持 `g1` 可追踪、可回退)。
