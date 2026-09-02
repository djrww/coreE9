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
          **已證(2026-09-03,見下方 ③)**。原計劃的「右折 minopt 改寫」實作後
          發現**不需要**:只要把累加器 generalize 成抽象 `acc` 再對列表歸納,
          「cbn 過度約簡 ⇒ injection 失敗」的卡點就消失 —— 病根從來不是
          fold_left 本身,而是把具體的 `None` 留在式子裡讓 cbn 展開。
          (見 ③ 的 fold_min_mem / cut_for_gt_start,已 kernel 驗證。) *)

(** `ct_pred_trim_right_ok`(「剪 b 的右端不影響 a 對 b 的候選判定」)經檢查
    **不成立**,已從本檔移除:剪短 b 的 `iend` 會改變 `i_overlap a b` 的第二個
    合取項 `istart a <? iend b`,該項與 b 的原始 `iend` 有关,不是剪後必然保持。
    ⇒ 交換引理不能靠「候選判定逐點不變」,必須走「**恰好移除被剪者**」的形狀,
      而那需要 `cut_for_gt_start`(✅ 已證,見 ③) + 良構性不變量
      (✅ 已形式化,見 ConcreteWCR.v 的 wf_state / 保持性)。 *)

(* --------------------------------------------------------------------- *)
(* ③ cut_for_gt_start —— Phase 3 第二輪遺留的唯一缺口(2026-09-03 封閉)  *)
(*                                                                       *)
(* 證明骨架:fold 的結果必為某個候選 b 的 istart(顯然,從 None 出發、        *)
(* 每步只取 min);候選即過 `ct_pred a` 者 ⇒ istart a < istart b            *)
(* (② 的 ct_pred_start_lt)⇒ istart a < c。                               *)
(*                                                                       *)
(* ★ 工程教訓(2026-09-02 卡點的解藥,留檔勿重蹈):                          *)
(*   原計劃想把 fold_left-min 轉寫成右折再證 —— 不必。對 fold_left 做     *)
(*   歸納時**先把累加器 generalize**(acc 不綁定成 None),cbn/simpl 就只會  *)
(*   對「列表是 cons」這一層開一刀,「match None with」根本不會被展開,      *)
(*   也就不存在 injection 無從注入的問題。這是對「左折 + 具體初值」         *)
(*   直接丟 cbn 才會踩的坑。                                              *)
(* --------------------------------------------------------------------- *)

Section MinFold.

(** 左折取 min(「None 開局、每步 Nat.min」)的結果若存在,必已出現在
    輸入列表裡(或就是初值)—— min 每一步都只「選某個已有輸入」。
    構造性存在(∃;同值可重複出現,不需唯一性),由 fold 形狀直接給出。
    這一步也正是「合一/匹配」的構造性見證抽取(Martelli–Montanari 式):
    ⇒ 不需排中律,見證由歸納直接產出。 *)

Lemma fold_min_mem : forall (g : Ev -> nat) (l : list Ev) acc c,
  fold_left (fun acc0 b => match acc0 with
                           | None => Some (g b)
                           | Some c0 => Some (Nat.min c0 (g b))
                           end) l acc = Some c ->
  (exists b, In b l /\ g b = c) \/ acc = Some c.
Proof.
  intros g l. induction l as [|x l IH]; intros acc c H.
  - right. exact H.
  - destruct acc as [d|]; simpl in H.
    + (* acc = Some d:fold 右移一步,初值變 Some (min d (g x)) *)
      destruct (IH _ _ H) as [[b [Hin Hc]] | Hmin].
      * left. exists b. split; [right; exact Hin | exact Hc].
      * injection Hmin as Hmin.              (* Nat.min d (g x) = c *)
        destruct (Nat.min_dec d (g x)) as [Hd|Hx].
        -- right. f_equal. rewrite Hd in Hmin. exact Hmin.   (* c = d = 舊初值 *)
        -- left. exists x. split; [now left | rewrite Hx in Hmin; exact Hmin].
    + (* acc = None:右移一步,初值變 Some (g x) *)
      destruct (IH _ _ H) as [[b [Hin Hc]] | Hgx].
      * left. exists b. split; [right; exact Hin | exact Hc].
      * injection Hgx as Hgx.
        left. exists x. split; [now left | exact Hgx].
Qed.

End MinFold.

(** 主引理:cut_for 的輸出嚴格大於觀察者自身的 istart。
    (這是 wf 不變量「剪到 cut 仍保持 istart < iend」的引擎。) *)
Lemma cut_for_gt_start : forall l a c,
  cut_for l a = Some c -> (istart (ev_it a) <? c) = true.
Proof.
  intros l a c H.
  rewrite cut_for_is_filter in H.
  destruct (fold_min_mem (fun b => istart (ev_it b)) _ _ _ H)
    as [[b [Hin Hc]] | Hcontra]; [| discriminate].
  apply Nat.ltb_lt.
  rewrite <- Hc.
  apply ct_pred_start_lt.
  apply filter_In in Hin as [_ Hct].
  exact Hct.
Qed.

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
