(* ===================================================================== *)
(* Phase 3 —— Guarded ↔ Raw 等價 + R3(Guarded WCR)+ R4(CT 合流)。         *)
(* ===================================================================== *)

From Coq Require Import List Arith Bool Lia.
From Coq Require Import Wellfounded.
Require Import Cl0r0.Mirror.
Require Import Cl0r0.WCRUtil.
Require Import Cl0r0.ConcreteWCR.
Require Import Cl0r0.ConcreteSN.
Require Import Cl0r0.AbstractArs.
Import ListNotations.

Local Open Scope bool_scope.
Set Implicit Arguments.
Unset Implicit Arguments.

(* --------------------------------------------------------------------- *)
(* A. ct_applicable 的「前綴分解」。                                        *)
(* --------------------------------------------------------------------- *)

Lemma ct_applicable_decomp : forall l r,
  In r (ct_applicable l) ->
  exists pre a post c,
    l = pre ++ a :: post /\
    r = R1Shorten (ev_id a) c /\
    cut_for (a :: post) a = Some c.
Proof.
  induction l as [|a0 l0 IH]; intros r Hin.
  - simpl in Hin. contradiction.
  - simpl in Hin. apply in_app_iff in Hin.
    destruct (cut_for (a0 :: l0) a0) as [c0|] eqn:Hc.
    + destruct Hin as [Hin|Hin].
      * simpl in Hin. destruct Hin as [Hr|Hr]; [|contradiction].
        subst r. exists [], a0, l0, c0. split; [reflexivity|]. split; [reflexivity|].
        exact Hc.
      * destruct (IH r Hin) as [pre' [a [post [c [Hl [Hr Hcut]]]]]].
        exists (a0 :: pre'), a, post, c. split; [simpl; rewrite Hl; reflexivity|].
        split; [exact Hr | exact Hcut].
    + destruct Hin as [Hin|Hin].
      * simpl in Hin. destruct Hin.
      * destruct (IH r Hin) as [pre' [a [post [c [Hl [Hr Hcut]]]]]].
        exists (a0 :: pre'), a, post, c. split; [simpl; rewrite Hl; reflexivity|].
        split; [exact Hr | exact Hcut].
Qed.

(* --------------------------------------------------------------------- *)
(* B. 前綴的 id 與被剪者不同。                                            *)
(* --------------------------------------------------------------------- *)

Lemma map_ev_id_pre_ne : forall pre a post,
  NoDup (map ev_id (pre ++ a :: post)) ->
  forall e, In e pre -> ev_id e <> ev_id a.
Proof.
  induction pre as [|e' pre' IH]; intros a post Hnd e He.
  - contradiction.
  - simpl in Hnd. apply NoDup_cons_iff in Hnd. destruct Hnd as [Hne' Hnd'].
    simpl in He. destruct He as [He|He].
    + subst e. intro H. apply Hne'. rewrite H.
      rewrite map_app. apply in_app_iff. right. now left.
    + apply (IH a post Hnd' e He).
Qed.

(* --------------------------------------------------------------------- *)
(* C. 核心接橋:任一 CT(Raw)規則在 uniq + runtime=[] 下嚴格遞減紅邊。        *)
(* --------------------------------------------------------------------- *)

Lemma ct_rule_red_strict : forall s r s',
  uniq_ids s -> st_runtime s = [] ->
  In r (applicable s CommutativeTrim Raw) ->
  apply_rule s r = Some s' ->
  length (red_edges s') < length (red_edges s).
Proof.
  intros s r s' Hun Hrt Hin Hap.
  unfold applicable in Hin. simpl in Hin.
  destruct (ct_applicable_decomp (st_evs s) r Hin)
    as [pre [a [post [c [Hl [Hr Hcut]]]]]].
  subst r.
  (* cut 落在 a 區間內: c <= iend a *)
  assert (Hc : c <= iend (ev_it a)).
  { destruct (cut_bounds (a :: post) a c Hcut) as [_ Hc_lt]. lia. }
  (* min 起點候選 b0,其 start = c *)
  assert (Hfold :
    fold_left (fun acc0 b => match acc0 with
                             | None => Some (istart (ev_it b))
                             | Some c0 => Some (Nat.min c0 (istart (ev_it b)))
                             end)
              (filter (ct_pred a) (a :: post)) None = Some c).
  { rewrite <- cut_for_is_filter. exact Hcut. }
  destruct (fold_min_mem (fun b => istart (ev_it b)) _ _ _ Hfold)
    as [[b0 [Hb0in Hb0c]] | Hnone]; [| discriminate].
  apply filter_In in Hb0in. destruct Hb0in as [Hbin Hct0].
  assert (Hb0inl : In b0 (st_evs s)).
  { rewrite Hl. apply in_app_iff. right. exact Hbin. }
  assert (Hov : i_overlap (ev_it (trim_ev a c)) (ev_it b0) = false).
  { apply (i_overlap_after_cut a b0 c). rewrite Hb0c. apply Nat.le_refl. }
  assert (Hrtm : rt_mem (st_runtime s)
                 (Nat.min (ev_id a) (ev_id b0))
                 (Nat.max (ev_id a) (ev_id b0)) = false).
  { rewrite Hrt. reflexivity. }
  assert (Hstrict :
    length (red_edges_aux (trim_at (st_evs s) (length pre) c) (st_runtime s)) <
    length (red_edges_aux (st_evs s) (st_runtime s))).
  { apply (r1_apply_red_edges_strict (st_evs s) (length pre) c (st_runtime s) a b0).
    - exact Hun.
    - rewrite Hl. apply nth_error_mid.
    - exact Hc.
    - exact Hct0.
    - exact Hb0inl.
    - exact Hrtm.
    - exact Hov. }
  (* 把 r1_apply 與位置版剪連接 *)
  assert (Hpos : pos_of (st_evs s) (ev_id a) = Some (length pre)).
  { rewrite Hl. apply (pos_of_mid pre post a).
    apply (map_ev_id_pre_ne pre a post). unfold uniq_ids in Hun. rewrite Hl in Hun. exact Hun. }
  assert (Htr : r1_apply (st_evs s) (ev_id a) c = Some (trim_at (st_evs s) (length pre) c)).
  { apply (r1_apply_eq_trim_at (st_evs s) (ev_id a) c (length pre) Hpos). }
  (* apply_rule 結構:砍到 l',再把 l' 接到 trim_at *)
  unfold apply_rule in Hap. simpl in Hap.
  destruct (r1_apply (st_evs s) (ev_id a) c) as [l'|] eqn:Hr1; [|discriminate].
  injection Hap as Hrec.
  assert (Hl' : l' = trim_at (st_evs s) (length pre) c).
  { congruence. }
  unfold red_edges in *. rewrite <- Hrec. rewrite Hl'. exact Hstrict.
Qed.

Print Assumptions ct_rule_red_strict.

(* --------------------------------------------------------------------- *)
(* D. Raw 規則通過 Guard 側條件 ⇒ 細分成立。                               *)
(* --------------------------------------------------------------------- *)

Lemma guard_keeps_ct_rule : forall s r s',
  uniq_ids s -> st_runtime s = [] ->
  In r (applicable s CommutativeTrim Raw) ->
  apply_rule s r = Some s' ->
  sd (measure s') (measure s) = true.
Proof.
  intros s r s' Hun Hrt Hin Hap.
  assert (Hlt : length (red_edges s') < length (red_edges s)).
  { apply (ct_rule_red_strict s r s' Hun Hrt Hin Hap). }
  unfold measure, sd. simpl.
  apply Bool.orb_true_iff. left. apply Nat.ltb_lt. exact Hlt.
Qed.

Print Assumptions guard_keeps_ct_rule.

(* --------------------------------------------------------------------- *)
(* E. Guarded 與 Raw 的單步關係等價(uniq + runtime=[])。                  *)
(* --------------------------------------------------------------------- *)

Lemma step_ct_iff_step_ct_raw : forall s s',
  uniq_ids s -> st_runtime s = [] ->
  step_ct s s' <-> step_ct_raw s s'.
Proof.
  intros s s' Hun Hrt. split.
  - intros [r [Hin Hap]].
    exists r. split; [|exact Hap].
    (* Guarded 名單 ⊆ Raw 名單 *)
    unfold applicable in Hin. simpl in Hin.
    rewrite filter_In in Hin. destruct Hin as [Hin _].
    unfold applicable. simpl. exact Hin.
  - intros [r [Hin Hap]].
    exists r. split; [|exact Hap].
    (* Raw 名單通過 guard *)
    unfold applicable. simpl.
    rewrite filter_In. split; [exact Hin|].
    rewrite Hap. apply (guard_keeps_ct_rule s r s' Hun Hrt Hin Hap).
Qed.

Print Assumptions step_ct_iff_step_ct_raw.

(* --------------------------------------------------------------------- *)
(* F. 把 Raw 的 star 鏈升格為 Guarded(沿途狀態均 uniq 且 runtime 空)。   *)
(* --------------------------------------------------------------------- *)

Lemma step_ct_preserves_empty : forall s s',
  step_ct s s' -> st_runtime s = [] -> st_runtime s' = [].
Proof.
  intros s s' Hstep Hend.
  destruct (step_ct_spec _ _ Hstep) as [pre [a [c [post [evs2 [_ [_ [_ Hs']]]]]]]].
  subst s'. exact Hend.
Qed.

Lemma step_raw_preserves_empty : forall s s',
  step_ct_raw s s' -> st_runtime s = [] -> st_runtime s' = [].
Proof.
  intros s s' [r [Hin Hap]] Hend.
  unfold applicable in Hin. simpl in Hin.
  destruct (ct_applicable_decomp (st_evs s) r Hin)
    as [pre [a [post [c [Hl [Hr Hcut]]]]]].
  subst r. unfold apply_rule in Hap. simpl in Hap.
  destruct (r1_apply (st_evs s) (ev_id a) c) as [l2|] eqn:E1; [|discriminate].
  injection Hap as Hrec. rewrite <- Hrec. exact Hend.
Qed.

Lemma step_raw_preserves_uniq : forall s s',
  step_ct_raw s s' -> uniq_ids s -> st_runtime s = [] -> uniq_ids s'.
Proof.
  intros s s' Hraw Hun Hend.
  apply (ct_step_preserves_uniq s s').
  - apply (proj2 (step_ct_iff_step_ct_raw s s' Hun Hend) Hraw).
  - exact Hun.
Qed.

Lemma star_raw_guarded : forall s t,
  uniq_ids s -> st_runtime s = [] ->
  star step_ct_raw s t -> star step_ct s t.
Proof.
  intros s t Hun Hrt Hstar.
  induction Hstar as [a | a b c Hstep Htail IH].
  - apply star_refl.
  - apply star_step with b.
    + apply (proj2 (step_ct_iff_step_ct_raw a b Hun Hrt) Hstep).
    + apply IH.
      * apply (step_raw_preserves_uniq a b Hstep Hun Hrt).
      * apply (step_raw_preserves_empty a b Hstep Hrt).
Qed.

Print Assumptions star_raw_guarded.

(* --------------------------------------------------------------------- *)
(* G. Guarded 版局部合流(可達類:uniq + runtime=[])。                        *)
(* --------------------------------------------------------------------- *)

Theorem R3_ct_wcr : forall s sa sb,
  uniq_ids s -> st_runtime s = [] ->
  step_ct s sa -> step_ct s sb ->
  joinable step_ct sa sb.
Proof.
  intros s sa sb Hun Hrt Hsa Hsb.
  assert (Hunas : uniq_ids sa) by (apply (ct_step_preserves_uniq s sa Hsa Hun)).
  assert (Hrts : st_runtime sa = []) by (apply (step_ct_preserves_empty s sa Hsa Hrt)).
  assert (Hunbs : uniq_ids sb) by (apply (ct_step_preserves_uniq s sb Hsb Hun)).
  assert (Hrtb : st_runtime sb = []) by (apply (step_ct_preserves_empty s sb Hsb Hrt)).
  destruct (R3_ct_wcr_raw s sa sb Hun
             (proj1 (step_ct_iff_step_ct_raw s sa Hun Hrt) Hsa)
             (proj1 (step_ct_iff_step_ct_raw s sb Hun Hrt) Hsb))
    as [cc [Hcc_a Hcc_b]].
  exists cc. split.
  - apply (star_raw_guarded sa cc Hunas Hrts Hcc_a).
  - apply (star_raw_guarded sb cc Hunbs Hrtb Hcc_b).
Qed.

Print Assumptions R3_ct_wcr.

(* --------------------------------------------------------------------- *)
(* H. R4:把合流限制在可達類(uniq + runtime=[]),由 newman 接龍。           *)
(* --------------------------------------------------------------------- *)

Definition step_ct_reach (s t : AState) : Prop :=
  uniq_ids s /\ st_runtime s = [] /\ step_ct s t.

Lemma star_reach_sublift : forall s t,
  star step_ct_reach s t -> star step_ct s t.
Proof.

  induction 1 as [a | a b c Hstep Htail IH].
  - apply star_refl.
  - apply star_step with b; [| exact IH].
    destruct Hstep as [_ [_ Hstep']]. exact Hstep'.
Qed.

Lemma star_reach_lift : forall s t,
  uniq_ids s -> st_runtime s = [] ->
  star step_ct s t -> star step_ct_reach s t.
Proof.
  intros s t Hun Hrt Hstar.
  induction Hstar as [a | a b c Hstep Htail IH].
  - apply star_refl.
  - apply star_step with b.
    + split; [exact Hun|]. split; [exact Hrt| exact Hstep].
    + apply IH.
      * apply (ct_step_preserves_uniq a b Hstep Hun).
      * apply (step_ct_preserves_empty a b Hstep Hrt).
Qed.

Lemma sn_step_ct_reach : sn step_ct_reach.
Proof.
  unfold sn. assert (Hsn := R2_sn_step_ct). unfold sn in Hsn.
  apply (wf_incl AState (fun x y => step_ct_reach y x) (fun x y => step_ct y x)).
  - intros x y Hxy. destruct Hxy as [_ [_ Hxy]]. exact Hxy.
  - exact Hsn.
Qed.

Lemma wcr_step_ct_reach : wcr step_ct_reach.
Proof.
  intros a b c Hab Hac.
  destruct Hab as [Huna [Hrta Hab'']].
  destruct Hac as [_ [_ Hac'']].
  destruct (R3_ct_wcr a b c Huna Hrta Hab'' Hac'') as [d [Hdb Hdc]].
  exists d. split.
  - apply (star_reach_lift b d).
    + apply (ct_step_preserves_uniq a b Hab'' Huna).
    + apply (step_ct_preserves_empty a b Hab'' Hrta).
    + exact Hdb.
  - apply (star_reach_lift c d).
    + apply (ct_step_preserves_uniq a c Hac'' Huna).
    + apply (step_ct_preserves_empty a c Hac'' Hrta).
    + exact Hdc.
Qed.

Print Assumptions wcr_step_ct_reach.

Theorem R4_ct_confluent : forall s,
  uniq_ids s -> st_runtime s = [] ->
  (forall b c, star step_ct s b -> star step_ct s c ->
               joinable step_ct b c).
Proof.
  intros s Hun Hrt b c Hsb Hsc.
  assert (Hconv := newman AState step_ct_reach sn_step_ct_reach wcr_step_ct_reach).
  destruct (Hconv s b c
             (star_reach_lift s b Hun Hrt Hsb)
             (star_reach_lift s c Hun Hrt Hsc)) as [d [Hdb Hdc]].
  exists d. split.
  - apply (star_reach_sublift b d Hdb).
  - apply (star_reach_sublift c d Hdc).
Qed.

Print Assumptions R4_ct_confluent.
