# ROCQ 形式化對照表(ROCQ-TRACE) — Phase 0–3 對帳文件(2026-09-02 更新)

> Phase 0–3 對帳文件(2026-09-02 更新:R3 前測 + 鏡像決策 D7)。主計劃見 `docs/ROCQ-PLAN.md`,難度計劃見 `docs/HARD-ITEMS.md`。
> 本檔 = 「Rocq 定理 ↔ 律 ↔ 具名測試 ↔ 鏡像決策」的機械對照與現況。
> 紀律:每條 Rocq 陳述必須能指回 Rust 側的律/測試;反之亦然。

---

## 零、環境(可重建性)

| 工具 | 版本 | 安裝方式 |
|---|---|---|
| coqc | 8.20.1(Debian trixie 套件;官方 9.x 為後備,敘述不變) | `apt-get install coq libcoq-mathcomp-ssreflect` |
| mathcomp-ssreflect | 2.3.0 | 同上 |
| rustc / cargo | 1.98.0 stable | `rustup-init`(沙箱快照不含 ~/.rustup/工具鏈與 .cargo/bin 符號連結,換環境須重建:`scripts/setup_dev.sh`) |

---

## 一、定理現況

| 編號 | Rocq 定理 | 檔案 | 狀態 | Rust 側對應(證人) |
|---|---|---|---|---|
| R1 | `newman : sn r -> wcr r -> confluent r` | `rocq/theories/AbstractArs.v` | ✅ 已證(2026-09-02) | L9a/L9b/L9c 窮舉 |
| R4 | `newman_unf`(唯一正規形) | 同上 | ✅ 已證(推論) | L9a |
| R4′ | `exists_normal_form`(正規形存在) | 同上 | ✅ 已證 | L8b(μ 保證終止) |
| R2 | 具體 SN:`sn step_ct`(CommutativeTrim × Guarded)| `rocq/theories/ConcreteSN.v` | ✅ 已證(2026-09-02)| L8a/L8b(窮舉) |
| R3 | 具體 WCR(交換引理 + 臨界對)— **數學核心** | `rocq/theories/WCRUtil.v`(含 ③ fold_min_mem/cut_for_gt_start ✅)+ `rocq/theories/ConcreteWCR.v`(不變量機器 ✅ + ⑥⑦⑧ 交換/規則存活/主定理 ✅)| 🟢 **Raw 版完成**(2026-09-03 第四輪):`R3_ct_wcr_raw`(uniq_ids 前提,wf 不需要)kernel 驗證、Print Assumptions 潔淨;🔶 Guarded 版 WCR 與 R4_confluent 仍未證 | L9b 窮舉(4×6 共 623,616 狀態 × 635,424 臨界對,0 違反)+ 前測「精確交換」已由測試級晉升定理級(R3_ct_wcr_raw) |
| R5 | L7b 迭代淨化終止 + 不動點 | — | ⬜ Phase 4 | L7b / `l7b_evaluate` |
| R6 | T2 χ = ω(max_overlap = greedy_chromatic)| — | ⬜ Phase 5 | `test_theorem_T2_interval_graphs_are_perfect`(400 樣本) |
| R7 | NaiveMenu 反例存在性 | — | ⬜ Phase 6(選配)| L9c 機器反例 |

**證明提要(R2)**:`guarded_apply_decreases`(Guarded 合法施用 ⇒ sd(µ′,µ)=true,
由 applicable 之 filter 內涵)、`sd0_lt`(第二分量恆 0 ⇒ 化為 |E_red| 遞減)、
`step_measure_lt`(單步 ⇒ 紅邊計數嚴格遞減)、`R2_sn_step_ct`
(以 `well_founded_lt_compat` 由 nat 良基導出 SN)、`ct_guarded_has_nf`
(R4′ × R2:每狀態有有限規約路徑達正規形;唯一性待 Phase 3 合流)。
對帳新增 `ct_step_measure=0`(canon 之 Guarded 首步紅邊歸零,Rocq/Rust 一致)。

**證明提要(R1)**:Huet 式。`star`(反射傳遞閉包)/ `joinable` / `wcr` / `confluent` /
`nf` 定義於 `AbstractArs.v`;`sn r := well_founded (fun x y => r y x)`(Coq 的
Acc 反向歸納恰給「∀a′, r a a′ → P a′」的推進 IH)。
主定理在 `Acc` 上同時歸納兩個性質:Q a(一步 vs 多步,「strip」)與 P a(多步 vs
多步);wcr 供 Q 的步進,IH 供 P 的步進。構造式,未用排中律(唯
`exists_normal_form` 用 `classic` 判別「是否存在一步」)。

---

### R3 前測(2026-09-02,`examples/r3_probe.rs` + `examples/r3_swap.rs`)

| 宇宙 | 狀態 | 不同後繼臨界對 | start 不變 | 精確交換 | Guarded≡Raw | 紅邊增加 |
|---|---|---|---|---|---|---|
| 3×6(+過濾)| 35,280 | 10,668 | 0 違反 | 10,668/10,668 | 0 偏離 | 0 |
| 4×5(+過濾)| 105,216 | 100,392 | 0 違反 | 100,392/100,392 | 0 偏離 | 0 |
| 4×6(+過濾,=CI)| 623,616 | 635,424 | 0 違反 | 635,424/635,424 | 0 偏離 | 0 |
| 4×6(**無過濾**)| 3,111,696 | 2,443,506 | 0 違反 | 2,443,506/2,443,506 | 0 偏離 | 0 |

推論(寫進 Phase 3 的定理陳述):
* `ct_join_exact`:兩步不同後繼時,`apply (apply s ra) rb = apply (apply s rb) ra`(等式級,非 merely joinable);
* `ct_guard_redundant`:CT 上 `applicable s CT Guarded = applicable s CT Raw` ⇒ µ 遞減是定理而非假設;
* `ct_red_edges_mono`:任何 CT 步不增紅邊數。
**鏡像決策 D7(新增)**:`enumerate_states` 不施加 distinct-start 過濾(過濾屬 Rust 側
規模控制,見 `src/l9newman.rs`);前測顯示定理在此更強宇宙仍成立 ⇒ 鏡像保持無過濾,
Rocq 定理的 ∀ 陳述亦不需該假設。

### R3 Rocq 開工(2026-09-02,`rocq/theories/WCRUtil.v` 入庫)

新增輔助層並掛進 `make -C rocq`(`THEORIES` 已加入 `theories/WCRUtil.v`)。
**已 kernel 驗證**(無 `Admitted`/`Axiom`,`make -C rocq` 全綠):

| 事實 | 內容 |
|---|---|
| `trim1_spec` | `trim1 i c l (trim_at l i c)` —— 用**成對歸納**精確刻畫修剪 |
| `trim1_length` | 修剪不改變列表長度 |
| `trim1_nth_other` | `k <> i ⇒ nth_error l' k = nth_error l k`(非修剪位逐字相同)|
| `trim1_nth_here` | **本輪移除** — 想寫成 `nth_error l i = Some e → nth_error l' i = Some (trim_ev e c)`,但 `H : trim1 i c l l'` 同時依賴 `l` 與 `l'`,Coq 8.20 拒絕任何單邊 `revert`;改由 `trim1` 的**構造**直接讀取(見檔內註記) |
| `cut_for_is_filter` | `Mirror.cut_for l a` = `fold_left … (filter (ct_pred a) l) None`(`reflexivity` 級,把鏡像實作與候選謂語**釘死**) |
| `ct_pred_start_lt` | `ct_pred a b = true → istart a < istart b`(候選集元素的起點**嚴格更晚** —— 這是 `cut_for > istart` 的引擎) |
| `trim_at_trim_at_here` | 同位重剪:`trim_at (trim_ev e c :: t) 0 d = trim_ev e d :: t` |
| `andb_l`/`andb_r`/`peel` | bool 合取拆解小件(鏡像 `ct_pred` 是五層 `&&`)|
| 三個 `vm_compute` 事實 | `ct_pred evA evB = true`;`ct_pred (trim_ev evA 2) evB = false`;`ct_pred evA (trim_ev evB 1) = true` |

**本輪最重要產出是「否證」**:原計劃 R3 的捷徑「start 不變 ⇒ 他人 `cut_for`
不變」為**假** —— `Mirror.cut_for` 的候選謂語含 `istart a <? iend b`,故被剪者
會從他人的候選集消失(上表第 6 列即此事實的計算證據)。改走的路線與剩餘缺口
見 `docs/ROCQ-PLAN.md` §三-R3「開工實測」。

### R3 第二輪(2026-09-02,良構性問題 + `fold_left` 障礙)

**新增探針 `examples/r3_wf.rs`(含自我否證)**:我上一輪推測「剪 a 到 cut 即把
a 從他人候選集移除」需要區間良構性 `istart ≤ iend`,並想用探針確認。第一版**失敗
且失敗得有價值** —— 它只跑 `enumerate_states`,而該生成式是 Rust
`for end in (start+1)..=max_coord` / Rocq `ends_for m s := iend := s + S d`,
**由構造排除**倒掛區間(實測 n=3 m=5:27,000 狀態、含倒掛 **0**)⇒ 「倒掛桶」恆空,
那個「全綠」對問題本身是**套套邏輯**。我沒有接受它,改寫成第二節**自行枚舉允許倒掛**
的宇宙(end 取 `0..=m`,不要求 `> start`):

| 宇宙(允許倒掛) | 狀態數 | 含倒掛 | CT peers | 非精確交換 |
|---|---|---|---|---|
| n=3 m=3 | 32,768 | 24,768 | 387 | **0** |
| n=3 m=4 | 125,000 | 98,000 | 2,259 | **0** |
| n=3 m=5 | 373,248 | 299,160 | 8,757 | **0** |

⇒ **倒掛樣本不破壞精確交換**(實測層面 wf 前提可能不必要)。但要注意:**機制不同** —
手工算 a=[5,2)、b=[0,10) 可知「被剪者從候選集消失」這條捷徑在倒掛下**仍為假**;
全綠來自别的候選把 min 撐住,不是來自捷徑成立。所以**不能**把探針結果寫成定理前提,
Rocq 側仍需「候選集變化 ⇒ min 不變」的刻畫。

~~原輪未證~~ → **第三輪(2026-09-03)進展,全部 kernel 驗證(SOTA 9 項
`Print Assumptions` 全為 "Closed under the global context",無 `classic`):**

1. ✅ **`cut_for_gt_start` 已封閉**(`WCRUtil.v` ③)。原計劃的「右折 minopt
   改寫 + 兩形等價」**實作後證明不需要**:對 fold_left 歸納前先把累加器
   **generalize 成抽象 `acc`**(`fold_min_mem`),cbn/simpl 就只對「列表是
   cons」開一刀,具體 `None` 不在場 ⇒ 不存在「過度約簡 ⇒ injection 失敗」。
   卡點的病根是「左折 + 具體初值 + cbn」的組合,不是 fold_left 本身。
   新引理:`fold_min_mem`(min 左折的結果 ∈ 輸入 ∪ {初值})+
   `cut_for_gt_start`(主定理,由 `ct_pred_start_lt` 收口)。
2. ✅ **不變量機器已建**(`ConcreteWCR.v` ④,新檔,非空殼):
   `wf_state` / `uniq_ids` / `ct_applicable_spec`(規則出身 = 某尾巴上的
   cut_for)/ `ct_rules_are_r1` / `r1_apply_keeps`(id 與 istart 向量不變)/
   `r1_apply_wf` / `uniq_map_eq` / `step_ct_spec` /
   `ct_step_start_invariant` / `ct_step_preserves_uniq` /
   `ct_step_preserves_wf` —— Iteration-4 清單第 2、3(前半)項**完成**。
   附 `WfWitness` 模塊 4 個 `vm_compute` 證人(含「Guarded≡Raw」的實例複驗)。
3. ✅/⬜ 見下方「第四輪」:`R3_ct_wcr_raw` 已落地(2026-09-03,同日);
   `R4_ct_confluent` 與 Guarded 版仍未開工。

---

## 第四輪(2026-09-03 同日):R3 Raw 版 WCR 落地

`rocq/theories/ConcreteWCR.v` 新增 ⑤~⑧ 段,全部 kernel 驗證,
`Print Assumptions R3_ct_wcr_raw` = **"Closed under the global context"**
(無 Axiom / Admitted / `classic`):

- ⑤(已有)計算性證人模塊保持全綠。
- ⑥ apply 層交換鏈:`r1_apply2`(兩步施作的純函數式)
  / `r1_apply2_comm`(異 id 兩步可換序,**本輪最硬的等式證明**:
  destruct-eqn 後 goal 會被 simpl 再生成新鮮條件子,收尾必須用頂層
  eqn(EA/EB)再 rewrite 一次)/ `apply_r1_comm_ev`(事件列表層交換)。
- ⑦ 規則存活鏈:`pred_trim_inv` / `ct_pred_self` / `cut_bounds`
  (andb 五合取一律用 `repeat (apply andb_prop in H; destruct H as [? H])`
  收斂,**Ltac peel 不安全**——匿名 destruct 會吃掉目標名)/ 
  **`map_starts_filter_r1`**(取代原計劃的 filtered-list 逐字等式——
  該敘述**數學上為假**:trim 只修剪 id 命中者的 iend,filter 留下的是
  修剪後 record;一切不變性改走 istart 值列層)/ `fold_min_via_map`
  (fold 融合,累加器抽象歸納)/ `cut_for_trim_other`(他人 cut 不變,
  即前輪被否證引理的正確版本)/ `ct_menu_id_inj` / `ct_menu_cons`
  (一步 definitional 展開,治 simpl 失靶)/ `ct_rule_survives`
  (施作後他人規則仍在菜單——依賴 `uniq_map_eq`,**uniq 前提的真正用處**)。
- ⑧ 主定理 **`R3_ct_wcr_raw`**:
  `forall s sa sb, uniq_ids s -> step_ct_raw s sa -> step_ct_raw s sb ->
   joinable step_ct_raw sa sb`。
  - 如實修正前輪口徑:**wf 前提本路線不需要**(cut_bounds 鏈自給自足);
    `uniq_ids` 不可省(重複 id 讓同名多 cut 規則破壞精確交換;
    枚舉宇宙由 D5 構造唯一,故前測全綠與此不矛盾)。
  - 證明結構:同 id ⇒ uniq 下 `ct_menu_id_inj` 得參數相等 ⇒ 兩邊同態;
    異 id ⇒ `apply_r1_comm_ev` 給共同中點,兩側各補一步
    (存在性用 `r1_apply_some` + `r1_apply_keeps` 的 id-map 不變性;
    合法性用 `ct_rule_survives`)。
- 仍未完成(不虛報):**Guarded 版** step_ct 的 WCR(需「修剪只刪紅邊」
  的遞減計量,或先證 Guarded≡Raw 於可達態)、**R4_ct_confluent**
  (newman + R2 + R3 的組裝,需先把 R3 接到 Guarded 或論證 Raw 足夠)、
  唯一正規形推論。ROADMAP Phase 3 行改標「Raw 完成 / Guarded·R4 進行中」。

**工程教訓(供後續輪次省時,已寫進 `WCRUtil.v` 頭註)**:
1. `nth_error` 按 nat 遞歸,遇到未約簡的 `trim_at q i c` 即卡死 ⇒ 改走
   成對歸納謂詞(`trim1`),不要在 `nth_error` 上疊 `cbn/simpl/native_compute`。
2. Coq 的 conversion **不對不透明記錄做 eta**:`change (trim_ev b c) with b`
   回報 `Not convertible` ⇒ 「只改 iend 的事件」必須真的寫成 constructor 形式,
   或改用逐欄 `cbn [ev_id ev_storage ev_kind ev_it istart iend]`。
3. `cbn in *` 會把假設約簡成 `True` 並被 `subst` 吞掉,錯誤訊息會指向**錯誤的一邊**
   (本輪據此誤判 `i_overlap` 有額外合取項,白跑十餘回合)⇒ 用 `Show` 印目標,
   別只讀錯誤文字。

## 二、鏡像決策(D1–D7,Phase 0-b)

> 完整鏡像:`rocq/theories/Mirror.v`(`K / Interval / Ev / AState / 紅邊 / µ /
> Rule / Policy / Menu / apply / applicable / 狀態枚舉`)。

| # | 決策 | 理由 | 對帳 |
|---|---|---|---|
| D1 | `u32 → nat`;半開區間 `Interval { istart; iend }` 原樣 | 座標無負;半開語義(`overlaps` = `a.start < b.end ∧ b.start < a.end`)不變 | `i_overlap_sym` 已證;kernel 以 `(0,4)/(1,3)/(2,5)` 樣本對帳 |
| D2 | `AState.log` 剔除 | Rust 註明「不參與相等性」,診斷用途 | —(唯一不鏡像欄位,已申明) |
| D3 | `µ = (|E_red|, 0)` | §6.3 `|Err_rustc|` 為 oracle 範疇,記 0(判定權不轉移);不可在證明中造模型 | `measure canon = (2, 0)` 一致 |
| D4 | 顯示標簽/`label`/`format` 不入鏡像 | 純診斷 | — |
| D5 | id = 位置索引;Rust `AState::new` 自補 | 鏡像中事件由 `build_evs` 依列表序賦 id | 樣本 id 0..3 兩邊一致 |
| D6 | 枚舉:鏡像用列表順序,Rust 用 `BTreeSet` 排序去重 | 只比計數,不比排序產生的具體順序 | `count_states n m = (m(m+1))^n` 對 Rust 計數全對(見 §三) |

**完整對帳矩陣**(`tools/rocq_reconcile.py` 生成 `reconcile_gen.v`,kernel 以
`vm_compute` 複驗):

| 關鍵 | 結果 | 關鍵 | 結果 |
|---|---|---|---|
| count(1,3) | 12 | appli(ct,g) | 1 |
| count(2,3) | 144 | appli(ct,r) | 1 |
| count(2,4) | 400 | appli(naive,r) | 22 |
| count(3,3) | 1728 | r1_red / r1_iv | 1 / (0,2) |
| count(3,4) | 8000 | r2_storage | [0,0,2,1] |
| red_edges | [(0,1);(0,2)] | r3_iv | (2,5,0,4) |
| measure | (2,0) | r4_red / r4_runtime | 1 / [(1,2);(0,1)] |
| nf | false | — | — |

> 對帳中修的一處真實分歧:Rust `R4Runtime` 以 `push` 尾插 `runtime`,
> 鏡像初版誤為頭插(`::`),kernel 以 `r4_runtime=[(1,2);(0,1)]` 抓出並修正。
> **這正是三層自証的價值:互相獨立實作,差異即暴露。**

---

## 三、建置/驗證命令

```sh
cd rocq && make            # Mirror.v + AbstractArs.v → kernel 檢查
make reconcile             # Rust 實例 ↔ Rocq 鏡像差分(需 cargo;生成 reconcile_gen.v 並複驗)
```

CI(`.github/workflows/ci.yml` → job `rocq`):apt 裝 coq + mathcomp →
`make -C rocq` → `make -C rocq reconcile`。

---

## 四、誠實申報

1. `Mirror.v` 曾含「`count_states_formula`(狀態數 = (m(m+1))^n)」定理草稿,
   未能在截止前完成證明 → **已移除**(非假裝);現狀以對帳矩陣的 5 個樣本點
   支撐該公式,數學證明留待 Phase 2(或由 `product`/`repeat` 結構歸納接出)。
2. R1 是「抽象」定理,其 `sn` 前提已由 R2 對接到鏡像的具體菜單
   (`R2_sn_step_ct`);`wcr` 前提仍未對接 —— 這是 Phase 3(R3)的任務。
   在此之前不宣稱「任意狀態的合流」(僅有 4×6 窮舉證人)。
2b. **Phase 3 的 R3 主定理:Raw 版已落地(2026-09-03 第四輪)**:
   `R3_ct_wcr_raw : forall s sa sb, uniq_ids s -> step_ct_raw s sa ->
   step_ct_raw s sb -> joinable step_ct_raw sa sb`,Print Assumptions 潔淨。
   **但**:這是 **Raw(未過濾)步關係** 的定理,且以 `uniq_ids` 為前提;
   **Guarded 版 WCR、R4_ct_confluent、唯一正規形推論仍未證**。
   在此之前不宣稱「Guarded 鏡像任意狀態的合流」。原第三輪寫下的
   「他人 cut 不變」否證依然有效——最終落地的是其修正版
   `cut_for_trim_other`(走 istart 值列層,不走 filtered-list 逐字等式;
   後者經證實**數學上為假**)。
3. 環境:沙箱快照不保留 `~/.rustup` 與 `/usr/local` 下的工具;
   `scripts/setup_dev.sh` 一鍵重建(apt 需 sudo;Rust 工具鏈重裝約 10 秒)。
4. `rocq-of-rust` 路線未採用(理由見 ROCQ-PLAN §4.2);若後續需要「實作層」
   驗證(如 `rep::apply` 無 panic),可作 Phase 6 選配。
5. **本輪如實修正兩處文件口徑**:(a)「46 具名測試」→ 實跑 `cargo test --all` = 50
   (15 單元 + 32 集成;本輪新增 L9b′ 後為 32);(b) `docs/BENCH.md`/`bench/BASELINE.json`
   所依託的 bench gate 曾在 CI 與本地**同時紅**(原因非代碼回歸:基線單機單次採樣
   + µs 級指標以 median 判定)。2026-09-02 已修:`tools/bench_gate.py` 改採
   **best-of-n** 判据 + `null` 環境標尺校正 + 同機 `--update` 基線 +
   `tools/bench_gate_selftest.py`(2026-09-03 起為 **13** 情境判別力自測,掛 CI)。
   本沙箱實跑:fmt/clippy/doc/reconcile 19 項/rocq/coverage/bench **全綠**。
   2026-09-03 健檢並修了 gate 判定式本身兩處(污染度算了沒用且方向反、
   null 底噪挪動紅線),見 `docs/BENCH.md`;另修 `tools/cov_gate.py` 的四條
   fail-open 防線(空報告 / 模組缺席 / 只剩被過濾的 bin / 同名 basename
   皆須判紅),並入庫 `tools/cov_gate_selftest.py`(9 情境,掛 CI)。
   **仍保留的限定**:CI runner 與基線不同機(4 核 vs 2 核),首次 CI 跑後需在
   該機 `--update` 一次才算「同境基線」;在此之前不宣稱「全管線綠(含 CI)」。
