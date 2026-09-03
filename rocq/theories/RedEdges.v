(* --------------------------------------------------------------------- *)
(* 紅邊集合的結構引理                                                     *)
(*                                                                        *)
(* 用途:R3 的 **Guarded 版** WCR。Raw 版(R3_ct_wcr_raw)只需要 apply 層的  *)
(* 交換律與規則存活;Guarded 版還需要「兩條補步仍然嚴格遞減 µ = |E_red|」, *)
(* 那是一個**紅邊計數**論證,故需要本檔的基建:                            *)
(*                                                                        *)
(*   ① 縮短關係 shorten_rel  —— r1_apply 的結構刻畫                       *)
(*   ② 述詞單調 ct_pred_shorten_{l,r}  —— 右端點變小 ⇒ 重疊只減不增        *)
(*   ③ 單調性 red_edges_aux_shorten_incl  —— 縮短後的紅邊是原本的子集      *)
(*   ④ 移除局部性 —— 消失的紅邊必涉及被縮短的那個事件                     *)
(*                                                                        *)
(* 為什麼先寫成演算法再寫成證明:R3 前測(r3_probe / r1_redexcl)已在 Rust   *)
(* 側窮舉驗證過「移除集合互斥」,本檔把同一個論證提升到定理級。             *)
(* --------------------------------------------------------------------- *)

From Coq Require Import List Arith Bool Lia.
Require Import Cl0r0.Mirror.
Import ListNotations.

(* --------------------------------------------------------------------- *)
(* ⓪ red_edges_aux 的判定述詞(與 Mirror.v 裡的 let p 逐字對應)           *)
(* --------------------------------------------------------------------- *)

Definition ct_red_pred (a b : Ev) (rt : list (nat * nat)) : bool :=
  andb (Nat.eqb (ev_storage b) (ev_storage a))
    (andb (k_conflict (ev_kind a) (ev_kind b))
      (andb (i_overlap (ev_it a) (ev_it b))
        (negb (rt_mem rt (Nat.min (ev_id a) (ev_id b))
                          (Nat.max (ev_id a) (ev_id b)))))).

Lemma red_edges_aux_cons : forall (a : Ev) (l : list Ev) (rt : list (nat * nat)),
  red_edges_aux (a :: l) rt =
    map (fun b => (Nat.min (ev_id a) (ev_id b), Nat.max (ev_id a) (ev_id b)))
        (filter (fun b => ct_red_pred a b rt) l) ++ red_edges_aux l rt.
Proof. intros. reflexivity. Qed.

(* --------------------------------------------------------------------- *)
(* ① 縮短關係                                                             *)
(* --------------------------------------------------------------------- *)

(** `l'` 由 `l` 把 id = i 的事件的**右端點改小**(其餘欄位、其餘元素逐字不變)。 *)
Definition shorten_rel (l l' : list Ev) (i : nat) : Prop :=
  exists l1 e e' l2,
    l = l1 ++ e :: l2 /\ l' = l1 ++ e' :: l2 /\
    ev_id e = i /\ ev_id e' = i /\
    ev_storage e' = ev_storage e /\ ev_kind e' = ev_kind e /\
    istart (ev_it e') = istart (ev_it e) /\ iend (ev_it e') <= iend (ev_it e).

(** `r1_apply` 確實是一次縮短(前提:新端點不大於舊端點 —— 由 cut_bounds 供給)。 *)
Lemma r1_apply_shorten : forall (l : list Ev) i c l',
  r1_apply l i c = Some l' ->
  (forall e, In e l -> ev_id e = i -> c <= iend (ev_it e)) ->
  shorten_rel l l' i.
Proof.
  induction l as [|e t IH]; intros i c l' Hap Hcut; simpl in Hap.
  - discriminate.
  - destruct (Nat.eqb (ev_id e) i) eqn:Eid.
    + injection Hap as Hinj. subst l'.
      exists [], e,
        {| ev_id := ev_id e; ev_storage := ev_storage e; ev_kind := ev_kind e;
           ev_it := {| istart := istart (ev_it e); iend := c |} |}, t.
      simpl. repeat split; auto.
      * apply Nat.eqb_eq. exact Eid.
      * apply Nat.eqb_eq. exact Eid.
      * apply Hcut; [now left | apply Nat.eqb_eq; exact Eid].
    + destruct (r1_apply t i c) as [l2|] eqn:E2; [|discriminate].
      injection Hap as Hinj. subst l'.
      destruct (IH i c l2 E2) as
        [l1 [e0 [e0' [l3 [Hl [Hl' [Hid0 [Hid0' [Hst [Hk [Hs Hi]]]]]]]]]]].
      { intros e1 H1 Hid1. apply Hcut; [now right | exact Hid1]. }
      exists (e :: l1), e0, e0', l3.
      split; [simpl; rewrite Hl; reflexivity |].
      split; [simpl; rewrite Hl'; reflexivity |].
      repeat split; auto.
Qed.

(** 縮短不改 id 列(供後面把「涉及的邊」對齊)。 *)
Lemma shorten_rel_ids : forall l l' i,
  shorten_rel l l' i -> map ev_id l' = map ev_id l.
Proof.
  intros l l' i [l1 [e [e' [l2 [Hl [Hl' [Hid [Hid' _]]]]]]]].
  subst l l'. repeat rewrite map_app. simpl.
  assert (H : ev_id e' = ev_id e) by lia.
  rewrite H. reflexivity.
Qed.

(* --------------------------------------------------------------------- *)
(* ② 述詞單調:右端點變小 ⇒ 重疊只減不增                                   *)
(* --------------------------------------------------------------------- *)

Lemma i_overlap_shorten_r : forall (x e e' : Ev),
  istart (ev_it e') = istart (ev_it e) ->
  iend (ev_it e') <= iend (ev_it e) ->
  i_overlap (ev_it x) (ev_it e') = true ->
  i_overlap (ev_it x) (ev_it e) = true.
Proof.
  intros x e e' Hs Hi Ho.
  unfold i_overlap in *.
  apply andb_true_iff in Ho. destruct Ho as [H1 H2].
  apply andb_true_iff. split.
  - apply Nat.ltb_lt in H1. apply Nat.ltb_lt. lia.
  - apply Nat.ltb_lt in H2. apply Nat.ltb_lt. lia.
Qed.

Lemma i_overlap_shorten_l : forall (x e e' : Ev),
  istart (ev_it e') = istart (ev_it e) ->
  iend (ev_it e') <= iend (ev_it e) ->
  i_overlap (ev_it e') (ev_it x) = true ->
  i_overlap (ev_it e) (ev_it x) = true.
Proof.
  intros x e e' Hs Hi Ho.
  unfold i_overlap in *.
  apply andb_true_iff in Ho. destruct Ho as [H1 H2].
  apply andb_true_iff. split.
  - apply Nat.ltb_lt in H1. apply Nat.ltb_lt. lia.
  - apply Nat.ltb_lt in H2. apply Nat.ltb_lt. lia.
Qed.

(** 被縮短的事件出現在述詞的**右**側(它是別人的尾巴)。 *)
Lemma ct_pred_shorten_r : forall (x e e' : Ev) rt,
  ev_id e' = ev_id e ->
  ev_storage e' = ev_storage e ->
  ev_kind e' = ev_kind e ->
  istart (ev_it e') = istart (ev_it e) ->
  iend (ev_it e') <= iend (ev_it e) ->
  ct_red_pred x e' rt = true -> ct_red_pred x e rt = true.
Proof.
  intros x e e' rt Hid Hst Hk Hs Hi H.
  unfold ct_red_pred in *.
  apply andb_true_iff in H. destruct H as [Ha H].
  apply andb_true_iff in H. destruct H as [Hb H].
  apply andb_true_iff in H. destruct H as [Hc Hd].
  rewrite Hst in Ha. rewrite Hk in Hb. rewrite Hid in Hd.
  apply andb_true_iff. split.
  - exact Ha.
  - apply andb_true_iff. split.
    + exact Hb.
    + apply andb_true_iff. split.
      * exact (i_overlap_shorten_r x e e' Hs Hi Hc).
      * exact Hd.
Qed.

(** 被縮短的事件出現在述詞的**左**側(它是這一段的頭)。 *)
Lemma ct_pred_shorten_l : forall (x e e' : Ev) rt,
  ev_id e' = ev_id e ->
  ev_storage e' = ev_storage e ->
  ev_kind e' = ev_kind e ->
  istart (ev_it e') = istart (ev_it e) ->
  iend (ev_it e') <= iend (ev_it e) ->
  ct_red_pred e' x rt = true -> ct_red_pred e x rt = true.
Proof.
  intros x e e' rt Hid Hst Hk Hs Hi H.
  unfold ct_red_pred in *.
  apply andb_true_iff in H. destruct H as [Ha H].
  apply andb_true_iff in H. destruct H as [Hb H].
  apply andb_true_iff in H. destruct H as [Hc Hd].
  rewrite Hst in Ha. rewrite Hk in Hb. rewrite Hid in Hd.
  apply andb_true_iff. split.
  - exact Ha.
  - apply andb_true_iff. split.
    + exact Hb.
    + apply andb_true_iff. split.
      * exact (i_overlap_shorten_l x e e' Hs Hi Hc).
      * exact Hd.
Qed.

(* --------------------------------------------------------------------- *)
(* ③ 單調性:縮短之後的紅邊是原本的子集                                    *)
(* --------------------------------------------------------------------- *)

Lemma red_edges_aux_shorten_incl : forall (l l' : list Ev) (rt : list (nat * nat))
                                          (i : nat),
  shorten_rel l l' i -> incl (red_edges_aux l' rt) (red_edges_aux l rt).
Proof.
  intros l l' rt i Hsr.
  destruct Hsr as [l1 [e [e' [l2 [Hl [Hl' [Hid [Hid' [Hst [Hk [Hs Hi]]]]]]]]]]].
  subst l l'.
  assert (Hee : ev_id e' = ev_id e) by lia.
  revert l2. induction l1 as [|x t IH]; intros l2 p Hp.
  - (* 頭就是被縮短的事件 *)
    cbn [app] in Hp. cbn [app].
    rewrite red_edges_aux_cons in Hp. rewrite red_edges_aux_cons.
    apply in_app_iff in Hp. destruct Hp as [Hp | Hp].
    + apply in_app_iff. left.
      apply in_map_iff in Hp. destruct Hp as [b [Hpeq Hb]].
      apply in_map_iff. exists b. split.
      * rewrite Hee in Hpeq. exact Hpeq.
      * apply filter_In in Hb. destruct Hb as [Hbmem Hbp].
        apply filter_In. split; [exact Hbmem |].
        eapply ct_pred_shorten_l; eauto.
    + apply in_app_iff. now right.
  - (* 頭相同,縮短發生在尾巴 *)
    cbn [app] in Hp. cbn [app].
    rewrite red_edges_aux_cons in Hp. rewrite red_edges_aux_cons.
    apply in_app_iff in Hp. destruct Hp as [Hp | Hp].
    + apply in_app_iff. left.
      apply in_map_iff in Hp. destruct Hp as [b [Hpeq Hb]].
      apply filter_In in Hb. destruct Hb as [Hbmem Hbp].
      apply in_app_iff in Hbmem. destruct Hbmem as [Hbt | Hbrest].
      * (* b 在縮短點之前:同一個事件,述詞逐字相同 *)
        apply in_map_iff. exists b. split; [exact Hpeq |].
        apply filter_In. split.
        -- apply in_app_iff. now left.
        -- exact Hbp.
      * simpl in Hbrest. destruct Hbrest as [Hbee' | Hbl2].
        -- (* b 正是被縮短的那個事件:對應回原始列表中的 e
              —— 注意見證要換成 e,因為 l' 裡放的是 e'、l 裡放的是 e,
                 兩者不相等;配對相等由 ev_id e' = ev_id e 補上。 *)
           subst b. apply in_map_iff. exists e. split.
           ++ rewrite <- Hee. exact Hpeq.
           ++ apply filter_In. split.
              ** apply in_app_iff. right. now left.
              ** eapply ct_pred_shorten_r; eauto.
        -- (* b 在縮短點之後:同一個事件 *)
           apply in_map_iff. exists b. split; [exact Hpeq |].
           apply filter_In. split.
           ++ apply in_app_iff. right. right. exact Hbl2.
           ++ exact Hbp.
    + apply in_app_iff. right. apply (IH l2 p Hp).
Qed.

(** AState 層的單調性(R1 不改 runtime,故 rt 相同)。 *)
Lemma red_edges_shorten_state : forall (s s' : AState) i,
  shorten_rel (st_evs s) (st_evs s') i ->
  st_runtime s' = st_runtime s ->
  incl (red_edges s') (red_edges s).
Proof.
  intros s s' i Hsr Hrt. unfold red_edges. rewrite Hrt.
  apply red_edges_aux_shorten_incl with (i := i). exact Hsr.
Qed.

(* --------------------------------------------------------------------- *)
(* ④ 下一輪的兩塊拼圖(如實標注:本檔不含任何未完成的證明)                 *)
(* --------------------------------------------------------------------- *)

(** 下一輪要補的兩條(構成了 Guarded WCR 的最後一塊):
 *
 *  (a) 移除局部性 —— 不涉及 i 的邊在縮短前後**等價**(而非只是單向包含):
 *      `x <> i -> y <> i -> In (x,y) (red_edges_aux l rt) <->
 *                            In (x,y) (red_edges_aux l' rt)`
 *      證明要對 shorten_rel 的 l1 歸納,並用「b 不是 e/e' 時述詞逐字相同」。
 *
 *  (b) NoDup —— `uniq_ids s -> NoDup (red_edges s)`
 *      (uniq_ids 下,每個 id 對只可能由唯一一對位置產生,故無重複;
 *       有了它才能把「子集」換成「長度不等式」做計數論證。)
 *
 * 有了 (a)(b) 之後的組裝:
 *   - ra 移除的邊都涉及 ia,rb 移除的邊都涉及 ib(由 (a) 反證);
 *   - 交集只可能是 (ia, ib);而「ra 移除它」⇒ istart ia < istart ib,
 *     「rb 移除它」⇒ istart ib < istart ia ⇒ 矛盾 ⇒ 交集為空;
 *   - 兩條規則皆 Guarded ⇒ 各移除 ≥1 條 ⇒ |E_red| 嚴格下降
 *     ⇒ 兩條補步仍是 Guarded 步 ⇒ wcr step_ct。
 *)
