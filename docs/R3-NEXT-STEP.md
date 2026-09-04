# R3 下一輪 —— 從 Raw 版 WCR 升格為 Guarded 版 WCR → R4 合流

> 2026-09-03。承 `docs/ROCQ-TRACE.md` §「第四輪」、`docs/R3-RESEARCH.md` §四。
> 前情:`R3_ct_wcr_raw`(Raw 步關係的 WCR,`uniq_ids` 前提)已 kernel 驗證;
> **Guarded 版 WCR、`R4_ct_confluent`、唯一正規形仍未證**。
> 本檔 = 在**全新環境**重現全綠 + 把「Guarded 版」的橋樑以**實測**釘死 + 給出精確的證明義務。

---

## 〇、一鍵重現(全新沙箱實測,2026-09-03)

工具鏈(依 `docs/ROCQ-TRACE.md` §零)+ 三層自証在本機**逐項複驗全綠**:

| 項目 | 版本 | 結果 |
|---|---|---|
| coqc | 8.20.1(Debian trixie) | ✅ `make -C rocq` 5 個理論全建 |
| mathcomp-ssreflect | 2.3.0 | ✅ |
| rustc / cargo | 1.98.0 stable(rustup) | ✅ `cargo test --all` = 15 單元 + 36 集成 = **51/51** |
| 對帳 | `make -C rocq reconcile` | ✅ `rocq_reconcile.py` 19 項 Rust 事實,`reconcile_gen.v` kernel 複驗 OK |
| Maude 交叉驗證 | 3.4 | ✅ CT 五性質全綠(3×3/3×4/3×5)+ 倒掛宇宙 P2 + 計數 1728/8000/32768 |
| NaTT 交叉驗證 | (尚未建檔,見下) | ⬜ 需現編 NaTT 2.3 + z3 |

NaTT 目前**未建**進本環境(EXTERNAL-XCHECK 已先在專案的機上跑過 YES)。若要本機重現:
`sudo apt-get install ocaml-nox ocaml-findlib libocamlgraph-ocaml-dev libre-ocaml-dev libxml-light-ocaml-dev z3`,
再依 `docs/EXTERNAL-XCHECK.md` §三 的踩坑備忘編 NaTT(大小寫 bug `sed -i 's/myXML/MyXML/' Makefile`)。

**結論:三層自証(具名測試 → 窮舉機器檢查 → Rocq 定理)在本新環境都能重建,無環境漂移。**

---

## 一、目前缺口(單一數學跳板)

現有:
* **R1** `newman : sn -> wcr -> confluent`(AbstractArs.v)→ 抽象層。
* **R2** `R2_sn_step_ct : sn step_ct`(ConcreteSN.v)。**注意:這是 Guarded 版的 SN**,其 `sn` 由
  `guarded_apply_decreases`(filter 內涵)直接導出 —— Guarded 政策**定義地**只保留 µ 嚴格遞減者。
* **R3_raw** `R3_ct_wcr_raw : forall s sa sb, uniq_ids s -> step_ct_raw s sa -> step_ct_raw s sb ->
  joinable step_ct_raw sa sb`(ConcreteWCR.v ⑧)。這是 **Raw(未過濾)步關係** 的 WCR。

缺什麼才拿到 Guarded 版的 `R4_ct_confluent`?新曼引理需要 `wcr step_ct`(**Guarded 步關係**):

```
第一步:把 R3_raw 的結論從 step_ct_raw 抬到 step_ct。
   因為 R3_ct_wcr_raw 的 join 是中點 cc: sa --rb--> cc, sb --ra--> cc。
   這兩步目前是 step_ct_raw(靠 ct_rule_survives 拿到的 Raw 菜單成員)。
   要把它們變成 step_ct(Guarded),需「rb 在 sa 處 Guarded 適用」,即
       sd (measure cc) (measure sa) = true。
   而 sd (x,0) (y,0) = (x<y) ∨ (x=y ∧ 0<0) = x<y(第二分量恆 0)。
   ⇒ 化為 **「紅邊計數嚴格遞減」:length (red_edges sa') < length (red_edges sa)**。

所以唯一缺的數學跳板是:

   ★ ★  ★  對 CommutativeTrim 的每條規則 R1Shorten i c,
   施作後 紅邊計數**嚴格遞減**:length (red_edges (apply s (R1Shorten i c))) < length (red_edges s)。
   ★ ★ ★

(2026-09-03 進展:**單調**減 `<=` 已 kernel 驗證(WCRUtil.v §⑤ `r1_trim_red_le`),
即「剪一事件紅邊不增」這半(幾何內容的主體)已成;剩餘的是把它升為**嚴格**的
「子集 + 班候選邊被丟形」組裝,見 §四 的 ⬜ 清單。)

把它證出來 ⇒ `ct_guard_redundant : applicable s CT Guarded = applicable s CT Raw`
(在適當範圍內)⇒ `step_ct ≡ step_ct_raw` ⇒ `R3_ct_wcr` 與 `R4_ct_confluent` 接龍。
```

---

## 二、★ 橋樑的實測釘死(本輪新增,`examples/r3_red.rs`)

> 把「Guarded≡Raw」這個**兩分式**的金句量化成一句可執行的探針:
> 枚舉宇宙 **加上其 CT 可達閉包**(不只看初始宇宙),逐狀態掃描
> (a) 有無 Raw CT 規則施作後紅邊**不**嚴格遞減(即 Guarded 剔除它);
> (b) Guarded 菜單 ≠ Raw 菜單 的狀態數。

本機實跑

| 宇宙 | 枚舉狀態數 | 可達閉包(Guarded)狀態數 | 紅邊未嚴格遞減(規則施作)**次數** | Guarded≠Raw 狀態數 |
|---|---|---|---|---|
| 3×4 | 8,000 | 8,000 | **0** | 0 |
| 3×5 | 27,000 | 27,000 | **0** | 0 |
| 3×6 | 74,088 | 74,088 | **0** | 0 |
| 4×6 | 3,111,696 | (未跑閉包) | **0** | 0 |

複現:

```sh
cargo run --release --example r3_red 3 4 --reach
cargo run --release --example r3_red 3 5 --reach
cargo run --release --example r3_red 3 6 --reach
cargo run --release --example r3_red 4 6
```

**解讀**:① 可達閉包(Guarded)恆等於枚舉宇宙 —— 因為 CT 步只把某事件端點移進 `(start, m]`,
枚舉宇宙已含一切這種區間;② 全部 0 違反 ⇒ 在 CT 宇宙上**每一條 raw 規則都嚴格遞減紅邊**,
這是把 `R3_ct_wcr_raw` 升格為 `R3_ct_wcr`(Guarded)的實測支柱。

---

## 三、★ 必先承認的「範圍限制」(否則會做白工)

`ct_guard_redundant` 作為對**所有** `AState` 的全稱命題是**假**的。原因:

* Guarded 的側條件 `sd (measure s2) (measure s)` 看的是 `red_edges s`——它**排除已在 runtime 的邊**。
* 若事件 a 的**全部候選** b(與 a 同 storage、kind 衝突、`start_a < start_b`、重疊)早已在
  `st_runtime` 裡,則剪 a 到 cut 只**移除那些本就不算紅邊的邊**,紅邊計數**不變** ⇒ Guarded 把規則剔除。
* 但「曾在 runtime」只能來自 `R4Runtime`,而 R4Runtime **不在** CT 菜單 ⇒ 從 `runtime=[]` 出發、
  只走 CT 步,**runtime 恆空**(R1Shorten 不改 runtime)。⇒ arXiv 上那類反例在 CT 可達宇宙**不可能**出現
  (本探針的可達閉包即證據)。

**結論**:R4 合流定理應**表述在 CT 可達類**(或帶上「a 的候選邊皆不屬 runtime」的假設),而不是
對任意 `AState` 的空泛全稱。這是一個**只靠文件讀不到、要動手才踩到**的坑 —— 本輪已把它抓出來。

---

## 四、✅ 紅邊嚴格遞減的證明計畫(下一輪照此走)

在 CT 可達宇宙(`st_runtime = []`,故 `rt_mem … = false`,`red_edges_aux` 化為純儲存/種類/重疊)。

給定事件 `a`(id = i,區間 `[s_a, e_a)`),cut_for 給 `c = min { start_b : ct_pred a b }`。
**候選集合**:所有 b 滿足 `ev_id b ≠ i ∧ storage b = storage a ∧ k_conflict (kind a) (kind b)
∧ s_a < start_b ∧ i_overlap (it a) (it b)`。`c` 存在(cut_for 為 Some)⇒ 候選非空。取 `b0` 使 `start_b0 = c`。

**論證骨架**:
1. **`b0` 是 a 的紅邊**(在 s 中):同 storage、kind 衝突、重疊(`i_overlap` 真)、且不在 runtime
   (runtime 空)⇒ 由 `red_edges_aux` 的 filter 包含。
2. **剪後 a 與所有候選不再重疊**:剪後 a' = `[s_a, c)`。與 b 重疊需 `start_b < c`(且 `s_a < end_b`)。
   候選皆有 `start_b ≥ c`(c 為最小)⇒ 對所有候選 `start_b < c = iend a'` 不成立 ⇒ 不重疊。
   (此即 `R3-RESEARCH §二 觀測三`「紅邊單調不增」的準確形式。)
3. **只有 a 的相關邊變少**、其他事件間邊不变:剪只動 `iend a`,`istart a` 不变、他人区间不变
   ⇒ a 與「非候選者」的邊維持原狀(見 `ConcreteWCR.v` 的 `map_starts_filter_r1` / `cut_for_trim_other`
   的 istart-值列層不変,同樣精神)。⇒ `length (red_edges l') < length (red_edges l)`。

**地基已 kernel 驗證**(2026-09-03,`WCRUtil.v` ③b,`Print Assumptions` 全潔淨):
* `i_overlap_after_cut`   : `c ≤ istart b ⇒ i_overlap (剪後 a) b = false`(剪到 min 起點 ⇒ 重疊消失)。
* `i_overlap_cut_implies_overlap` : 剪後(落在 a 原區間)重合 ⇒ 剪前也重合(剪不新增交集)。
* `ct_cut_min_drops`      : 從 `cut_for l a = Some c` 抽 min 起點候選 b,
  `ct_pred a b = true ∧ istart b = c ∧ i_overlap (ev_it (trim_ev a c)) (ev_it b) = false`。

**尚缺的組裝**(在 `rocq/theories/R3Guard.v`,與 `WCRUtil`/`ConcreteWCR` 同層):
* ✅ `r1_trim_red_le`(WCRUtil.v §⑤,2026-09-03 kernel 驗證):`length (red_edges_aux (trim_at l i c) rt) <= length (red_edges_aux l rt)` —— 剪一事件 ⇒ 紅邊計數**不增**(非嚴格)。
* ✅ 配套(WCRUtil.v §⑤,kernel 驗證):`red_p`、`overlap_trim_left/right`、`red_p_trim_left/right`、`red_edges_aux_cons`、`filter_sub_len`、`filter_trim_at_le`。**前提**:被剪事件 `c <= iend`(真收縮;CT 菜單由 `cut_for_gt_start` + 候選重疊保證)。
* ✅ `r1_apply_eq_trim_at`(WCRUtil.v §⑤,kernel 驗證):`pos_of l i = Some p -> r1_apply l i c = Some (trim_at l p c)`(id 版規則 ↔ 位置版剪)。
* ✅ `mem_trim_at_pos`(WCRUtil.v §⑤,2026-09-03 kernel 驗證,位置版):`In b (trim_at t i c) -> (exists a, nth_error t i = Some a /\ b = trim_ev a c) \/ In b t`(比純 `In` 版精準,供 `nth_error` 配 `c <= iend`)。
* ✅ `red_edges_aux_trim_subset`(WCRUtil.v §⑤,2026-09-03 kernel 驗證):`(forall e, nth_error l i = Some e -> c <= iend (ev_it e)) -> forall p, In p (red_edges_aux (trim_at l i c) rt) -> In p (red_edges_aux l rt)` —— **剪後邊集 ⊆ 剪前邊集**(逐點用 `red_p_trim_left/right` + `mem_trim_at_pos`)。
* ✅ `red_p_rt_ct_pred`(WCRUtil.v §⑤,kernel 驗證):`ct_pred a b = true ∧ rt_mem rt (min id) (max id) = false → red_p rt a b = true`(候選 b 是 a 的**紅邊**種子)。
* ✅ 邊結構件(WCRUtil.v §⑤,kernel 驗證):`pk`(=(min id,max id))、`red_p_sym`、`min_plus_max`、`pk_minmax`、`nodup_map_ev_id_inj`(唯 id ⇒ 同 id 即同事件)、`NoDup_map_inj`(injective map 保 NoDup)、`in_red_edges_aux`(x,y ∈ l 互紅 ⇒ `pk x y` ∈ 紅邊清單,無關順序)。
* ✅ `in_red_char`(WCRUtil.v §⑤,2026-09-05 kernel 驗證,強化版):`NoDup (map ev_id l) → In p (red_edges_aux l rt) → ∃x y, In x l ∧ In y l ∧ x ≠ y ∧ p = pk x y ∧ red_p rt x y = true`(`in_red_edges_aux` 反方向;`id_ne_notin`/`pk_comp_has_a`/`pk_comp_in_ids` 作件)。
* ✅ `red_edges_aux_nodup`(WCRUtil.v §⑤,2026-09-05 kernel 驗證):`NoDup (map ev_id l) → NoDup (red_edges_aux l rt)`(map 邊含 `id_a`、t 內邊不含 ⇒ `NoDup_app`;`map_edge_not_in_tail` + `NoDup_map_inj`)。**關鍵結構性質 —— 紅邊清單在唯 id 前提下無重複。**
* ✅ 剪刀作件(WCRUtil.v §⑤,kernel 驗證):`red_p_not_overlap`(重疊消失 ⇒ 非紅邊)、`trim_at_preserves_ids`(`map ev_id (trim_at l p c) = map ev_id l`)、`trim_at_nth_here`(第 p 位剪後正是 `trim_ev _ c`)。
* ⬜ **剩餘**:把 §⑤ 的**單調**(`r1_trim_red_le`)升為**嚴格** `length … < length …`。骨架:`red_edges_aux` 無重複(✅) + 剪後邊集 ⊆ 剪前邊集(✅) + 一班「候選邊被丟」⇒ 嚴格。僅需:
  * `pk_comp_in_ab`:`pk a b = pk x y → 分量的 id 皆屬 {id_a, id_b}`(把「邊出現在剪後」拉回原事件的 id 對)。
  * 班候選 `b0` 丟性 `pk_abs_after_trim`:`c ≤ iend (ev_it a)` + `i_overlap (ev_it (trim_ev a c)) (ev_it b0) = false` ⇒ `~ In (pk a b0) (red_edges_aux (trim_at l p c) rt)`(剪後 `red_p` 對 `(trim_ev a c, b0)` 因重疊消失而 = false)。
  * `length_lt_nodup`:`NoDup l1 → NoDup l2 → incl l1 l2 → (∃x, In x l2 ∧ ~ In x l1) → length l1 < length l2`。
  * 組裝 `r1_apply_red_edges_strict` → `ct_guard_redundant`(於可達類,`st_runtime=[]`)→ `step_ct ≡ step_ct_raw`。

> **✅ 全部組裝完成(2026-09-05,`rocq/theories/R3Guard.v`,kernel 驗證)**。接橋鏈:
> `ct_applicable_decomp`(每條 CT 規則的前綴分解)`→ ct_rule_red_strict`(紅邊嚴格遞減)
> `→ guard_keeps_ct_rule`(Raw 規則通過側條件)`→ step_ct_iff_step_ct_raw`(單步等價)
> `→ star_raw_guarded / R3_ct_wcr`(Guarded 局部合流)`→ step_ct_reach / R4_ct_confluent`(可達類合流)。
> 全部 `Print Assumptions` 輸出「Closed under the global context」(無 Axiom/Admitted)。

> **誠實限定**:第四輪的 `R3_ct_wcr_raw` 證明**沒有**走到 `red_edges`(它是純機械式精確交換,不觸碰計量);
> 因此本跳板是**新**的硬活,不是把既有證明改名。若紅邊遞減卡住(>4 人日),按 `ROCQ-PLAN §三-R3`
> 的備援:`peak-decreasingness`(1 標籤)或有限空間反射證書;兩者皆有先例。

---

## 五、接龍(跳板已成立,定理已入 `R3Guard.v`)

```
R3_ct_wcr (Guarded) : forall s sa sb, uniq_ids s -> st_runtime s = [] ->
                       step_ct s sa -> step_ct s sb -> joinable step_ct sa sb.
   // 可達類(wf 非必要:R3_ct_wcr_raw 不用 wf;uniq + runtime=[] 已足)。
   // ✅ `Theorems R3_ct_wcr`(R3Guard.v),2026-09-05 kernel 驗證。

R4_ct_confluent : forall s, uniq_ids s -> st_runtime s = [] ->
                    (forall b c, star step_ct s b -> star step_ct s c ->
                                 joinable step_ct b c).
   // 在可達子關係 step_ct_reach 上用 newman;✅ `Theorem R4_ct_confluent`。

 已知局域(§三):拿掉 `st_runtime s = []` 則 `R3_ct_wcr` / `R4_ct_confluent`
 對任意 AState 是**假**的(runtime 已含候選邊時 Guarded 側條件不遞減)。
 故 R4 的「合流」以可達類為域 —— 這正是 §三 抓到的範圍限制。

唯一正規形 : forall s n1 n2, uniq_ids s -> st_runtime s = [] ->
              star step_ct s n1 -> star step_ct s n2 ->
              nf step_ct n1 -> nf step_ct n2 -> n1 = n2.
   // newman_unf R1 + R2_sn_step_ct + R3_ct_wcr(仍開放,下一輪接)。
```

---

## 六、本輪交付物

* `examples/r3_red.rs`:紅邊遞減 + Guarded≡Raw 的實測探針(枚舉 + 可達閉包)。
* `rocq/theories/WCRUtil.v` ③b:三個紅邊幾何引理(`i_overlap_after_cut` / `i_overlap_cut_implies_overlap` /
  `ct_cut_min_drops`),全部 kernel 驗證(無 Axiom/Admitted)。
* 環境全綠重現:coqc 8.20.1 / mathcomp 2.3.0 / rustc 1.98.0 / maude 3.4,
  `make -C rocq && make -C rocq reconcile` + `cargo test --all`(51/51)+ Maude 交叉驗證 全過。
* `rocq/theories/R3Guard.v`(**新**):`ct_applicable_decomp` + `ct_rule_red_strict`(紅邊嚴格遞減)
  + `guard_keeps_ct_rule` + `step_ct_iff_step_ct_raw` + `star_raw_guarded` + `R3_ct_wcr`(Guarded 局部合流)
  + `step_ct_reach` / `sn_step_ct_reach` / `wcr_step_ct_reach` + `R4_ct_confluent`(可達類合流),
  全部 kernel 驗證(無 Axiom/Admitted)。`make -C rocq`(6 理論)+ `make reconcile`(19 點)全綠。
* 範圍限制(§三)已定稿並體現在定理前置:`R3_ct_wcr` / `R4_ct_confluent` 以「可達類」
  (`uniq_ids ∧ st_runtime = []`)為域。剩餘開放:**唯一正規形**的接龍(`newman_unf`)。
