(* ===================================================================== *)
(* R2 —— 具體 SN(Phase 2):CommutativeTrim × Guarded 的強正規化。          *
 *                                                                       *
 * 陳述(R2):菜單的每條規則的每個「合法施用」(通過側條件者)都使            *
 *          µ = (|E_red|, 0) 字典序嚴格遞減 ⇒ 單步關係強正規化(SN)。        *
 * 對應 Rust:`test_law_L8_red_edge_decreasing` 與                           *
 *          `test_law_L8_measure_is_guaranteeing_termination`               *
 *          (機器窮舉 623,616 狀態 × 635,424 臨界對全綠)。                 *
 * 本檔:「所有狀態」的證明(不限空間),把 L8 從「測試級」升為「定理級」。   *
 *                                                                       *
 * 註:Guarded 政策在鏡像中定義為「filter 保留 sd(µ) 為真者」——           *
 *     側條件不是裝飾:它正是定律的載體(§4.1 Policy::Raw 的對照)。        *
 *     因此 L8a 是定義的內涵;SN 則由 µ 的良基(自然數)直接導出。            *
 * ===================================================================== *)

From Coq Require Import List Arith Bool Lia.
From Coq Require Import Wellfounded.
Require Import Cl0r0.Mirror.
Require Import Cl0r0.AbstractArs.
Import ListNotations.

Set Implicit Arguments.
Unset Implicit Arguments.

(* --------------------------------------------------------------------- *)
(* L8a:Guarded 合法施用 ⇒ µ 嚴格遞減                                       *)
(* --------------------------------------------------------------------- *)

(* Guarded 的「合法性」= 側條件(µ 遞減)為真;所以本引理由 applicable 的
 * 定義直接內涵出:每一條出現在 applicable s CommutativeTrim Guarded 的
 * 規則,施用後 µ 嚴格遞減。 *)
Lemma guarded_apply_decreases :
  forall (s : AState) (r : Rule),
    In r (applicable s CommutativeTrim Guarded) ->
    exists s', apply_rule s r = Some s' /\ sd (measure s') (measure s) = true.
Proof.
  intros s r Hin.
  unfold applicable in Hin.
  rewrite filter_In in Hin.
  destruct Hin as [_ Hf].
  destruct (apply_rule s r) as [s2 |] eqn:E.
  - exists s2. split; [reflexivity | exact Hf].
  - discriminate.
Qed.

(* 特化:第二分量恆 0,故 sd 的字典序遞減 ⟺ 紅邊計數嚴格遞減。 *)
Lemma sd0_lt : forall a b, sd (a, 0) (b, 0) = true -> a < b.
Proof.
  intros a b H.
  (* sd (c,0) (d,0) = (c<d) ∨ (c=d ∧ 0<0);第二分量恆 0<0 → 假。
   * 故 H 的第一析取必真 → c<d。 *)
  unfold sd in H. simpl in H.
  destruct (proj1 (Bool.orb_true_iff _ _) H) as [Hlt | Hend].
  - apply Nat.ltb_lt. exact Hlt.
  - apply (proj1 (Bool.andb_true_iff _ _)) in Hend. destruct Hend as [_ Hz].
    apply Nat.ltb_lt in Hz. lia.
Qed.

(* --------------------------------------------------------------------- *)
(* 單步關係及其測度遞減                                                   *)
(* --------------------------------------------------------------------- *)

Definition step_ct (s s' : AState) : Prop :=
  exists r, In r (applicable s CommutativeTrim Guarded) /\ apply_rule s r = Some s'.

Lemma step_measure_lt : forall s s', step_ct s s' ->
  length (red_edges s') < length (red_edges s).
Proof.
  intros s s' [r [Hin Hap]].
  destruct (guarded_apply_decreases s r Hin) as [s2 [Hap2 Hsd]].
  assert (s2 = s') by congruence. subst s2.
  unfold measure in Hsd.
  apply (sd0_lt (length (red_edges s')) (length (red_edges s)) Hsd).
Qed.

(* --------------------------------------------------------------------- *)
(* RSS2(定理):CommutativeTrim × Guarded 的單步關係強正規化。               *)
(* --------------------------------------------------------------------- *)

(* 證明:µ 的第一分量 |E_red| ∈ nat,每步嚴格遞減 ⇒ 由 nat 的良基性,
 * 不存在無限下降鏈 —— 即反向關係 well-founded。 *)
Theorem R2_sn_step_ct : sn step_ct.
Proof.
  unfold sn.
  (* µ 第一分量 |E_red| 為測度:每步嚴格遞減 → nat 良基 → SN。 *)
  assert (Hdec : forall x y : AState,
                 (fun s s' => step_ct s' s) x y ->
                 length (red_edges x) < length (red_edges y)).
  { intros x y Hxy. apply (step_measure_lt y x Hxy). }
  exact (well_founded_lt_compat AState (fun s : AState => length (red_edges s))
                                (fun s s' => step_ct s' s) Hdec).
Qed.

(* 推論(R4′ × R2):SN 下每一狀態都有一條有限的合法規約路徑到達正規形
 * (唯一性需 Phase 3 的合流,見 ROCQ-TRACE)。 *)
Corollary ct_guarded_has_nf :
  forall s, exists n, star step_ct s n /\ nf step_ct n.
Proof.
  apply (exists_normal_form AState step_ct R2_sn_step_ct).
Qed.
