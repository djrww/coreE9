(* ===================================================================== *)
(* R3 —— 具體 WCR(Phase 3):CommutativeTrim × Guarded 的局部合流。          *
 *                                                                       *
 * 本檔現況(2026-09-03,第三輪開工,誠實申報):                              *
 *   ✅ cut_for_gt_start(在 WCRUtil.v ③)— 上一轮的唯一缺口,本轮封闭。    *
 *   ✅ 不變量機器:wf_state / uniq_ids 定義 + CT 步保持性(三條引理) ——    *
 *      即 R3-RESEARCH §四 Iteration-4 的第 2 項(ct_step_start_invariant)  *
 *      與第 3 項的前半(不變量載體)。Guarded 側因前測證「guard 在 CT 上    *
 *      冗餘」,故步層的 wf/id 保持性對 Raw 與 Guarded 同真。               *
 *   ⬜ ct_join_exact / R3_ct_wcr / R4_ct_confluent —— 仍未開工,不建空殼。  *
 *                                                                       *
 * 證明骨架(對應 R3-RESEARCH §四-2/3):                                    *
 *   1. ct_applicable 的每條規則都是 R1Shorten (ev_id a) c,且 c 來自        *
 *      某個尾巴列表上的 cut_for(ct_applicable_spec —— 注意:cut_for        *
 *      的「全表」是遞歸當下的**尾巴**,不是原始全表;前置知識:              *
 *      ct_applicable 遞歸時只把「剩下的尾巴」當候選池)。                   *
 *   2. R1Shorten 只改 iend ⇒ id 向量與 istart 向量逐字不變                 *
 *      (r1_apply_keeps ⇒ ct_step_start_invariant / uniq 保持)。           *
 *   3. 良構性保持:被剪事件的 cut 由 cut_for_gt_start 擔保 > 其 istart;     *
 *      在 id 唯一後提下,r1_apply 命中的就是那個 a ⇒ 新事件仍 wf。          *
 * ===================================================================== *)

From Coq Require Import List Arith Bool Lia.
Require Import Cl0r0.Mirror.
Require Import Cl0r0.AbstractArs.
Require Import Cl0r0.ConcreteSN.
Require Import Cl0r0.WCRUtil.
Import ListNotations.

Set Implicit Arguments.
Unset Implicit Arguments.

(* --------------------------------------------------------------------- *)
(* ① 不變量定義:良構性(wf)與 id 唯一                                     *)
(* --------------------------------------------------------------------- *)

(** 每個事件都是「正向」半開區間:istart < iend。
    對應 Rust 的生成紀律(ends_for / enumerate_states);R3-RESEARCH §一
    的實測證明:這是**必須顯式假設、不能靠生成器蒙混**的不變量。 *)
Definition wf_evs (l : list Ev) : Prop :=
  forall e, In e l -> (istart (ev_it e) <? iend (ev_it e)) = true.

Definition wf_state (s : AState) : Prop := wf_evs (st_evs s).

(** 鏡像決策 D5(id = 位置索引)的謂詞化;step 不改 id ⇒ 此性質免費保持。 *)
Definition uniq_ids (s : AState) : Prop := NoDup (map ev_id (st_evs s)).

(* --------------------------------------------------------------------- *)
(* ② ct_applicable 的結構刻畫:規則 = 某尾巴上的 R1Shorten                *)
(* --------------------------------------------------------------------- *)

(** 每條 CT 規則都有「出身證明」:它是某事件 a 在其尾巴列表上的
    cut_for 產物。交換引理要做中間態計算時,兩條規則各自的出身
    就由本引理提供。 *)
Lemma ct_applicable_spec : forall l r,
  In r (ct_applicable l) ->
  exists pre a c post,
    l = pre ++ a :: post /\
    cut_for (a :: post) a = Some c /\
    r = R1Shorten (ev_id a) c.
Proof.
  induction l as [|x t IH]; intros r Hin; simpl in Hin.
  - contradiction.
  - destruct (cut_for (x :: t) x) as [c|] eqn:E; simpl in Hin.
    + destruct Hin as [Heq|Hin].   (* simpl 已把 [r0] ++ rest 收成 r0 :: rest、
                                       再把 In r (r0 :: _) 收成 etc = r \/ In … *)
      * subst r.
        exists [], x, c, t. split; [reflexivity|]. split; [exact E|reflexivity].
      * destruct (IH _ Hin) as [pre [a [d [post [Heq [Hcut Hr]]]]]]. subst r.
        exists (x :: pre), a, d, post.
        split; [simpl; rewrite Heq; reflexivity|].
        split; [exact Hcut|reflexivity].
    + destruct (IH _ Hin) as [pre [a [d [post [Heq [Hcut Hr]]]]]]. subst r.
      exists (x :: pre), a, d, post.
      split; [simpl; rewrite Heq; reflexivity|].
      split; [exact Hcut|reflexivity].
Qed.

(** 推論:CT 菜單只有 R1Shorten(沒有 R2/R3/R4 混進來)。 *)
Corollary ct_rules_are_r1 : forall l r,
  In r (ct_applicable l) -> exists i c, r = R1Shorten i c.
Proof.
  intros l r Hin.
  destruct (ct_applicable_spec _ _ Hin) as [pre [a [c [post [_ [_ Heq]]]]]].
  exists (ev_id a), c. exact Heq.
Qed.

(* --------------------------------------------------------------------- *)
(* ③ R1Shorten 的保持性(列表層)                                          *)
(* --------------------------------------------------------------------- *)

(** r1_apply 只重寫命中事件的 iend ⇒ id 向量與 istart 向量逐字不變。
    (這是「start 不變量」的最小年鑑:不需要 nth_error,直接歸納。) *)
Lemma r1_apply_keeps : forall l i c l',
  r1_apply l i c = Some l' ->
  map ev_id l' = map ev_id l /\
  map (fun e => istart (ev_it e)) l' = map (fun e => istart (ev_it e)) l.
Proof.
  induction l as [|e t IH]; intros i c l' Hap; simpl in Hap.
  - discriminate.
  - destruct (Nat.eqb (ev_id e) i) eqn:Eid.
    + injection Hap as Hinj. subst l'. simpl. auto.
    + destruct (r1_apply t i c) as [l2|] eqn:E2; [|discriminate].
      injection Hap as Hinj. subst l'. simpl.
      destruct (IH i c l2 E2) as [H1 H2].
      rewrite H1, H2. auto.
Qed.

(** 良構性保持:命中事件(第一個 id 匹配者)的 cut 若嚴格大於其 istart,
    修剪後全列表仍 wf。 *)
Lemma r1_apply_wf : forall l i c l',
  r1_apply l i c = Some l' ->
  wf_evs l ->
  (forall e, In e l -> ev_id e = i -> (istart (ev_it e) <? c) = true) ->
  wf_evs l'.
Proof.
  induction l as [|e t IH]; intros i c l' Hap Hwf Hcut; simpl in Hap.
  - discriminate.
  - destruct (Nat.eqb (ev_id e) i) eqn:Eid.
    + injection Hap as Hinj. subst l'.
      intros e0 Hin. destruct Hin as [Heq|Hin].
      * subst e0. simpl. apply Hcut; [now left|].
        apply Nat.eqb_eq. exact Eid.
      * apply Hwf. now right.
    + destruct (r1_apply t i c) as [l2|] eqn:E2; [|discriminate].
      injection Hap as Hinj. subst l'.
      intros e0 Hin. destruct Hin as [Heq|Hin].
      * subst e0. apply Hwf. now left.
      * apply (IH i c l2 E2).
        -- intros e1 H1. apply Hwf. now right.
        -- intros e1 H1 He1. apply Hcut; [now right|exact He1].
        -- exact Hin.
Qed.

(** id 唯一 + 同 id 必同事件(內涵形式,供後面拚接)。 *)
Lemma uniq_map_eq : forall (l : list Ev) x y,
  NoDup (map ev_id l) -> In x l -> In y l -> ev_id x = ev_id y -> x = y.
Proof.
  induction l as [|z t IH]; intros x y Hnd Hx Hy Hid; simpl in *.
  - contradiction.
  - apply NoDup_cons_iff in Hnd. destruct Hnd as [Hz Hnd].
    destruct Hx as [Hx|Hx]; [destruct Hy as [Hy|Hy]|destruct Hy as [Hy|Hy]].
    + subst x y. reflexivity.
    + subst x. exfalso. apply Hz. rewrite Hid. apply in_map with (f := ev_id) in Hy. exact Hy.
    + subst y. exfalso. apply Hz. rewrite <- Hid. apply in_map with (f := ev_id) in Hx. exact Hx.
    + eapply IH; eauto.
Qed.

(* --------------------------------------------------------------------- *)
(* ④ 步層:三條不變量引理(Iteration-4 清單第 2 項 + 第 3 項前半)          *)
(* --------------------------------------------------------------------- *)

(** 從 Guarded 的 In 回到 Raw 的 In + 規則出身,一次拼好的輔助戰術工具。 *)
Lemma step_ct_spec : forall s s',
  step_ct s s' ->
  exists pre a c post evs2,
    st_evs s = pre ++ a :: post /\
    cut_for (a :: post) a = Some c /\
    r1_apply (st_evs s) (ev_id a) c = Some evs2 /\
    s' = {| st_evs := evs2; st_runtime := st_runtime s |}.
Proof.
  intros s s' [r [Hin Hap]].
  unfold applicable in Hin. simpl in Hin.
  apply filter_In in Hin. destruct Hin as [Hin _].
  destruct (ct_applicable_spec _ _ Hin) as [pre [a [c [post [Heq [Hcut Hr]]]]]].
  subst r. simpl in Hap.
  destruct (r1_apply (st_evs s) (ev_id a) c) as [evs2|] eqn:E1; [|discriminate].
  injection Hap as Hinj. subst s'.
  exists pre, a, c, post, evs2. auto.
Qed.

(** 不變量 1:start 向量不變(「cut 只寫 iend」的步層版本)。 *)
Lemma ct_step_start_invariant : forall s s',
  step_ct s s' ->
  map (fun e => istart (ev_it e)) (st_evs s')
  = map (fun e => istart (ev_it e)) (st_evs s).
Proof.
  intros s s' Hstep.
  destruct (step_ct_spec _ _ Hstep) as [pre [a [c [post [evs2 [Heq [Hcut [E1 Hs']]]]]]]].
  subst s'. simpl.
  destruct (r1_apply_keeps _ _ _ _ E1) as [_ Hstart]. exact Hstart.
Qed.

(** 不變量 2:id 唯一性保持。 *)
Lemma ct_step_preserves_uniq : forall s s',
  step_ct s s' -> uniq_ids s -> uniq_ids s'.
Proof.
  intros s s' Hstep Hun.
  destruct (step_ct_spec _ _ Hstep) as [pre [a [c [post [evs2 [Heq [Hcut [E1 Hs']]]]]]]].
  subst s'. unfold uniq_ids in *. simpl.
  destruct (r1_apply_keeps _ _ _ _ E1) as [Hid _].
  rewrite Hid. exact Hun.
Qed.

(** 不變量 3:良構性保持 —— 上一次迭代的唯一缺口(cut_for_gt_start)
    在此接入:r1_apply 命中的事件在 id 唯一前提**就是** a,
    其 cut 已由 WCRUtil ③ 擔保 > istart。 *)
Lemma ct_step_preserves_wf : forall s s',
  step_ct s s' -> wf_state s -> uniq_ids s -> wf_state s'.
Proof.
  intros s s' Hstep Hwf Hun.
  destruct (step_ct_spec _ _ Hstep) as [pre [a [c [post [evs2 [Heq [Hcut [E1 Hs']]]]]]]].
  subst s'. unfold wf_state in *. simpl in *.
  apply (r1_apply_wf _ _ _ _ E1 Hwf).
  intros e Hein Heid.
  assert (Hina : In a (st_evs s)).
  { rewrite Heq. apply in_or_app. right. now left. }
  assert (ea : e = a).
  { symmetry. apply (uniq_map_eq _ a e Hun Hina Hein). symmetry. exact Heid. }
  subst e.
  apply (cut_for_gt_start _ _ _ Hcut).
Qed.

(* --------------------------------------------------------------------- *)
(* ⑥ apply 層菱形:兩條不同 id 的 R1Shorten 精確交換(不需任何不變量)。      *)
(* --------------------------------------------------------------------- *)

Section ApplyDiamond.

(** 兩步連用的記法(中間一步失敗則整體 None)。 *)
Definition r1_apply2 (l : list Ev) (ia ca ib cb : nat) : option (list Ev) :=
  match r1_apply l ia ca with
  | Some l' => r1_apply l' ib cb
  | None => None
  end.

(** 主引理:不同 id 的修剪可精確交換(等式級,非 merely joinable)。
    對應 Rust 探針 examples/r3_swap.rs 的「精確交換」全綠觀測,
    亦對應 Maude 交叉驗證的 P2 —— 此處是 ∀ 列表的證明。 *)
Lemma r1_apply2_comm : forall l ia ca ib cb,
  ia <> ib -> r1_apply2 l ia ca ib cb = r1_apply2 l ib cb ia ca.
Proof.
  induction l as [|e t IH]; intros ia ca ib cb Hne; unfold r1_apply2; simpl.
  - reflexivity.
  - destruct (Nat.eqb (ev_id e) ia) eqn:EA; destruct (Nat.eqb (ev_id e) ib) eqn:EB.
    + exfalso. apply Hne. apply Nat.eqb_eq in EA. apply Nat.eqb_eq in EB. lia.
    + (* ia 命中頭、ib 不命中頭 *)
      simpl. rewrite EB.
      destruct (r1_apply t ib cb) as [t2|] eqn:E2; simpl; [|reflexivity].
      rewrite EA. reflexivity.
    + (* ib 命中頭、ia 不命中頭(對稱) *)
      simpl. rewrite EA.
      destruct (r1_apply t ia ca) as [t1|] eqn:E1; simpl; [|reflexivity].
      rewrite EB. reflexivity.
    + (* 兩者皆不命中頭 ⇒ 歸尾巴 *)
      specialize (IH ia ca ib cb Hne). unfold r1_apply2 in IH.
      destruct (r1_apply t ia ca) as [t1|] eqn:E1;
      destruct (r1_apply t ib cb) as [t2|] eqn:E2;
      simpl in IH; simpl.
      * rewrite IH, EA, EB. reflexivity.
      * rewrite IH, EB. reflexivity.
      * rewrite <- IH. rewrite EA. reflexivity.
      * reflexivity.
Qed.

End ApplyDiamond.

(** 兩端點皆成功時,交換的兩側落在同一個列表上。 *)
Lemma apply_r1_comm_ev : forall s ia ca ib cb ea eb ec1 ec2,
  ia <> ib ->
  r1_apply (st_evs s) ia ca = Some ea ->
  r1_apply (st_evs s) ib cb = Some eb ->
  r1_apply ea ib cb = Some ec1 ->
  r1_apply eb ia ca = Some ec2 ->
  ec1 = ec2.
Proof.
  intros s ia ca ib cb ea eb ec1 ec2 Hne H1 H2 H3 H4.
  generalize (r1_apply2_comm (st_evs s) ia ca ib cb Hne).
  unfold r1_apply2. rewrite H1, H2, H3, H4.
  intros H. congruence.
Qed.

(* --------------------------------------------------------------------- *)
(* ⑦ 翻轉分析 + 菜單存活性(R3 的組合核心)                                *)
(* --------------------------------------------------------------------- *)

(** 修剪「他人」不改變觀察者的 ct_pred 判定:
    5 個合取項中僅 `istart o <? iend y` 可能變化;
    P = istart o <? istart y 真 ⇒ istart o < istart y < c ≤ iend y 鎖死;
    P 假 ⇒ 雙邊由 && false 各自塌縮。 *)
Lemma pred_trim_inv : forall o y c,
  istart (ev_it y) < c -> c <= iend (ev_it y) ->
  ct_pred o y = ct_pred o (trim_ev y c).
Proof.
  intros o y c Hlt Hle.
  unfold ct_pred, trim_ev, i_overlap. simpl.
  destruct (Nat.ltb (istart (ev_it o)) (istart (ev_it y))) eqn:HP.
  - apply Nat.ltb_lt in HP.
    assert (H1 : (istart (ev_it o) <? iend (ev_it y)) = true)
      by (apply Nat.ltb_lt; lia).
    assert (H2 : (istart (ev_it o) <? c) = true)
      by (apply Nat.ltb_lt; lia).
    rewrite H1, H2. reflexivity.
  - rewrite !andb_false_l, !andb_false_r. reflexivity.
Qed.

(** 自我不是自己的候選(同一 id 直接出局)。 *)
Lemma ct_pred_self : forall x, ct_pred x x = false.
Proof.
  intro x. unfold ct_pred. rewrite Nat.eqb_refl. reflexivity.
Qed.

(** cut 的雙側界:cut_for 成立 ⇒ istart a < c < iend a。
    左側 = cut_for_gt_start;右側 = fold_min_mem 取見證 b + ct_pred 的
    overlap 合取項(istart b <? iend a)。 *)
Lemma cut_bounds : forall l a c,
  cut_for l a = Some c ->
  istart (ev_it a) < c /\ c < iend (ev_it a).
Proof.
  intros l a c Hcut. split.
  - apply Nat.ltb_lt. exact (cut_for_gt_start _ _ _ Hcut).
  - rewrite cut_for_is_filter in Hcut.
    destruct (fold_min_mem (fun b => istart (ev_it b)) _ _ _ Hcut)
      as [[b [Hin Hc]] | Hcontra]; [|discriminate].
    apply filter_In in Hin. destruct Hin as [_ Hpred].
    unfold ct_pred in Hpred.
    repeat (apply andb_prop in Hpred; destruct Hpred as [? Hpred]).
    (* apply 有 conversion:i_overlap 也會被展開拆穿 ⇒ 最後剩
       (istart (ev_it b) <? iend (ev_it a)) = true *)
    apply Nat.ltb_lt in Hpred. lia.
Qed.

(** filter 層不變性【istart 值列層;逐字相等是假的——留下的元素在
    修剪後是 trim_ev 形):修剪不改「候選者是否入選」也不改「入選者的 start」。
    前提是命中者的 cut 落在 (istart, iend](由 cut_bounds 供給)。 *)
Lemma map_starts_filter_r1 : forall l i c l',
  r1_apply l i c = Some l' ->
  (forall e, In e l -> ev_id e = i -> istart (ev_it e) < c /\ c <= iend (ev_it e)) ->
  forall o, map (fun e0 => istart (ev_it e0)) (filter (ct_pred o) l')
          = map (fun e0 => istart (ev_it e0)) (filter (ct_pred o) l).
Proof.
  induction l as [|e t IH]; intros i c l' Hap Hb; simpl in Hap.
  - discriminate.
  - destruct (Nat.eqb (ev_id e) i) eqn:Eid.
    + apply Nat.eqb_eq in Eid.
      injection Hap as Hinj. subst l'. intros o.
      destruct (Hb e (or_introl eq_refl) Eid) as [H1 H2].
      (* 修剪所成的事件換名成 trim_ev 形,以便套 pred_trim_inv *)
      change {| ev_id := ev_id e; ev_storage := ev_storage e;
                ev_kind := ev_kind e;
                ev_it := {| istart := istart (ev_it e); iend := c |} |}
        with (trim_ev e c).
      cbn [filter map].
      rewrite <- (pred_trim_inv o e c H1 H2).
      (* 兩側判定同值;true 分支的 cons 頭 istart 逐字相同(conversion) *)
      destruct (ct_pred o e); cbn [map]; reflexivity.
    + destruct (r1_apply t i c) as [t2|] eqn:E2; [|discriminate].
      injection Hap as Hinj. subst l'. intros o.
      cbn [filter]. destruct (ct_pred o e); cbn [map].
      * apply f_equal2; [reflexivity|].
        apply (IH i c t2 E2). intros e1 H1 He1. apply Hb; [now right|exact He1].
      * apply (IH i c t2 E2). intros e1 H1 He1. apply Hb; [now right|exact He1].
Qed.

(** fold 融合:cut_for 的「一邊走候選一邊取 istart-min」可以先 map 再 fold。
    (本輪工程經驗結晶:歸納時累加器必須全程抽象,
     None 具體初值+cbn 的組合就是 9-02 那輪的死結。) *)
Lemma fold_min_via_map : forall (l : list Ev) (acc : option nat),
  fold_left (fun acc0 b => match acc0 with
                           | None => Some (istart (ev_it b))
                           | Some c0 => Some (Nat.min c0 (istart (ev_it b)))
                           end) l acc
  = fold_left (fun acc0 n => match acc0 with
                             | None => Some n
                             | Some c0 => Some (Nat.min c0 n)
                             end) (map (fun e0 => istart (ev_it e0)) l) acc.
Proof.
  induction l as [|e t IH]; intro acc; simpl.
  - reflexivity.
  - rewrite (IH (match acc with
                 | None => Some (istart (ev_it e))
                 | Some c0 => Some (Nat.min c0 (istart (ev_it e)))
                 end)).
    destruct acc; reflexivity.
Qed.

(** 菜單規則把 bounds 提升為「所有同 id 事件」的形式(id 唯一 ⇒ 命中者唯一)。 *)
Lemma id_bounds_of_menu : forall l i c,
  NoDup (map ev_id l) ->
  In (R1Shorten i c) (ct_applicable l) ->
  forall e, In e l -> ev_id e = i -> istart (ev_it e) < c /\ c <= iend (ev_it e).
Proof.
  intros l i c Hun Hin e He Hei.
  destruct (ct_applicable_spec _ _ Hin) as [pre [a [c0 [post [Hl [Hcut Hr]]]]]].
  injection Hr as Hi Hcc. subst c0.
  assert (Hea : e = a).
  { apply (uniq_map_eq _ e a Hun He).
    - rewrite Hl. apply in_or_app. right. now left.
    - rewrite Hei. exact Hi. }
  subst e. destruct (cut_bounds _ _ _ Hcut) as [H1 H2].
  split; [exact H1 | lia].
Qed.

(** filter 的白名單預約:fold 之前先吃掉自指頭部(ct_pred x x ≡ false)。
    注意 cbn [filter] 白名單——若用 simpl 會把 ct_pred x x 也攤開,
    後面的 rewrite 就找不到項。 *)
Lemma filter_ct_pred_cons_self : forall (x : Ev) l,
  filter (ct_pred x) (x :: l) = filter (ct_pred x) l.
Proof.
  intros x l. cbn [filter]. rewrite ct_pred_self. reflexivity.
Qed.

(** 菜單 cut 對「他人修剪」不變:觀察者 x 的候選 istart 值列不變 ⇒ cut 不變。 *)
Lemma cut_for_trim_other : forall l i c l' (x : Ev),
  r1_apply l i c = Some l' ->
  (forall e, In e l -> ev_id e = i -> istart (ev_it e) < c /\ c <= iend (ev_it e)) ->
  cut_for l' x = cut_for l x.
Proof.
  intros l i c l' x Hap Hb.
  rewrite !cut_for_is_filter, !fold_min_via_map.
  rewrite (map_starts_filter_r1 _ _ _ _ Hap Hb x). reflexivity.
Qed.

(** 每個 id 在 CT 菜單至多一條規則(id 唯一 ⇒ 同 id 同 cut)。 *)
Lemma ct_menu_id_inj : forall l i c1 c2,
  NoDup (map ev_id l) ->
  In (R1Shorten i c1) (ct_applicable l) ->
  In (R1Shorten i c2) (ct_applicable l) -> c1 = c2.
Proof.
  induction l as [|x t IH]; intros i c1 c2 Hun H1 H2; simpl in H1, H2.
  - contradiction.
  - simpl in Hun. apply NoDup_cons_iff in Hun. destruct Hun as [Hx Hun].
    destruct (cut_for (x :: t) x) as [cx|] eqn:Hxcut; simpl in H1, H2.
    + destruct H1 as [H1|H1]; destruct H2 as [H2|H2].
      * congruence.
      * injection H1 as Hj _. subst i.
        destruct (ct_applicable_spec _ _ H2) as [pre [a [c3 [post [Hl [_ Hr]]]]]].
        injection Hr as Hai _. exfalso. apply Hx.
        rewrite Hl, map_app. simpl. rewrite <- Hai.
        apply in_or_app. right. now left.
      * injection H2 as Hj _. subst i.
        destruct (ct_applicable_spec _ _ H1) as [pre [a [c3 [post [Hl [_ Hr]]]]]].
        injection Hr as Hai _. exfalso. apply Hx.
        rewrite Hl, map_app. simpl. rewrite <- Hai.
        apply in_or_app. right. now left.
      * apply (IH i c1 c2 Hun H1 H2).
    + apply (IH i c1 c2 Hun H1 H2).
Qed.

(** ct_applicable 在 cons 上的一步展開(釘死形狀,拒絕 simpl 輪盤賭)。 *)
Lemma ct_menu_cons : forall (x : Ev) t,
  ct_applicable (x :: t) =
  (match cut_for (x :: t) x with
   | Some c0 => [R1Shorten (ev_id x) c0]
   | None => []
   end) ++ ct_applicable t.
Proof. intros. reflexivity. Qed.

(** 過頭直通:來自尾巴的規則在任何 cons 之下仍是菜單成員。 *)
Lemma ct_menu_cons_tail : forall (x' : Ev) t r,
  In r (ct_applicable t) -> In r (ct_applicable (x' :: t)).
Proof.
  intros x' t r Hin. simpl. destruct (cut_for (x' :: t) x'); simpl;
  [right|]; exact Hin.
Qed.

(** ★ 規則存活引理(R3 的核心拼圖):a 的規則施用後,b 的規則帶**同一 cut**
    仍在菜單裡。對照 Rust 前測「精確交換 2,443,506/2,443,506」:
    此處是 ∀ 狀態(id 唯一)版的證明。 *)
Lemma ct_rule_survives : forall l i c l',
  NoDup (map ev_id l) ->
  In (R1Shorten i c) (ct_applicable l) ->
  r1_apply l i c = Some l' ->
  forall j cb, In (R1Shorten j cb) (ct_applicable l) -> i <> j ->
  In (R1Shorten j cb) (ct_applicable l').
Proof.
  induction l as [|x t IH]; intros i c l' Hun Him Hap j cb Hjm Hne.
  - simpl in Hjm. contradiction.
  - simpl in Hun. apply NoDup_cons_iff in Hun. destruct Hun as [Hx Hun].
    simpl in Hap.
    destruct (Nat.eqb (ev_id x) i) eqn:Exi.
    + (* 頭部被剪:j-rule 必來自尾巴(id 唯一) *)
      apply Nat.eqb_eq in Exi.
      injection Hap as Hap. subst l'.
      simpl in Hjm. destruct (cut_for (x :: t) x) as [cx|] eqn:Hcx; simpl in Hjm.
      * destruct Hjm as [Hjm|Hjm].
        -- injection Hjm as Hj _. exfalso. apply Hne. congruence.
        -- apply (ct_menu_cons_tail _ t _ Hjm).
      * apply (ct_menu_cons_tail _ t _ Hjm).
    + (* 尾巴被剪:l' = x :: t2 *)
      apply Nat.eqb_neq in Exi.
      destruct (r1_apply t i c) as [t2|] eqn:E2; [|discriminate].
      simpl in Hap. injection Hap as Hap. subst l'.
      assert (Himt : In (R1Shorten i c) (ct_applicable t)).
      { simpl in Him. destruct (cut_for (x :: t) x) as [cx0|] eqn:Hcx0; simpl in Him.
        - destruct Him as [Him|Him]; [|exact Him].
          injection Him as Hi0 _. congruence.
        - exact Him. }
      assert (Hbd : forall e, In e t -> ev_id e = i ->
                     istart (ev_it e) < c /\ c <= iend (ev_it e)).
      { intros e He Hei. apply (id_bounds_of_menu t i c Hun Himt e He Hei). }
      assert (Hf : cut_for (x :: t2) x = cut_for (x :: t) x).
      { rewrite !cut_for_is_filter, !filter_ct_pred_cons_self, !fold_min_via_map.
        rewrite (map_starts_filter_r1 _ _ _ _ E2 Hbd). reflexivity. }
      simpl in Hjm. destruct (cut_for (x :: t) x) as [cx|] eqn:Hcx; simpl in Hjm.
      * destruct Hjm as [Hjm|Hjm].
        -- injection Hjm as Hj Hcb. subst j cb.
           (* 注意:Hcx 的 destruct 已把上下文中的 cut_for (x::t) x 全部代入
              ⇒ Hf 此時已正是目標所需的等式 *)
           rewrite ct_menu_cons. rewrite Hf. simpl. left. reflexivity.
        -- apply ct_menu_cons_tail. exact (IH i c t2 Hun Himt E2 j cb Hjm Hne).
      * apply ct_menu_cons_tail. exact (IH i c t2 Hun Himt E2 j cb Hjm Hne).
Qed.

(* --------------------------------------------------------------------- *)
(* ⑧ Raw 版 R3:局部合流定理                                              *)
(* --------------------------------------------------------------------- *)

(** r1_apply 的進行性:有目標 id 就能施。 *)
Lemma r1_apply_some : forall l i c,
  (exists e, In e l /\ ev_id e = i) ->
  exists l', r1_apply l i c = Some l'.
Proof.
  induction l as [|x t IH]; intros i c [e [He Hid]]; simpl in *.
  - contradiction.
  - destruct (Nat.eqb (ev_id x) i) eqn:E.
    + eexists. reflexivity.
    + apply Nat.eqb_neq in E.
      destruct He as [He|He].
      * subst x. congruence.
      * destruct (IH i c) as [t' Ht']; [exists e; auto|].
        rewrite Ht'. eexists. reflexivity.
Qed.

(** Raw 單步(無 Guarded 過濾;與 step_ct 同型以便平行對帳)。 *)
Definition step_ct_raw (s s' : AState) : Prop :=
  exists r, In r (applicable s CommutativeTrim Raw) /\ apply_rule s r = Some s'.

(** Raw 版 R3:局部合流(一步等式級連接——比「joinable 需搜索深度」強,
    即前測觀測的「精確交換」由測試級晉升定理級)。
    如實修正前輪口徑:**wf 前提本路線不需要**(bounds 鏈條自給自足);
    uniq_ids 不可省(重複 id 會讓同名多 cut 規則破壞精確交換——
    枚舉宇宙由 D5 構造唯一,故前測全綠與此不矛盾)。 *)
Theorem R3_ct_wcr_raw : forall s sa sb,
  uniq_ids s ->
  step_ct_raw s sa -> step_ct_raw s sb ->
  joinable step_ct_raw sa sb.
Proof.
  intros s sa sb Hun [ra [Hina Hapa]] [rb [Hinb Hapb]].
  unfold applicable in Hina, Hinb. simpl in Hina, Hinb.
  destruct (ct_rules_are_r1 _ _ Hina) as [ia [ca Hra]]; subst ra.
  destruct (ct_rules_are_r1 _ _ Hinb) as [ib [cb Hrb]]; subst rb.
  simpl in Hapa, Hapb.
  destruct (r1_apply (st_evs s) ia ca) as [ea|] eqn:EA; [|discriminate].
  destruct (r1_apply (st_evs s) ib cb) as [eb|] eqn:EB; [|discriminate].
  injection Hapa as Hapa. injection Hapb as Hapb. subst sa sb.
  destruct (Nat.eq_dec ia ib) as [Heq|Hne].
  - (* 同 id ⇒ 菜單同 id 唯一 ⇒ 同一步 ⇒ sa = sb *)
    subst ib.
    assert (ca = cb) as Hcc by exact (ct_menu_id_inj _ _ _ _ Hun Hina Hinb).
    subst cb.
    assert (ea = eb) by congruence. subst eb.
    exists {| st_evs := ea; st_runtime := st_runtime s |}.
    split; apply star_refl.
  - (* 異 id:⑥ apply 層交換 + ⑦ 規則存活,各補一條合法步 *)
    assert (Hex1 : exists ec1, r1_apply ea ib cb = Some ec1).
    { apply r1_apply_some.
      destruct (ct_applicable_spec _ _ Hinb) as [pb [ab [cb0 [qb [Hlb [_ Hr1]]]]]].
      injection Hr1 as Hbi Hc1. subst cb0.
      destruct (r1_apply_keeps _ _ _ _ EA) as [Hids _].
      assert (Hinid : In ib (map ev_id (st_evs s))).
      { rewrite Hbi. apply in_map. rewrite Hlb. apply in_app_iff. right. now left. }
      rewrite <- Hids in Hinid. apply in_map_iff in Hinid.
      destruct Hinid as [e' [Hid' Hin']]. exists e'. split; auto. }
    assert (Hex2 : exists ec2, r1_apply eb ia ca = Some ec2).
    { apply r1_apply_some.
      destruct (ct_applicable_spec _ _ Hina) as [pa [aa [ca0 [qa [Hla [_ Hr2]]]]]].
      injection Hr2 as Hai Hc2. subst ca0.
      destruct (r1_apply_keeps _ _ _ _ EB) as [Hids _].
      assert (Hinid : In ia (map ev_id (st_evs s))).
      { rewrite Hai. apply in_map. rewrite Hla. apply in_app_iff. right. now left. }
      rewrite <- Hids in Hinid. apply in_map_iff in Hinid.
      destruct Hinid as [e' [Hid' Hin']]. exists e'. split; auto. }
    destruct Hex1 as [ec1 Hec1]. destruct Hex2 as [ec2 Hec2].
    assert (ec1 = ec2) as Hec by
      exact (apply_r1_comm_ev s ia ca ib cb ea eb ec1 ec2 Hne EA EB Hec1 Hec2).
    subst ec2.
    (* 中點狀態:兩側 runtime 都逐字是 s 的(R1 不碰 runtime) *)
    set (cc := {| st_evs := ec1; st_runtime := st_runtime s |}).
    exists cc. split.
    + (* sa --rb--> cc *)
      eapply star_step; [|apply star_refl].
      exists (R1Shorten ib cb). split.
      * unfold applicable. simpl.
        apply (ct_rule_survives _ ia ca ea Hun Hina EA ib cb Hinb Hne).
      * simpl. rewrite Hec1. unfold cc. reflexivity.
    + (* sb --ra--> cc *)
      eapply star_step; [|apply star_refl].
      exists (R1Shorten ia ca). split.
      * unfold applicable. simpl.
        apply (ct_rule_survives _ ib cb eb Hun Hinb EB ia ca Hina (not_eq_sym Hne)).
      * simpl. rewrite Hec2. unfold cc. reflexivity.
Qed.

(* --------------------------------------------------------------------- *)
(* ⑤ 計算性證人(vm_compute):不變量引理不是空話                            *)
(* --------------------------------------------------------------------- *)

(** 兩事件宇宙的實例:觀察者 evA(id 1, Sh, [0,3))看見 evB(id 0, Mut,
    [2,10));CT 菜單只產生「剪 evA 到 2」(min 候選 start = istart evB = 2,
    由 cut_for 產生,故 cut_for_gt_start 保證 2 > istart evA = 0 ⇒ wf 保持)。 *)
Module WfWitness.

Definition s0 : AState :=
  {| st_evs := [ WCRUtil.evA; WCRUtil.evB ]; st_runtime := [] |}.

(** 前提成立的計算證據:s0 是 wf 的、id 唯一的。 *)
Goal wf_state s0.
Proof.
  intros e Hin.
  destruct Hin as [Heq|[Heq|[]]]; subst e; vm_compute; reflexivity.
Qed.

Goal uniq_ids s0.
Proof.
  unfold uniq_ids. simpl.
  apply NoDup_cons.
  - intros [Hb|[]]. discriminate.
  - apply NoDup_cons; [intros []| constructor].
Qed.

(** 菜單輸出(只有一條:R1Shorten 1 2;Raw 與 Guarded 一致 —— 前測
    「Guarded≡Raw」在此見證上的計算複驗)。 *)
Goal applicable s0 CommutativeTrim Raw = [R1Shorten 1 2].
Proof. vm_compute; reflexivity. Qed.

Goal applicable s0 CommutativeTrim Guarded = [R1Shorten 1 2].
Proof. vm_compute; reflexivity. Qed.

(** 套用後 evA := [0,2)(新狀態仍 wf:0 < 2;紅邊 1 → 0,µ 嚴格遞減)。 *)
Goal apply_rule s0 (R1Shorten 1 2) =
     Some {| st_evs := [ WCRUtil.trim_ev WCRUtil.evA 2; WCRUtil.evB ];
             st_runtime := [] |}.
Proof. vm_compute; reflexivity. Qed.

Goal wf_state {| st_evs := [ WCRUtil.trim_ev WCRUtil.evA 2; WCRUtil.evB ];
                 st_runtime := [] |}.
Proof.
  intros e Hin.
  destruct Hin as [Heq|[Heq|[]]]; subst e; vm_compute; reflexivity.
Qed.

End WfWitness.
