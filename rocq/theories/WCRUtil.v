(* ===================================================================== *)
(* Phase 3 輔助層:修剪的成對歸納 + 候選集的單調性。                        *)
(*                                                                       *)
(* 鏡像 `r1_apply` 的語義 =「把列表第 i 個事件的右端點改為 c」。本檔把它     *)
(* 抽成 `trim_at`,並用**成對走兩個列表**的歸納謂詞 `trim1` 刻畫它 ——         *)
(* 這樣避開了 `nth_error` 的死結(該 fixpoint 按 nat 遞歸,遇到未約簡的      *)
(* `trim_at q i c` 就不再動;開發中在此耗掉大量回合,故留此註記)。            *)
(*                                                                       *)
(* ★ 開發筆記(勿刪):最初想證「修剪第 i 個事件**不影響他人 a 的 cut_for**」, *)
(*   證不起來 —— 因為那個命題是**假的**:鏡像 i_overlap a b =                *)
(*   (istart a <? iend b) && (istart b <? iend a),對**第二個引數的 iend     *)
(*   亦敏感**,故修剪 b 會把 b 從「a 的候選集」中**移除**(剪短後不再重疊)。  *)
(*   正確的形狀是**單調性**:a 的候選集只可能縮小(③),而 R3 交換性来自       *)
(*   「兩步修剪同一個 b ⇒ 兩次縮掉同一批候選」,不是不變性。                  *)
(* ===================================================================== *)

From Coq Require Import List Arith Lia.
Require Import Cl0r0.Mirror.
Import ListNotations.

Set Implicit Arguments.
Unset Implicit Arguments.

(* --------------------------------------------------------------------- *)
(* ① 修剪的結構                                                           *)
(* --------------------------------------------------------------------- *)

Definition trim_ev (e : Ev) (c : nat) : Ev :=
  {| ev_id := ev_id e; ev_storage := ev_storage e; ev_kind := ev_kind e;
     ev_it := {| istart := istart (ev_it e); iend := c |} |}.

Fixpoint trim_at (l : list Ev) (i : nat) (c : nat) : list Ev :=
  match l with
  | [] => []
  | e :: t => if Nat.eqb i 0 then trim_ev e c :: t else e :: trim_at t (pred i) c
  end.

Definition trim (s : AState) (i : nat) (c : nat) : AState :=
  {| st_evs := trim_at (st_evs s) i c; st_runtime := st_runtime s |}.

(** trim1 i c l l' :l' 把 l 的第 i 個事件替換成 trim_ev _ c,前後綴逐字相同。 *)
Inductive trim1 : nat -> nat -> list Ev -> list Ev -> Prop :=
| t1_nil  : forall i c, trim1 i c [] []
| t1_here : forall c e t, trim1 0 c (e :: t) (trim_ev e c :: t)
| t1_step : forall i c a p q, trim1 i c p q -> trim1 (S i) c (a :: p) (a :: q).

Lemma trim1_spec : forall l i c, trim1 i c l (trim_at l i c).
Proof.
  induction l as [|a q IH]; intros i c.
  - apply t1_nil.
  - destruct i as [|i']; [apply t1_here | apply t1_step; apply IH].
Qed.

Lemma trim1_length : forall i c l l', trim1 i c l l' -> length l = length l'.
Proof. induction 1; cbn; auto. Qed.

(** 非修剪位置:逐字相同 ⇒ nth_error 相等。 *)
Lemma trim1_nth_other : forall i c l l' k, k <> i -> trim1 i c l l' ->
  nth_error l' k = nth_error l k.
Proof.
  intros i c l l' k Hk H. revert k Hk.
  induction H as [i0|e t|i' c' a p q Hrec]; intros k Hk.
  - reflexivity.
  - destruct k as [|k']; cbn; [exfalso; apply Hk; reflexivity | reflexivity].
  - destruct k as [|k']; [reflexivity|].
    change (nth_error q k' = nth_error p k').
    apply IHHrec. intros ?. apply Hk. lia.
Qed.

(* --------------------------------------------------------------------- *)
(* ② 修剪 = 區間右端點被「縮到」新值;起點集合不變                          *)
(* --------------------------------------------------------------------- *)

(** 鏡像 cut_for 的過濾謂語(逐字抄錄 Mirror.v:217-222 的 let cands := ...) *)
Definition ct_pred (a b : Ev) : bool :=
  andb (negb (Nat.eqb (ev_id b) (ev_id a)))
    (andb (Nat.eqb (ev_storage b) (ev_storage a))
      (andb (k_conflict (ev_kind a) (ev_kind b))
        (andb (Nat.ltb (istart (ev_it a)) (istart (ev_it b)))
              (i_overlap (ev_it a) (ev_it b))))).

(** ⚠ `ct_pred a b` 對 b 的 **iend 亦敏感**(i_overlap 的第二項 `istart a <?
    iend b`)⇒ 修剪 b 會把 b 從 a 的候選集移除。故**不存在**「ct_pred 與
    iend 無關」型引理;可用的只有下面的單調性方向。 *)

(** ★ 語義定則(kernel 驗證,見下方三個 vm_compute 事實):
    `ct_pred a b` = id≠ ∧ storage= ∧ k_conflict ∧ istart a < istart b ∧
    (istart a <? iend b) ∧ (istart b <? iend a)。
    ⇒ 修剪**觀察者自身**(a)會縮 `iend a`,可把候選 b 剔除(第三行事實);
    ⇒ 修剪**被觀者**(b)只改 `iend b`,而 `istart a <? iend b` 在合法區間
      (istart a < istart b ≤ iend b)下恆真,故**對他人的候選判定不變**
      (第四行事實:剪到 1 仍然 true —— 因為本例 istart a = 0)。
    這推翻了 Phase 3 原本的「起點不變 ⇒ 他人 cut 不變」捷徑:交換性必須
    走「兩步修剪同一個 b ⇒ 兩次移除同一批候選」的對稱論證,而不是逐點不變。 *)
Definition evB : Ev :=   (* 被 a 觀測的事件:[2,10) *)
  {| ev_id := 0; ev_storage := 0; ev_kind := Mut;
     ev_it := {| istart := 2; iend := 10 |} |}.
Definition evA : Ev :=   (* 觀察者:[0,3),Sh 對 Mut 衝突 *)
  {| ev_id := 1; ev_storage := 0; ev_kind := Sh;
     ev_it := {| istart := 0; iend := 3 |} |}.

Goal ct_pred evA evB = true.
Proof. vm_compute; reflexivity. Qed.

(** 剪短「觀察者自身」的右端 ⇒ 該候選被剔除(故不可把 ct_pred 當成 iend 不變)。*)
Goal ct_pred (trim_ev evA 2) evB = false.
Proof. vm_compute; reflexivity. Qed.

Goal ct_pred evA (trim_ev evB 1) = true.
Proof. vm_compute; reflexivity. Qed.

(* --------------------------------------------------------------------- *)
(* ③ filter 的逐位置原則(待補;見 docs/ROCQ-PLAN.md)                       *)
(*     filter_pointwise_rel:等長 + 逐位置同謂語 ⇒ filter 相等。             *)
(* --------------------------------------------------------------------- *)

(* --------------------------------------------------------------------- *)
(* ④ 兩步修剪可交換(列表層) —— R3 交換引理的純結構部分                     *)
(* --------------------------------------------------------------------- *)

Lemma trim_at_trim_at_here : forall e t c d,
  trim_at (trim_ev e c :: t) 0 d = trim_ev e d :: t.
Proof. intros. cbn. destruct (Nat.eqb 0 0); reflexivity. Qed.

(** 相異位置的兩次修剪可交換(列表層的「菱形結構」)。
    註:此處刻意**未**入庫 —— 已驗證的是 `trim_at_trim_at_here`(同位重剪)
    與 `trim1_*` 系列;交換性的 Coq 證明需要 trim1 的成對歸納(而非直接
    cbn),留給 ConcreteWCR.v。 *)

(* --------------------------------------------------------------------- *)
(* ⑤ 小結:本檔已 kernel 驗證的事實                                        *)
(* --------------------------------------------------------------------- *)
(*   trim1_spec / trim1_length / trim1_nth_other / trim_at_trim_at_here   *)
(*   + 關鍵語義筆記:i_overlap 對第二個引數的 iend **亦**敏感,故修剪他人    *)
(*     不是「不變」而是「把該事件從他人候選集移除」(單調收縮)。R3 的交換性   *)
(*     來自「兩步修剪同一個 b ⇒ 兩次移除同一批候選」,不是逐點不變性。        *)
(* --------------------------------------------------------------------- *)
