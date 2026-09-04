(* ===================================================================== *)
(* R1 / R4 —— 抽象 Newman 引理(定理層,與專案類型無關)。                   *
 *                                                                       *
 * 陳述(R1):對任意抽象重寫系統 (A, →):強正規化(SN)∧ 局部合流(WCR)        *
 *          ⇒ 合流(CR);且每個元素的正規形唯一(R4,newman_unf)。           *
 * 證明:Huet 式 —— 於 SN 的良基關係上做歸納:先證「一步 vs 多步」的       *
 *       strip 性質(Q a),再證「多步 vs 多步」的可連接性(P a)。構造式。    *
 * 對應:Rust 側 L9a/L9b/L9c 為機器窮舉;本檔為「所有情形」的證明。         *
 * ===================================================================== *)

From Coq Require Import List Arith Lia.
From Coq Require Import Classical.
Import ListNotations.

Set Implicit Arguments.
(* 關閉自動隱式化:定理陳述全顯式,呼叫語法可預測(涉及量化型別時,
 * 隱式化會讓 A 與 r 都被吞掉,造成「nat 被丟進 sn 槽」的錯位)。 *)
Unset Implicit Arguments.

(* --------------------------------------------------------------------- *)
(* 定義:反射傳遞閉包 / 可連接 / WCR / 合流 / 正規形                       *)
(* --------------------------------------------------------------------- *)

Inductive star {A : Type} (r : A -> A -> Prop) : A -> A -> Prop :=
| star_refl : forall a, star r a a
| star_step : forall a b c, r a b -> star r b c -> star r a c.

Definition joinable {A : Type} (r : A -> A -> Prop) (b c : A) : Prop :=
  exists d, star r b d /\ star r c d.

(* 局部合流:單步分叉可連接。 *)
Definition wcr {A : Type} (r : A -> A -> Prop) : Prop :=
  forall a b c, r a b -> r a c -> joinable r b c.

(* 合流:多步分叉可連接。 *)
Definition confluent {A : Type} (r : A -> A -> Prop) : Prop :=
  forall a b c, star r a b -> star r a c -> joinable r b c.

(* 正規形:無出步。 *)
Definition nf {A : Type} (r : A -> A -> Prop) (a : A) : Prop :=
  forall b, ~ r a b.

(* 強正規化:無無限鏈 a0 → a1 → …。
 * Coq 的 Acc/well_founded 沿「方向反向」歸納(Acc R x : 所有 R y x 皆可達),
 * 故 SN = 對(反向的)step 關係為 well_founded:Acc 的歸納 IH 給出
 * 「forall a', r a a' -> P a'」—— 沿前向步推進,正合 Newman 證明所需。 *)
Definition sn {A : Type} (r : A -> A -> Prop) : Prop :=
  well_founded (fun x y => r y x).

(* --------------------------------------------------------------------- *)
(* 基本引理                                                                *)
(* --------------------------------------------------------------------- *)

Lemma star_trans (A : Type) (r : A -> A -> Prop) :
  forall a b c, star r a b -> star r b c -> star r a c.
Proof.
  intros a b c Hab Hbc.
  revert c Hbc.
  induction Hab as [a | a' b' c'' Hstep Htail IH];
    intros c Hbc; auto.
  apply star_step with b'; [exact Hstep | now apply IH].
Qed.

(* 正規形出發的多步恆為自身:證明鏈條上第一步即矛盾。 *)
Lemma star_nf_id (A : Type) (r : A -> A -> Prop) :
  forall a b, star r a b -> nf r a -> b = a.
Proof.
  intros a b Hab Hnf.
  revert Hnf.
  induction Hab as [a' | a' b' c' Hstep Htail IH]; intros Hnf.
  - reflexivity.
  - exfalso. now apply Hnf in Hstep.
Qed.

(* --------------------------------------------------------------------- *)
(* R1:Newman 引理                                                        *)
(* --------------------------------------------------------------------- *)

(* 主證明:於 Acc(良基)上同時歸納兩個性質 ——
 *   Q a:一步( r a b )對上多步( star r a c )可連接(「strip」性質);
 *   P a:多步對多步可連接(即 confluence at a)。                            *)
Theorem newman (A : Type) (r : A -> A -> Prop) :
  sn r -> wcr r -> confluent r.
Proof.
  intros Hsn Hwcr.
  unfold confluent.
  assert (Hpq : forall a, Acc (fun x y => r y x) a ->
              (forall b c, star r a b -> star r a c -> joinable r b c) /\
              (forall b c, r a b -> star r a c -> joinable r b c)).
  { intros a Hacc.
    induction Hacc as [a Hacc' IH].
    (* ---- Q a:一步對多步 ---- *)
    assert (Qa : forall b c, r a b -> star r a c -> joinable r b c).
    { intros b c Hr Hac.
      (* 歸納於多步推導;索引(出發點)被抽取,以 a 為重命名。 *)
      induction Hac as [a | a c1 c2 Hac1 Hc1c2 IHac].
      + (* c = a:同一狀態,取證人 b *)
        exists b. split; [apply star_refl |].
        apply star_step with b; [exact Hr | apply star_refl].
      + (* a → c1 →* c2:P(c1) 連接 wcr 產物與 c2 *)
        destruct (Hwcr a b c1 Hr Hac1) as [d0 [Hbd0 Hc1d0]].
        destruct (IH c1 Hac1) as [Pc1 _].        (* r a c1 ⇒ P c1 可用 *)
        destruct (Pc1 d0 c2 Hc1d0 Hc1c2) as [f [Hd0f Hc2f]].
        exists f. split.
        * eapply star_trans; [exact Hbd0 | exact Hd0f].
        * exact Hc2f.
    }
    split; [| exact Qa].
    (* ---- P a:多步對多步 ---- *)
    intros b c Hab Hac.
    revert c Hac.
    induction Hab as [a1 | a1 b1 b2 Hax Hxb IHxb].
    + (* b = a:取證人 c *)
      intros c Hac. exists c. split; [exact Hac | apply star_refl].
    + intros c Hac.
      (* Q a 連接「一步 a→b1」與「多步 a→* c」 *)
      destruct (Qa b1 c Hax Hac) as [d [Hb1d Hcd]].
      (* P 在 b1(r a b1,故由 IH 可得):連接 b1→* b2 與 b1→* d *)
      destruct (IH b1 Hax) as [Pb1 _].
      destruct (Pb1 b2 d Hxb Hb1d) as [e [Hb2e Hde]].
      exists e. split; [exact Hb2e |].
      eapply star_trans; [exact Hcd | exact Hde].
  }
  intros a b c Hab Hac.
  apply (proj1 (Hpq a (Hsn a)) b c Hab Hac).
Qed.

(* --------------------------------------------------------------------- *)
(* R4:唯一正規形 + 正規形存在                                               *)
(* --------------------------------------------------------------------- *)

(* 唯一性:同一出發點的兩個正規形必相同(合流 + 正規形無步)。 *)
Theorem newman_unf (A : Type) (r : A -> A -> Prop) :
  sn r -> wcr r ->
  forall a n1 n2, star r a n1 -> star r a n2 -> nf r n1 -> nf r n2 ->
  n1 = n2.
Proof.
  intros Hsn Hwcr a n1 n2 Ha1 Ha2 Hn1 Hn2.
  destruct (newman A r Hsn Hwcr a n1 n2 Ha1 Ha2) as [d [Hd1 Hd2]].
  assert (d = n1) by (now apply (star_nf_id A r n1 d)).
  assert (d = n2) by (now apply (star_nf_id A r n2 d)).
  congruence.
Qed.

(* 存在性:SN 下每個狀態都有正規形。構造性歸納 + 排中律(僅用於判別
 * 「是否存在一步」,證人由 destruct 存在命題取得,不引用選擇公理)。 *)
Theorem exists_normal_form (A : Type) (r : A -> A -> Prop) :
  sn r -> forall a, exists n, star r a n /\ nf r n.
Proof.
  intros Hsn a.
  induction (Hsn a) as [a Hacc IH].
  destruct (classic (exists b, r a b)) as [[b Hab] | Hnob].
  - destruct (IH b Hab) as [n [Hbn Hnf]]. exists n. split.
    + apply star_step with b; [exact Hab | exact Hbn].
    + exact Hnf.
  - exists a. split; [apply star_refl |].
    intros b Hb. apply Hnob. exists b. exact Hb.
Qed.
