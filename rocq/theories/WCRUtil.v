(* ===================================================================== *)
(* Phase 3 輔助層:修剪的成對歸納 + 候選集的「恰好移除」刻畫。              *)
(*                                                                       *)
(* 鏡像 `r1_apply` 的語義 =「把 id 等於 i 的那個事件的右端點改為 cut」。     *)
(* 本檔把它抽成**位置版** `trim_at`(列表第 i 個),並用一個成對走兩個列表     *)
(* 的歸納謂詞 `trim1` 精確刻畫 —— 這樣避開了 `nth_error` 的死結:該         *)
(* fixpoint 按 nat 遞歸,遇到未約簡的 `trim_at q i c` 就不再約簡            *)
(* (開發中在此耗掉大量回合,故留此註記,勿重蹈)。                            *)
(*                                                                       *)
(* ★ 語義定則(kernel 驗證,見本檔末 ④ 的三個 vm_compute 事實):              *)
(*   原計劃假設「修剪只改 iend ⇒ 他人 cut_for 不變」——**這是假的**。         *)
(*   Mirror.cut_for 的候選謂語含 `istart a <? iend b`,i_overlap 對**第二     *)
(*   個引數的 iend 亦敏感** ⇒ 修剪 b 會把 **b 自己**從他人候選集移除。        *)
(*   正確的形狀是「**恰好移除被剪者**」(③),而 R3 的交換性正靠這個對稱性。  *)
(* ===================================================================== *)

From Coq Require Import List Arith Lia.
Require Import Cl0r0.Mirror.
Import ListNotations.
(* 鏡像 Mirror.v 一樣開 bool_scope,才寫得出 `&&` 匹配。 *)
Local Open Scope bool_scope.

Set Implicit Arguments.
Unset Implicit Arguments.

(* --------------------------------------------------------------------- *)
(* ① 修剪的結構刻畫                                                       *)
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

(** 被剪位置的形狀:由 `trim1` 的**構造**直接可讀(第 i 位是 `trim_ev _ c`,
    前後綴逐字相同),不需要 `nth_error` 版本的引理 —— 實測顯示
    「`nth_error l i = Some e → nth_error l' i = Some (trim_ev e c)`」這類
    陳述要把 `H : trim1 i c l l'` 與 `l`,`l'` 一起 generalize 才能歸納,
    而 Coq 8.20 因 `H` 同時依賴 `l` 與 `l'` 拒絕任何單邊 revert。
    → 需要時請改用「把 trim1 改成歸納**函數**(return 出 l')」的寫法,
      不要再在 relation + nth_error 的组合上耗時間。 *)

(** 同位重剪:再剪一次**只換 cut**,不會雙重施加。R3 的中間態計算要用它。 *)
Lemma trim_at_trim_at_here : forall e t c d,
  trim_at (trim_ev e c :: t) 0 d = trim_ev e d :: t.
Proof. reflexivity. Qed.

(* --------------------------------------------------------------------- *)
(* ② 鏡像 cut_for 的候選謂語(逐字抄錄 Mirror.v:217-222)                  *)
(* --------------------------------------------------------------------- *)

Definition ct_pred (a b : Ev) : bool :=
  andb (negb (Nat.eqb (ev_id b) (ev_id a)))
    (andb (Nat.eqb (ev_storage b) (ev_storage a))
      (andb (k_conflict (ev_kind a) (ev_kind b))
        (andb (Nat.ltb (istart (ev_it a)) (istart (ev_it b)))
              (i_overlap (ev_it a) (ev_it b))))).

(** 與 Mirror.cut_for 內部的 let cands := filter _ 逐字相同(欄位順序也一致)。 *)
Lemma cut_for_is_filter : forall l a,
  cut_for l a =
  fold_left (fun acc b =>
      match acc with
      | None => Some (istart (ev_it b))
      | Some c => Some (Nat.min c (istart (ev_it b)))
      end)
    (filter (ct_pred a) l) None.
Proof. reflexivity. Qed.

Ltac peel :=
  repeat (match goal with
          | H : andb _ _ = true |- _ =>
              apply Bool.andb_true_iff in H; destruct H
          end).

Lemma andb_r : forall p q, (p && q) = true -> q = true.
Proof. intros p q H. apply Bool.andb_true_iff in H. tauto. Qed.
Lemma andb_l : forall p q, (p && q) = true -> p = true.
Proof. intros p q H. apply Bool.andb_true_iff in H. tauto. Qed.

(** 候選謂語的兩個直接後果。 *)
Lemma ct_pred_start_lt : forall a b, ct_pred a b = true ->
  istart (ev_it a) < istart (ev_it b).
Proof.
  intros a b H. unfold ct_pred in H. peel.
  apply Nat.ltb_lt; first [assumption | apply andb_l; assumption
                          | apply andb_r; assumption].
Qed.

(** ★ 良構性:必須**显式假設**,不能靠「鏡像不會生成倒掛區間」蒙混。
    本輪曾想以 `examples/r3_wf.rs` 探針證明「不需要 wf」——**那個結論無效**:
    `enumerate_states` 的生成式是 Rust 的 `for end in (start+1)..=max_coord`、
    Rocq 的 `ends_for m s := iend := s + S d`,兩者**由構造排除** istart > iend
    (探針實測:n=3 m=5 宇宙 27,000 個狀態,`含倒掛狀態=0`),
    在倒掛桶裡**不可能**出現反例 ⇒ 該探針對「要不要 wf」是**套套邏輯**,零資訊。
    而 `AState` 的型別**允許**倒掛事件,故:
      (a) 想在 Rocq 對**所有** AState 證 R3 ⇒ 「剪到 cut 即從他人候選集消失」為假
          (a=[5,2) 倒掛、b=[0,10)、剪 a 後 `0 <? 2` 仍真 ⇒ a 仍是 b 的候選);
      (b) 正確寫法是把良構性當**不變量**帶進定理:
            wf_state s := forall e, In e (st_evs s) -> istart (ev_it e) <? iend (ev_it e).
          並證 CT 步保持它 —— 這需要「cut_for 的結果 > istart a」一支引理:
            cut_for_gt_start : cut_for l a = Some c -> istart (ev_it a) <? c = true
          (證明策略已設計完成:把 Mirror 的 left-fold-min 轉寫成右折 `minopt`,
           再證 `minopt f l = Some d -> (forall y, In y l -> P y) -> P d`,
           其中 P := `0 <? f y`;兩邊等價用長度歸納。見下方「未證清單」。) *)

(** `ct_pred_trim_right_ok`(「剪 b 的右端不影響 a 對 b 的候選判定」)經檢查
    **不成立**,已從本檔移除:剪短 b 的 `iend` 會改變 `i_overlap a b` 的第二個
    合取項 `istart a <? iend b`,該項與 b 的原始 `iend` 有关,不是剪後必然保持。
    ⇒ 交換引理不能靠「候選判定逐點不變」,必須走「**恰好移除被剪者**」的形狀,
      而那需要 `cut_for_gt_start`(未證) + 良構性不變量(未形式化)。 *)

(* --------------------------------------------------------------------- *)
(* ④ 語義定則的計算證據(kernel 複驗;不可刪 —— 這是「捷徑為假」的證物)      *)
(* --------------------------------------------------------------------- *)

Definition evB : Ev :=   (* 被 a 觀測的事件:[2,10) *)
  {| ev_id := 0; ev_storage := 0; ev_kind := Mut;
     ev_it := {| istart := 2; iend := 10 |} |}.
Definition evA : Ev :=   (* 觀察者:[0,3),Sh 對 Mut 衝突 *)
  {| ev_id := 1; ev_storage := 0; ev_kind := Sh;
     ev_it := {| istart := 0; iend := 3 |} |}.

Goal ct_pred evA evB = true.
Proof. vm_compute; reflexivity. Qed.

(** 剪「觀察者自身」的右端 ⇒ 該候選被剔除(故不可宣稱 ct_pred 與 iend 無關)。 *)
Goal ct_pred (trim_ev evA 2) evB = false.
Proof. vm_compute; reflexivity. Qed.

(** 剪「被觀者」的右端、但不剪到 cut ⇒ 判定可能不變(捷徑看起來成立的原因)。 *)
Goal ct_pred evA (trim_ev evB 1) = true.
Proof. vm_compute; reflexivity. Qed.
