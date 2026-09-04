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
(* ③ b 紅邊幾何:k_clag 的「剪到 min 起點 ⇒ 重疊消失」與「剪不新增交集」      *)
(* --------------------------------------------------------------------- *)

(** 剪右端到 `c`(完全落在 a 原區間內)⇒ 與 `b` 的**重疊**只可能變少:
    即剪後(仍落在 a 原區間內)重合 ⇒ 剪前也重合。 *)
Lemma i_overlap_cut_implies_overlap : forall a b c,
  istart (ev_it a) < c < iend (ev_it a) ->
  i_overlap {| istart := istart (ev_it a); iend := c |} (ev_it b) = true ->
  i_overlap (ev_it a) (ev_it b) = true.
Proof.
  intros a b c Hlt Hov. unfold i_overlap in *. simpl in *.
  rewrite Bool.andb_true_iff in Hov. destruct Hov as [H1 H2].
  apply Bool.andb_true_iff. split; [exact H1|].
  apply Nat.ltb_lt. apply Nat.ltb_lt in H2. lia.
Qed.

(** 剪到 `c`(≤ 候選 b 的起點)⇒ a 與 b 不再重疊。 *)
Lemma i_overlap_after_cut : forall a b c,
  c <= istart (ev_it b) ->
  i_overlap (ev_it (trim_ev a c)) (ev_it b) = false.
Proof.
  intros a b c Hc. unfold i_overlap. simpl.
  rewrite Bool.andb_false_iff. right. apply Nat.ltb_ge. exact Hc.
Qed.

(** 從一條 CT 規則(其 cut 來自 cut_for)抽取「min 起點候選」b:o 是 a 的候選
    (同 storage、kind 衝突、當前重疊)—— 且**剪後與 a 不再重疊**。
    這正是「剪 a 到 c 會刪掉與 b 的紅邊」的種子;配合 runtime=[] 即
    `ct_guard_redundant` / `R3_ct_wcr` 的引擎(見 docs/R3-NEXT-STEP.md §四)。 *)
Lemma ct_cut_min_drops :
  forall l a c,
    cut_for l a = Some c ->
    exists b, ct_pred a b = true /\ istart (ev_it b) = c /\
              i_overlap (ev_it (trim_ev a c)) (ev_it b) = false.
Proof.
  intros l a c Hcut.
  rewrite cut_for_is_filter in Hcut.
  destruct (fold_min_mem (fun b => istart (ev_it b)) _ _ _ Hcut)
    as [[b [Hin Hc]] | Hcontra]; [| discriminate].
  apply filter_In in Hin. destruct Hin as [_ Hpred].
  exists b. split; [exact Hpred|]. split; [exact Hc|].
  apply i_overlap_after_cut. rewrite <- Hc. apply Nat.le_refl.
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

(* --------------------------------------------------------------------- *)
(* ⑤ 紅邊「剪一事件 ⇒ 計數不增」(單調,2026-09-03 重建)。                  *)
(*                                                                       *)
(* 這是「紅邊嚴格遞減」的**非嚴格**一半,是 R3 橋樑的地基:                    *)
(* 剪一事件的右端只會使與它重疊的候選**減少**(i_overlap 對右端單調),       *)
(* 其它事件不變 ⇒ 紅邊計數 `length (red_edges_aux …)` 不增。               *)
(* --------------------------------------------------------------------- *)

(* red 謂語:與 Mirror.red_edges_aux 內聯的 `p` 逐字一致(抽出方便帶型)。 *)
Definition red_p (rt : list (nat*nat)) (a b : Ev) : bool :=
  andb (Nat.eqb (ev_storage b) (ev_storage a))
    (andb (k_conflict (ev_kind a) (ev_kind b))
      (andb (i_overlap (ev_it a) (ev_it b))
        (negb (rt_mem rt (Nat.min (ev_id a) (ev_id b)) (Nat.max (ev_id a) (ev_id b)))))).

Lemma overlap_trim_left : forall (e b : Ev) (c : nat),
  c <= iend (ev_it e) ->
  i_overlap (ev_it (trim_ev e c)) (ev_it b) = true ->
  i_overlap (ev_it e) (ev_it b) = true.
Proof.
  intros e b c Hc Hov. unfold i_overlap in *. simpl in *.
  rewrite Bool.andb_true_iff in Hov. destruct Hov as [H1 H2].
  apply Bool.andb_true_iff. split. { exact H1. }
  apply Nat.ltb_lt. apply Nat.ltb_lt in H2. lia.
Qed.

Lemma overlap_trim_right : forall (e b : Ev) (c : nat),
  c <= iend (ev_it b) ->
  i_overlap (ev_it e) (ev_it (trim_ev b c)) = true ->
  i_overlap (ev_it e) (ev_it b) = true.
Proof.
  intros e b c Hc Hov. unfold i_overlap in *. simpl in *.
  rewrite Bool.andb_true_iff in Hov. destruct Hov as [H1 H2].
  apply Bool.andb_true_iff. split.
  { apply Nat.ltb_lt. apply Nat.ltb_lt in H1. lia. }
  { exact H2. }
Qed.

Lemma red_p_trim_left : forall rt e b c,
  c <= iend (ev_it e) ->
  red_p rt (trim_ev e c) b = true -> red_p rt e b = true.
Proof.
  intros rt e b c Hc H. unfold red_p in H.
  apply Bool.andb_true_iff in H. destruct H as [Hs H1].
  apply Bool.andb_true_iff in H1. destruct H1 as [Hk H2].
  apply Bool.andb_true_iff in H2. destruct H2 as [Hov Hn].
  apply Bool.andb_true_iff. split.
  { exact Hs. }
  apply Bool.andb_true_iff. split.
  { exact Hk. }
  apply Bool.andb_true_iff. split.
  { apply (overlap_trim_left e b c Hc Hov). }
  { exact Hn. }
Qed.

Lemma red_p_trim_right : forall rt e b c,
  c <= iend (ev_it b) ->
  red_p rt e (trim_ev b c) = true -> red_p rt e b = true.
Proof.
  intros rt e b c Hc H. unfold red_p in H.
  apply Bool.andb_true_iff in H. destruct H as [Hs H1].
  apply Bool.andb_true_iff in H1. destruct H1 as [Hk H2].
  apply Bool.andb_true_iff in H2. destruct H2 as [Hov Hn].
  apply Bool.andb_true_iff. split.
  { exact Hs. }
  apply Bool.andb_true_iff. split.
  { exact Hk. }
  apply Bool.andb_true_iff. split.
  { apply (overlap_trim_right e b c Hc Hov). }
  { exact Hn. }
Qed.

Lemma red_edges_aux_cons : forall (a : Ev) (t : list Ev) rt,
  red_edges_aux (a :: t) rt =
  map (fun b => (Nat.min (ev_id a) (ev_id b), Nat.max (ev_id a) (ev_id b)))
      (filter (red_p rt a) t) ++ red_edges_aux t rt.
Proof. intros. reflexivity. Qed.

(** `ct_pred a b = true` 且該 (min,max) 不在 runtime ⇒ `red_p rt a b = true`。
    這是「候選 b 是 a 的紅邊」的種子:ct_pred 給 storage/kind/重疊,再補 runtime 空。 *)
Lemma red_p_rt_ct_pred : forall rt a b,
  ct_pred a b = true ->
  rt_mem rt (Nat.min (ev_id a) (ev_id b)) (Nat.max (ev_id a) (ev_id b)) = false ->
  red_p rt a b = true.
Proof.
  intros rt a b Hct Hrt. unfold ct_pred, red_p in *. simpl in *.
  apply Bool.andb_true_iff in Hct. destruct Hct as [_ Hct1].
  apply Bool.andb_true_iff in Hct1. destruct Hct1 as [Hs Hct2].
  apply Bool.andb_true_iff in Hct2. destruct Hct2 as [Hk Hct3].
  apply Bool.andb_true_iff in Hct3. destruct Hct3 as [_ Hov].
  apply Bool.andb_true_iff.
  split; [exact Hs|].
  apply Bool.andb_true_iff.
  split; [exact Hk|].
  apply Bool.andb_true_iff.
  split; [exact Hov|].
  rewrite Hrt. reflexivity.
Qed.


(** 紅邊清單的 `pk` = (min id, max id);`red_p` 對稱 ⇒ 先出現者與後出現者互配。 *)
Definition pk (x y : Ev) : nat*nat := (Nat.min (ev_id x)(ev_id y), Nat.max (ev_id x)(ev_id y)).

Lemma red_p_sym : forall rt a b, red_p rt a b = red_p rt b a.
Proof.
  intros rt a b. unfold red_p. simpl.
  rewrite Nat.eqb_sym. rewrite k_conflict_sym. rewrite i_overlap_sym.
  rewrite Nat.min_comm, Nat.max_comm. reflexivity.
Qed.

Lemma min_plus_max : forall n m, Nat.min n m + Nat.max n m = n + m.
Proof.
  intros n m. destruct (Nat.lt_ge_cases n m) as [Hlt|Hge].
  - rewrite Nat.min_l, Nat.max_r; lia.
  - rewrite Nat.min_r, Nat.max_l; lia.
Qed.

Lemma pk_minmax : forall a b1 b2,
  ev_id b1 <> ev_id a -> ev_id b2 <> ev_id a ->
  pk a b1 = pk a b2 -> ev_id b1 = ev_id b2.
Proof.
  intros a b1 b2 H1 H2 Heq. unfold pk in Heq.
  injection Heq as Hm Hx.
  assert (Heq2 : ev_id a + ev_id b1 = ev_id a + ev_id b2).
  { rewrite <- min_plus_max. rewrite Hm. rewrite Hx. rewrite min_plus_max. reflexivity. }
  lia.
Qed.

(** 唯一 id ⇒ 同 id 即同事件。 *)
Lemma nodup_map_ev_id_inj : forall l x y,
  NoDup (map ev_id l) -> In x l -> In y l -> ev_id x = ev_id y -> x = y.
Proof.
  induction l as [|a t IH]; intros x y Hnd Hx Hy Heq.
  - contradiction.
  - simpl in Hnd. apply NoDup_cons_iff in Hnd. destruct Hnd as [Ha Hnd_t].
    destruct Hx as [Hxa | Hxt]; destruct Hy as [Hya | Hyt].
    + subst x. subst y. reflexivity.
    + subst x. exfalso. apply Ha. apply in_map_iff. exists y. split; [exact (eq_sym Heq) | exact Hyt].
    + subst y. exfalso. apply Ha. apply in_map_iff. exists x. split; [exact Heq | exact Hxt].
    + apply IH; auto.
Qed.

Lemma NoDup_map_inj : forall (f : Ev -> nat*nat) l,
  NoDup l -> (forall x y, In x l -> In y l -> f x = f y -> x = y) ->
  NoDup (map f l).
Proof.
  intros f l. induction l as [|a t IH]; intros Hnd Hinj.
  - constructor.
  - apply NoDup_cons_iff.
    apply NoDup_cons_iff in Hnd. destruct Hnd as [Ha Hnd_t].
    split.
    + intro Hin. apply in_map_iff in Hin. destruct Hin as [x [Hfx Hx]].
      assert (Hxa : a = x).
      { apply Hinj with (x := a) (y := x); [now left | right; exact Hx | exact (eq_sym Hfx)]. }
      subst a. exfalso. exact (Ha Hx).
    + apply IH.
      * exact Hnd_t.
      * intros x y Hx Hy Heq. apply Hinj; [right; exact Hx | right; exact Hy | exact Heq].
Qed.

(** 若 x,y 都在列表中且互為紅邊 ⇒ (min id, max id) 是紅邊清單中的一個邊。
   無關順序:red_p 對稱,先出現者與後出現者互配。 *)
Lemma in_red_edges_aux : forall l rt x y,
  In x l -> In y l -> x <> y -> red_p rt x y = true ->
  In (pk x y) (red_edges_aux l rt).
Proof.
  intros l. induction l as [|a t IH]; intros rt x y Hx Hy Hnx Hred.
  - contradiction.
  - rewrite red_edges_aux_cons.
    destruct Hx as [Hxa|Hxt].
    + subst x.
      destruct Hy as [Hya|Hyt].
      * subst y. congruence.
      * apply in_or_app. left. apply in_map_iff. exists y. split.
        { unfold pk. reflexivity. }
        { apply filter_In. split; [exact Hyt| exact Hred]. }
    + destruct Hy as [Hya|Hyt].
      * subst y.
        apply in_or_app. left. apply in_map_iff. exists x. split.
        { unfold pk. f_equal. { apply Nat.min_comm. } { apply Nat.max_comm. } }
        { apply filter_In. split; [exact Hxt|]. rewrite red_p_sym in Hred. exact Hred. }
      * apply in_or_app. right. apply IH with (x := x) (y := y); auto.
Qed.

(** `In x t → In (ev_id x) (map ev_id t)`。 *)
Lemma In_ev_id_in_map : forall t x, In x t -> In (ev_id x) (map ev_id t).
Proof.
  intros t x Hx. apply in_map_iff. exists x. split; [reflexivity | exact Hx].
Qed.

Lemma id_ne_notin : forall t a b, ~ In (ev_id a) (map ev_id t) -> In b t -> ev_id b <> ev_id a.
Proof.
  intros t a b Hna Hin Hx. apply Hna. rewrite <- Hx. apply In_ev_id_in_map. exact Hin.
Qed.

Lemma pk_comp_has_a : forall a b, ev_id a <> ev_id b ->
  ev_id a = fst (pk a b) \/ ev_id a = snd (pk a b).
Proof.
  intros a b Hne. unfold pk. simpl.
  destruct (Nat.lt_ge_cases (ev_id a) (ev_id b)) as [Hlt|Hge].
  - left. rewrite (Nat.min_l _ _ (Nat.lt_le_incl _ _ Hlt)). reflexivity.
  - right. rewrite (Nat.max_l _ _ Hge). reflexivity.
Qed.

Lemma pk_comp_in_ids : forall t x y, In x t -> In y t ->
  In (fst (pk x y)) (map ev_id t) /\ In (snd (pk x y)) (map ev_id t).
Proof.
  intros t x y Hx Hy. unfold pk. simpl.
  destruct (Nat.le_ge_cases (ev_id x) (ev_id y)) as [Hle|Hge].
  - rewrite Nat.min_l, Nat.max_r; try apply Hle.
    split; [apply In_ev_id_in_map; exact Hx | apply In_ev_id_in_map; exact Hy].
  - rewrite Nat.min_r, Nat.max_l; try apply Hge.
    split; [apply In_ev_id_in_map; exact Hy | apply In_ev_id_in_map; exact Hx].
Qed.

(* min/max 的「回推」小件:`pk a b = pk x y` 時,x、y 的 id 皆屬 {id_a, id_b}。 *)
Lemma minmax_mem : forall u v, u = Nat.min u v \/ u = Nat.max u v.
Proof. intros u v. destruct (Nat.le_ge_cases u v) as [Hle|Hge].
  - left. rewrite (Nat.min_l _ _ Hle). reflexivity.
  - right. rewrite (Nat.max_l _ _ Hge). reflexivity. Qed.

Lemma minmax_mem_r : forall u v, v = Nat.min u v \/ v = Nat.max u v.
Proof. intros u v. destruct (Nat.le_ge_cases u v) as [Hle|Hge].
  - right. rewrite (Nat.max_r _ _ Hle). reflexivity.
  - left. rewrite (Nat.min_r _ _ Hge). reflexivity. Qed.

Lemma minmem : forall u v, Nat.min u v = u \/ Nat.min u v = v.
Proof. intros u v. destruct (Nat.le_ge_cases u v) as [H|H].
  - left. apply Nat.min_l. exact H.
  - right. apply Nat.min_r. exact H. Qed.

Lemma maxmem : forall u v, Nat.max u v = u \/ Nat.max u v = v.
Proof. intros u v. destruct (Nat.le_ge_cases u v) as [H|H].
  - right. apply Nat.max_r. exact H.
  - left. apply Nat.max_l. exact H. Qed.

Lemma minmax_pair : forall u v, u <> v ->
  (u = Nat.min u v \/ u = Nat.max u v) /\
  (v = Nat.min u v \/ v = Nat.max u v) /\
  Nat.min u v <> Nat.max u v.
Proof.
  intros u v Hne.
  destruct (Nat.lt_ge_cases u v) as [Hlt|Hge].
  - assert (Hle : u <= v) by lia.
    rewrite (Nat.min_l _ _ Hle). rewrite (Nat.max_r _ _ Hle).
    split; [now left |]. split; [now right |]. lia.
  - rewrite (Nat.min_r _ _ Hge). rewrite (Nat.max_l _ _ Hge).
    split; [now right |]. split; [now left |]. lia.
Qed.

(** 核心回推:相同邊 `pk a b = pk x y` 且 `id_a ≠ id_b` => x、y 的 id 皆屬 {id_a, id_b}。
    這是「邊出現在剪後 => 牽涉的是原事件對」的解析關鍵。 *)
Lemma pk_comp_in_ab : forall a b x y,
  ev_id a <> ev_id b ->
  pk a b = pk x y ->
  (ev_id x = ev_id a \/ ev_id x = ev_id b) /\
  (ev_id y = ev_id a \/ ev_id y = ev_id b).
Proof.
  intros a b x y Hnab Hpk. unfold pk in Hpk. injection Hpk as Hm Hmm.
  split.
  - destruct (minmax_mem (ev_id x) (ev_id y)) as [Hx|Hx].
    + rewrite <- Hm in Hx. rewrite Hx. apply minmem.
    + rewrite <- Hmm in Hx. rewrite Hx. apply maxmem.
  - destruct (minmax_mem_r (ev_id x) (ev_id y)) as [Hy|Hy].
    + rewrite <- Hm in Hy. rewrite Hy. apply minmem.
    + rewrite <- Hmm in Hy. rewrite Hy. apply maxmem.
Qed.




(** 紅邊清單中每個邊皆由兩個相異事件產生(唯 id 前提),配 `x ≠ y`。 *)
Lemma in_red_char : forall l rt p,
  NoDup (map ev_id l) ->
  In p (red_edges_aux l rt) ->
  exists x y, In x l /\ In y l /\ x <> y /\ p = pk x y /\ red_p rt x y = true.
Proof.
  intros l rec p. induction l as [|a t IH]; intros Hnd Hp.
  - contradiction.
  - simpl in Hnd. apply NoDup_cons_iff in Hnd. destruct Hnd as [Ha Hnd_t].
    rewrite red_edges_aux_cons in Hp. apply in_app_or in Hp. destruct Hp as [Hm | Hr].
    + apply in_map_iff in Hm. destruct Hm as [b [Hp_eq Hb]].
      apply filter_In in Hb. destruct Hb as [Hbin Hpred].
      exists a. exists b. split; [now left|]. split; [right; exact Hbin|].
      split. { intro Hab. apply (id_ne_notin t a b Ha Hbin). subst b. reflexivity. }
      split. { symmetry. unfold pk in Hp_eq. exact Hp_eq. } { exact Hpred. }
    + destruct (IH Hnd_t Hr) as [x [y [Hx [Hy [Hne [Hpeq Hred]]]]]].
      exists x. exists y. split; [right; exact Hx|]. split; [right; exact Hy|].
      split; [exact Hne|]. split; [exact Hpeq| exact Hred].
Qed.

(* map 塊的邊(含 id_a)不在 t 內邊中。 *)
Lemma map_edge_not_in_tail : forall t a rt b p,
  NoDup (map ev_id t) -> ~ In (ev_id a) (map ev_id t) -> In b t ->
  pk a b = p -> ~ In p (red_edges_aux t rt).
Proof.
  intros t a rt b p Hnd_t Ha Hbin Hp_eq Hr.
  destruct (in_red_char t rt p Hnd_t Hr) as [x [y [Hx [Hy [Hxy [Hpeq Hred]]]]]].
  assert (Hcomp : ev_id a = fst (pk a b) \/ ev_id a = snd (pk a b)).
  { apply (pk_comp_has_a a b). intro Habs.
    apply (id_ne_notin t a b Ha Hbin). symmetry. exact Habs. }
  destruct (pk_comp_in_ids t x y Hx Hy) as [Hxid Hyid].
  assert (Hpk : pk a b = pk x y). { rewrite Hp_eq. exact Hpeq. }
  rewrite Hpk in Hcomp.
  destruct Hcomp as [Hc | Hc].
  - apply Ha. rewrite Hc. exact Hxid.
  - apply Ha. rewrite Hc. exact Hyid.
Qed.

(** `red_edges_aux` 無重複(唯 id 前提):map 邊含 id_a、t 內邊不含,故 `NoDup_app`。 *)
Lemma red_edges_aux_nodup : forall l rt,
  NoDup (map ev_id l) -> NoDup (red_edges_aux l rt).
Proof.
  intros l. induction l as [|a t IH]; intros rt Hnd.
  - simpl. constructor.
  - simpl in Hnd. apply NoDup_cons_iff in Hnd. destruct Hnd as [Ha Hnd_t].
    rewrite red_edges_aux_cons.
    apply NoDup_app.
    + refine (NoDup_map_inj (fun b => pk a b) _ _ _).
      * apply NoDup_filter. apply NoDup_map_inv with (f := ev_id). exact Hnd_t.
      * intros b1 b2 Hb1 Hb2 Heq.
        apply filter_In in Hb1. destruct Hb1 as [Hb1t _].
        apply filter_In in Hb2. destruct Hb2 as [Hb2t _].
        apply (nodup_map_ev_id_inj t b1 b2 Hnd_t Hb1t Hb2t).
        apply (pk_minmax a b1 b2).
        -- apply (id_ne_notin t a b1 Ha Hb1t).
        -- apply (id_ne_notin t a b2 Ha Hb2t).
        -- exact Heq.
    + apply IH. exact Hnd_t.
    + intros p Hm Hr.
      apply in_map_iff in Hm. destruct Hm as [b [Hp_eq Hb]].
      apply filter_In in Hb. destruct Hb as [Hbin _].
      apply (map_edge_not_in_tail t a rt b p Hnd_t Ha Hbin Hp_eq Hr).
Qed.

Lemma filter_sub_len : forall (l : list Ev) (P Q : Ev -> bool),
  (forall b, In b l -> P b = true -> Q b = true) ->
  length (filter P l) <= length (filter Q l).
Proof.
  intros l. induction l as [|a t IH]; intros P Q H; simpl.
  - lia.
  - destruct (P a) eqn:Pa; destruct (Q a) eqn:Qa; simpl in *.
    + assert (Hle : length (filter P t) <= length (filter Q t))
        by (apply IH; intros; apply H; [right|]; auto).
      apply (proj1 (Nat.succ_le_mono _ _) Hle).
    + exfalso. assert (Hqa : Q a = true) by (apply H; [now left | exact Pa]).
      rewrite Qa in Hqa. discriminate.
    + assert (Hle : length (filter P t) <= length (filter Q t))
        by (apply IH; intros; apply H; [right|]; auto).
      exact (Nat.le_trans _ _ _ Hle (le_S _ _ (Nat.le_refl _))).
    + apply IH; intros; apply H; [right|]; auto.
Qed.

Lemma filter_trim_at_le : forall t i c P,
  (forall e, nth_error t i = Some e -> P (trim_ev e c) = true -> P e = true) ->
  length (filter P (trim_at t i c)) <= length (filter P t).
Proof.
  intros t. induction t as [|e t' IH]; intros i c P H.
  - cbn. reflexivity.
  - destruct i as [|i']; cbn [trim_at Nat.eqb].
    + simpl. destruct (P (trim_ev e c)) eqn:Pt; destruct (P e) eqn:Pe; simpl.
      * apply (proj1 (Nat.succ_le_mono _ _)). apply Nat.le_refl.
      * exfalso. assert (Hpe : P e = true) by (apply H; [cbn; reflexivity | exact Pt]).
        rewrite Pe in Hpe. discriminate.
      * apply le_S. exact (Nat.le_refl _).
      * apply Nat.le_refl.
    + simpl. destruct (P e) eqn:Pe; simpl.
      * apply (proj1 (Nat.succ_le_mono _ _)).
        apply IH. intros x Hx Hpx. apply H. { cbn. apply Hx. } { exact Hpx. }
      * apply IH. intros x Hx Hpx. apply H. { cbn. apply Hx. } { exact Hpx. }
Qed.

(** §⑤ 主引理:剪一事件的右端 ⇒ 紅邊計數**不增**(單調)。
    前提:被剪事件(列表第 i 個)滿足 `c <= iend`(真收縮;CT 菜單由
    `cut_for_gt_start`+候選重疊保證)。 *)
Lemma r1_trim_red_le : forall l i c rt,
  (forall e, nth_error l i = Some e -> c <= iend (ev_it e)) ->
  length (red_edges_aux (trim_at l i c) rt) <= length (red_edges_aux l rt).
Proof.
  intros l. induction l as [|a t IH]; intros i c rt Hb.
  - cbn. lia.
  - destruct i as [|i'].
    + assert (Hsub : length (filter (red_p rt (trim_ev a c)) t) <=
                     length (filter (red_p rt a) t)).
      { apply filter_sub_len. intros b Hb' Hpb.
        apply (red_p_trim_left rt a b c). { apply Hb. cbn. reflexivity. } { exact Hpb. } }
      cbn [trim_at Nat.eqb]. rewrite !red_edges_aux_cons. rewrite !length_app. rewrite !length_map.
      apply Nat.add_le_mono. { exact Hsub. } { apply Nat.le_refl. }
    + assert (Hsub : length (filter (red_p rt a) (trim_at t i' c)) <=
                     length (filter (red_p rt a) t)).
      { apply filter_trim_at_le. intros e He Hpb.
        apply (red_p_trim_right rt a e c). { apply Hb. cbn. apply He. } { exact Hpb. } }
      assert (Hrec : length (red_edges_aux (trim_at t i' c) rt) <=
                     length (red_edges_aux t rt)).
      { apply IH. intros e He. apply Hb. cbn. apply He. }
      cbn [trim_at Nat.eqb]. rewrite !red_edges_aux_cons. rewrite !length_app. rewrite !length_map.
      apply Nat.add_le_mono. { exact Hsub. } { exact Hrec. }
Qed.

(** r1_apply 以 **id** 找事件,trim_at 以 **位置** 剪;`pos_of` 連接兩者。
    這是把「剪 id 事件」的規則套用到「位置版」單調引理的必要橋樑。 *)
Lemma r1_apply_eq_trim_at : forall l i c p,
  pos_of l i = Some p -> r1_apply l i c = Some (trim_at l p c).
Proof.
  intros l. induction l as [|e t IH]; intros i c p H; cbn in *.
  - discriminate.
  - destruct (Nat.eqb (ev_id e) i) eqn:E.
    + apply Nat.eqb_eq in E. subst i. injection H as Hp. subst p. reflexivity.
    + apply Nat.eqb_neq in E.
      destruct (pos_of t i) as [p'|] eqn:Hp; [|discriminate].
      injection H as HpS. subst p. rewrite IH with (p := p'); [|exact Hp].
      reflexivity.
Qed.

(** trim_ev 保 id(剪只改區間右端)。 *)
Lemma trim_ev_id : forall e c, ev_id (trim_ev e c) = ev_id e.
Proof. intros. reflexivity. Qed.

(** 成員提升(位置版):在位 `i` 剪過的列表,其成員要嘛是第 i 位事件 `trim_ev _ c` 化、
    要嘛原本就在原列表。位置版給出 `nth_error`(供剪條件 `c <= iend` 配對),比
    純 `In` 版更精準 —— 這是「剪後邊集 ⊆ 剪前邊集 `red_edges_aux_trim_subset`」
    的逐點提升關鍵。 *)
Lemma mem_trim_at_pos : forall t i c b,
  In b (trim_at t i c) ->
  (exists a, nth_error t i = Some a /\ b = trim_ev a c) \/ In b t.
Proof.
  intros t. induction t as [|e t' IH]; intros i c b H.
  - cbn [trim_at Nat.eqb] in H. contradiction.
  - cbn [trim_at Nat.eqb] in H.
    destruct i as [|i']; cbn [Nat.eqb] in H.
    + destruct H as [H|H].
      * left. exists e. split; [reflexivity | symmetry; exact H].
      * right. right. exact H.
    + destruct H as [H|H].
      * right. left. exact H.
      * destruct (IH i' c b H) as [[a [Ha Hb]] | Hb].
        -- left. exists a. split. { cbn. exact Ha. } { exact Hb. }
        -- right. right. exact Hb.
Qed.

(** 剪後邊集 ⊆ 剪前邊集(紅邊計數組裝的**原子**事實)。
    證明:剪只把「第 i 位事件」換成 `trim_ev _ c`;任一剪後紅邊,若牽涉該事件則
    用 `red_p_trim_left/right`(剪⇒剪前仍紅)拉回原事件、否則原樣保留;`pair`
    的 id 由 `trim_ev_id` 保持。 *)
Lemma red_edges_aux_trim_subset : forall l i c rt,
  (forall e, nth_error l i = Some e -> c <= iend (ev_it e)) ->
  forall p, In p (red_edges_aux (trim_at l i c) rt) ->
         In p (red_edges_aux l rt).
Proof.
  intros l. induction l as [|a t IH]; intros i c rt Hb p Hp.
  - cbn [trim_at Nat.eqb] in Hp. contradiction.
  - destruct i as [|i']; cbn [trim_at Nat.eqb] in Hp.
    + rewrite red_edges_aux_cons in Hp.
      apply in_app_or in Hp. destruct Hp as [Hfirst | Hrest].
      * apply in_map_iff in Hfirst. destruct Hfirst as [b [Hp_eq Hfb]].
        apply filter_In in Hfb. destruct Hfb as [HbIn Hpred].
        apply in_or_app. left. apply in_map_iff. exists b. split.
        { rewrite trim_ev_id in Hp_eq. exact Hp_eq. }
        { apply filter_In. split.
          { exact HbIn. }
          { apply (red_p_trim_left rt a b c).
            { apply Hb. reflexivity. }
            { exact Hpred. } } }
      * apply in_or_app. right. exact Hrest.
    + rewrite red_edges_aux_cons in Hp.
      apply in_app_or in Hp. destruct Hp as [Hfirst | Hrest].
      * apply in_map_iff in Hfirst. destruct Hfirst as [b [Hp_eq Hfb]].
        apply filter_In in Hfb. destruct Hfb as [HbIn Hpred].
        destruct (mem_trim_at_pos t i' c b HbIn) as [[x [HxPos Hb_eq]] | HbIn2].
        -- subst b.
           assert (Hiend : c <= iend (ev_it x)).
           { apply Hb. cbn. exact HxPos. }
           apply in_or_app. left. apply in_map_iff. exists x. split.
           { rewrite trim_ev_id in Hp_eq. exact Hp_eq. }
           { apply filter_In. split.
             { apply nth_error_In with (n := i'); exact HxPos. }
             { apply (red_p_trim_right rt a x c Hiend). exact Hpred. } }
        -- apply in_or_app. left. apply in_map_iff. exists b. split.
           { exact Hp_eq. }
           { apply filter_In. split; [exact HbIn2 | exact Hpred]. }
      * apply in_or_app. right.
        apply IH with (i := i') (c := c) (rt := rt); [|exact Hrest].
        intros e He. apply Hb. cbn. apply He.
Qed.

(** `i_overlap = false ⇒ red_p = false`(剪後重疊消失 ⇒ 不再是紅邊)。 *)
Lemma red_p_not_overlap : forall rt e b, i_overlap (ev_it e) (ev_it b) = false -> red_p rt e b = false.
Proof.
  intros. unfold red_p. rewrite H. apply Bool.andb_false_iff. right. apply Bool.andb_false_iff. right. reflexivity.
Qed.

(** 剪只改區間右端,`ev_id` 逐位相同 ⇒ `map ev_id (trim_at l p c) = map ev_id l`。 *)
Lemma trim_at_preserves_ids : forall l p c, map ev_id (trim_at l p c) = map ev_id l.
Proof.
  intros l. induction l as [|e t IH]; intros p c.
  - reflexivity.
  - destruct p.
    + cbn [trim_at Nat.eqb]. simpl. reflexivity.
    + cbn [trim_at Nat.eqb]. simpl. rewrite IH. reflexivity.
Qed.

(** 第 p 位事件在剪後正是 `trim_ev _ c`。 *)
Lemma trim_at_nth_here : forall l p c a, nth_error l p = Some a ->
  nth_error (trim_at l p c) p = Some (trim_ev a c).
Proof.
  intros l. induction l as [|e t IH]; intros p c a Hn.
  - destruct p; simpl in Hn; discriminate.
  - destruct p; cbn [trim_at Nat.eqb] in *.
    + cbn in Hn. injection Hn as <-. reflexivity.
    + cbn in Hn. apply IH. exact Hn.
Qed.

(* ct_pred a b0 = true => ev_id b0 <> ev_id a *)
Lemma ct_pred_ne_id : forall a b0, ct_pred a b0 = true -> ev_id b0 <> ev_id a.
Proof.
  intros a b0 H. unfold ct_pred in H. simpl in H.
  apply Bool.andb_true_iff in H. destruct H as [Hne _].
  apply Bool.negb_true_iff in Hne.
  apply Nat.eqb_neq in Hne. exact Hne.
Qed.

(* In b (trim_at l p c) 的「搬移」:b 是 list 中第 p 位以外的元素 ⇒ 剪後仍在。 *)
Lemma in_trim_at_other : forall l p c a b,
  nth_error l p = Some a -> b <> a -> In b l -> In b (trim_at l p c).
Proof.
  intros l. induction l as [|e t IH]; intros p c a b Hpos Hne Hb.
  - destruct p; simpl in Hpos; discriminate.
  - destruct p; cbn [trim_at Nat.eqb] in Hpos, Hb.
    + simpl in Hpos, Hb. injection Hpos as <-.
      destruct Hb as [Heb|Hbt]; [subst b; contradiction | now right].
    + simpl in Hpos, Hb.
      destruct Hb as [Heb|Hbt].
      * left. exact Heb.
      * right. apply (IH p c a b). { exact Hpos. } { exact Hne. } { exact Hbt. }
Qed.

(* 主「丟邊」引理:剪後(t = trim_at l p c)`pk a b0` 不再是紅邊。 *)
Lemma pk_absent_after_trim : forall l p c rt a b0,
  NoDup (map ev_id l) ->
  nth_error l p = Some a ->
  c <= iend (ev_it a) ->
  ct_pred a b0 = true ->
  In b0 l ->
  i_overlap (ev_it (trim_ev a c)) (ev_it b0) = false ->
  ~ In (pk a b0) (red_edges_aux (trim_at l p c) rt).
Proof.
  intros l p c rt a b0 Huniq Hpos Hc Hct Hb0 Hov Hpk.
  assert (Hnab : ev_id a <> ev_id b0).
  { intro Hab. apply (ct_pred_ne_id a b0 Hct). exact (eq_sym Hab). }
  assert (Hnab' : b0 <> a).
  { intro E. subst b0. apply Hnab. reflexivity. }
  assert (Huniq_t : NoDup (map ev_id (trim_at l p c))).
  { rewrite trim_at_preserves_ids. exact Huniq. }
  assert (Hb0_t : In b0 (trim_at l p c)).
  { apply (in_trim_at_other l p c a b0 Hpos Hnab' Hb0). }
  assert (Htrim_in : In (trim_ev a c) (trim_at l p c)).
  { apply nth_error_In with (n := p). apply trim_at_nth_here. exact Hpos. }
  destruct (in_red_char (trim_at l p c) rt (pk a b0) Huniq_t Hpk)
    as [x [y [Hxin [Hyin [Hxy [Hpeq Hred]]]]]].
  assert (Hcomp : (ev_id x = ev_id a \/ ev_id x = ev_id b0) /\
                  (ev_id y = ev_id a \/ ev_id y = ev_id b0)).
  { apply pk_comp_in_ab; [exact Hnab | exact Hpeq]. }
  destruct Hcomp as [[Hxid|Hxid] [Hyid|Hyid]].
  - (* both id_a *)
    assert (Hx : x = trim_ev a c).
    { apply (nodup_map_ev_id_inj (trim_at l p c) x (trim_ev a c) Huniq_t Hxin Htrim_in).
      rewrite trim_ev_id. exact Hxid. }
    assert (Hy : y = trim_ev a c).
    { apply (nodup_map_ev_id_inj (trim_at l p c) y (trim_ev a c) Huniq_t Hyin Htrim_in).
      rewrite trim_ev_id. exact Hyid. }
    subst x y. contradiction.
  - (* x=id_a, y=id_b0 *)
    assert (Hx : x = trim_ev a c).
    { apply (nodup_map_ev_id_inj (trim_at l p c) x (trim_ev a c) Huniq_t Hxin Htrim_in).
      rewrite trim_ev_id. exact Hxid. }
    assert (Hy : y = b0).
    { apply (nodup_map_ev_id_inj (trim_at l p c) y b0 Huniq_t Hyin Hb0_t). exact Hyid. }
    subst y. subst x. rewrite red_p_not_overlap in Hred; [discriminate | exact Hov].
  - (* x=id_b0, y=id_a *)
    assert (Hx : x = b0).
    { apply (nodup_map_ev_id_inj (trim_at l p c) x b0 Huniq_t Hxin Hb0_t). exact Hxid. }
    assert (Hy : y = trim_ev a c).
    { apply (nodup_map_ev_id_inj (trim_at l p c) y (trim_ev a c) Huniq_t Hyin Htrim_in).
      rewrite trim_ev_id. exact Hyid. }
    subst x y. rewrite red_p_sym in Hred. rewrite red_p_not_overlap in Hred; [discriminate | exact Hov].
  - (* both id_b0 *)
    assert (Hx : x = b0).
    { apply (nodup_map_ev_id_inj (trim_at l p c) x b0 Huniq_t Hxin Hb0_t). exact Hxid. }
    assert (Hy : y = b0).
    { apply (nodup_map_ev_id_inj (trim_at l p c) y b0 Huniq_t Hyin Hb0_t). exact Hyid. }
    subst x y. contradiction.
Qed.
(* 無重複 + 子集 + 真少一元素 => 長度嚴格小 *)
Lemma length_lt_nodup : forall l1 l2 : list (nat*nat),
  NoDup l1 -> NoDup l2 -> incl l1 l2 ->
  (exists x, In x l2 /\ ~ In x l1) -> length l1 < length l2.
Proof.
  intros l1 l2 Hn1 Hn2 Hincl [x [Hx2 Hx1]].
  assert (Hle : length l1 <= length l2).
  { apply NoDup_incl_length; [exact Hn1 | exact Hincl]. }
  destruct (Nat.lt_ge_cases (length l1) (length l2)) as [Hlt|Hge].
  - exact Hlt.
  - exfalso.
    assert (Heq : length l1 = length l2) by lia.
    assert (Hincl2 : incl l2 l1).
    { apply NoDup_length_incl; [exact Hn1 | | exact Hincl].
      rewrite Heq. lia. }
    apply Hx1. apply Hincl2. exact Hx2.
Qed.

(* 主「嚴格遞減」:剪第 p 位(事件 a)到 c,drops 候選 b0 的邊 => 紅邊數嚴格減 *)
Lemma r1_apply_red_edges_strict : forall l p c rt a b0,
  NoDup (map ev_id l) ->
  nth_error l p = Some a ->
  c <= iend (ev_it a) ->
  ct_pred a b0 = true ->
  In b0 l ->
  rt_mem rt (Nat.min (ev_id a) (ev_id b0)) (Nat.max (ev_id a) (ev_id b0)) = false ->
  i_overlap (ev_it (trim_ev a c)) (ev_it b0) = false ->
  length (red_edges_aux (trim_at l p c) rt) < length (red_edges_aux l rt).
Proof.
  intros l p c rt a b0 Huniq Hpos Hc Hct Hb0 Hrt Hov.
  assert (Hnab' : b0 <> a).
  { intro E. subst b0.
    assert (Hx : ev_id a <> ev_id a) by (apply ct_pred_ne_id with (a:=a)(b0:=a); exact Hct). exact (Hx eq_refl). }
  assert (Hat : a <> b0).
  { intro E. apply Hnab'. exact (eq_sym E). }
  assert (Htr : red_p rt a b0 = true).
  { apply (red_p_rt_ct_pred rt a b0 Hct). exact Hrt. }
  assert (Hsub : incl (red_edges_aux (trim_at l p c) rt) (red_edges_aux l rt)).
  { intro p'. apply (red_edges_aux_trim_subset l p c rt).
    intros e He. assert (e = a) by congruence. subst e. exact Hc. }
  assert (Hnodup : NoDup (red_edges_aux l rt)).
  { apply (red_edges_aux_nodup l rt Huniq). }
  assert (Hnodup_t : NoDup (red_edges_aux (trim_at l p c) rt)).
  { apply (red_edges_aux_nodup (trim_at l p c) rt).
    rewrite trim_at_preserves_ids. exact Huniq. }
  assert (Hpresent : In (pk a b0) (red_edges_aux l rt)).
  { apply (in_red_edges_aux l rt a b0).
    - apply nth_error_In with (n := p). exact Hpos.
    - exact Hb0.
    - exact Hat.
    - exact Htr. }
  assert (Habsent : ~ In (pk a b0) (red_edges_aux (trim_at l p c) rt)).
  { apply (pk_absent_after_trim l p c rt a b0 Huniq Hpos Hc Hct Hb0 Hov). }
  apply (length_lt_nodup (red_edges_aux (trim_at l p c) rt) (red_edges_aux l rt)
         Hnodup_t Hnodup Hsub (ex_intro _ (pk a b0) (conj Hpresent Habsent))).
Qed.


(* --------------------------------------------------------------------- *)
(* ⑥ 接橋:前綴分解給出位置與命中事件                                       *)
(* --------------------------------------------------------------------- *)

(* 在「該 id 在 pre 中不再出現」的前提下,pos_of 命中第 length pre 位。 *)
Lemma pos_of_mid : forall (pre post : list Ev) (a : Ev),
  (forall e, In e pre -> ev_id e <> ev_id a) ->
  pos_of (pre ++ a :: post) (ev_id a) = Some (length pre).
Proof.
  intros pre. induction pre as [|e pre' IH]; intros post a Hne.
  - simpl. rewrite Nat.eqb_refl. reflexivity.
  - simpl. destruct (Nat.eqb (ev_id e) (ev_id a)) eqn:E.
    + apply Nat.eqb_eq in E. exfalso. specialize (Hne e (or_introl eq_refl)). exact (Hne E).
    + rewrite (IH post a). { reflexivity. } { intros x Hx. apply Hne. right. exact Hx. }
Qed.

(* 第 length pre 位正是 a(前綴分解式)。 *)
Lemma nth_error_mid : forall (pre : list Ev) (a : Ev) (post : list Ev),
  nth_error (pre ++ a :: post) (length pre) = Some a.
Proof.
  intros pre. induction pre as [|e pre' IH]; intros a post.
  - simpl. reflexivity.
  - simpl. exact (IH a post).
Qed.
