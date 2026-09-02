(* ===================================================================== *)
(* cl0r0 內核鏡像 —— §4 重寫系統(rep.rs)在 Rocq 中的機械鏡像。
 *
 * Phase 0(P0-b 內核抽取)。鏡像決策見 docs/ROCQ-TRACE.md §二:
 *   D1  u32 → nat;半開區間原樣保留(相接不算重疊)。
 *   D2  AState.log 剔除(Rust 註明「不參與相等性」,診斷用途)。
 *   D3  µ 的第二分量 |Err_rustc| := 0(§6.3 oracle 範疇,判定權不轉移)。
 *   D4  顯示標簽(label/format)屬診斷輸出,不入鏡像。
 *   D5  id 由構造補齊(鏡像中 = 位置索引);Rust 由 AState::new 補齊。
 *   D6  枚舉順序:Rust 以 BTreeSet 排序去重,鏡像以列表順序 —— 只比計數。
 * 完整性:apply(四條規則)/ applicable(兩菜單 × 兩政策)/ 紅邊 / 測度 /
 * 狀態枚舉全部鏡像;語義與 rep.rs 逐條對照(見 ROCQ-TRACE.md §二)。
 * ===================================================================== *)

From Coq Require Import List Arith Bool Lia.
Import ListNotations.

Set Implicit Arguments.

(* --------------------------------------------------------------------- *)
(* §4.1 原子類型:訪問種類 / 半開區間 / 事件 / 狀態                        *)
(* --------------------------------------------------------------------- *)

Inductive K : Set :=
| Mut                       (* &mut / write *)
| Sh.                       (* &   / read  *)

(* Rust: a == Mut || b == Mut *)
Definition k_conflict (a b : K) : bool :=
  match a with
  | Mut => true
  | Sh   => match b with Mut => true | Sh => false end
  end.

Lemma k_conflict_sym : forall a b, k_conflict a b = k_conflict b a.
Proof. destruct a, b; reflexivity. Qed.

Record Interval : Set :=
  { istart : nat                            (* 左端點(含) *)
  ; iend   : nat }.                         (* 右端點(不含) *)

(* Rust: self.start < o.end && o.start < self.end *)
Definition i_overlap (a b : Interval) : bool :=
  andb (Nat.ltb (istart a) (iend b)) (Nat.ltb (istart b) (iend a)).

Lemma i_overlap_sym : forall a b, i_overlap a b = i_overlap b a.
Proof. intros [s1 e1] [s2 e2]. unfold i_overlap. simpl.
       apply andb_comm. Qed.

Record Ev : Set :=
  { ev_id      : nat
  ; ev_storage : nat
  ; ev_kind    : K
  ; ev_it      : Interval }.

Record AState : Set :=
  { st_evs     : list Ev
  ; st_runtime : list (nat * nat) }.

(* --------------------------------------------------------------------- *)
(* 紅邊集合 / 測度(§4.1)                                                 *)
(* --------------------------------------------------------------------- *)

Definition rt_mem (rt : list (nat * nat)) (x y : nat) : bool :=
  existsb (fun p => andb (Nat.eqb (fst p) x) (Nat.eqb (snd p) y)) rt.

(* Rust 迴圈序:for i in 0..len, for j in i+1..len;
 * 邊輸出 (min id, max id);runtime 命中則跳過;storage 不同跳過。 *)
Fixpoint red_edges_aux (l : list Ev) (rt : list (nat * nat)) :
    list (nat * nat) :=
  match l with
  | [] => []
  | a :: l' =>
      let p := fun b =>
        andb (Nat.eqb (ev_storage b) (ev_storage a))
          (andb (k_conflict (ev_kind a) (ev_kind b))
            (andb (i_overlap (ev_it a) (ev_it b))
              (negb (rt_mem rt (Nat.min (ev_id a) (ev_id b))
                                 (Nat.max (ev_id a) (ev_id b)))))) in
      map (fun b => (Nat.min (ev_id a) (ev_id b),
                     Nat.max (ev_id a) (ev_id b))) (filter p l')
        ++ red_edges_aux l' rt
  end.

Definition red_edges (s : AState) : list (nat * nat) :=
  red_edges_aux (st_evs s) (st_runtime s).

(* µ = (|E_red|, |Err_rustc|) —— 本演示 Err_rustc := 0(D3) *)
Definition measure (s : AState) : nat * nat := (length (red_edges s), 0).

Definition sd (a b : nat * nat) : bool :=
  orb (Nat.ltb (fst a) (fst b))
      (andb (Nat.eqb (fst a) (fst b)) (Nat.ltb (snd a) (snd b))).

Definition is_nf (s : AState) : bool :=
  Nat.eqb (length (red_edges s)) 0.

(* --------------------------------------------------------------------- *)
(* 菜單規則(§4.1)                                                        *)
(* --------------------------------------------------------------------- *)

Inductive Rule : Set :=
| R1Shorten (r_id r_cut : nat)     (* 縮短右端點 *)
| R2Split   (r_id r_m : nat)       (* 切點分裂遷移 storage *)
| R3Swap    (r_a r_b : nat)        (* 交換兩事件區間 *)
| R4Runtime (r_a r_b : nat).       (* 標記運行期借用 *)

Inductive Policy : Set := Guarded | Raw.
Inductive Menu : Set := CommutativeTrim | Naive.

(* --------------------------------------------------------------------- *)
(* apply —— 四條規則(Rust rep.rs apply,逐條對照;log 剔除 D2)             *)
(* --------------------------------------------------------------------- *)

Fixpoint find_ev (l : list Ev) (i : nat) : option Ev :=
  match l with
  | [] => None
  | e :: l' => if Nat.eqb (ev_id e) i then Some e else find_ev l' i
  end.

(* R1:找第一個 id 匹配的事件,切斷其右端點(須嚴格落在區間內)。 *)
Fixpoint r1_apply (l : list Ev) (i cut : nat) : option (list Ev) :=
  match l with
  | [] => None
  | e :: l' =>
      if Nat.eqb (ev_id e) i
      then Some ({| ev_id := ev_id e; ev_storage := ev_storage e;
                    ev_kind := ev_kind e;
                    ev_it := {| istart := istart (ev_it e); iend := cut |} |}
                 :: l')
      else option_map (fun l2 => e :: l2) (r1_apply l' i cut)
  end.

Definition max_storage (l : list Ev) : nat :=
  fold_left (fun acc e => Nat.max acc (ev_storage e)) l 0.

(* R2:複製語義 —— 原 storage 中「切點之後」(start >= m)者遷移 new。 *)
Definition r2_remap (l : list Ev) (old new m : nat) : list Ev :=
  map (fun e =>
    if andb (Nat.eqb (ev_storage e) old) (Nat.leb m (istart (ev_it e)))
    then {| ev_id := ev_id e; ev_storage := new;
            ev_kind := ev_kind e; ev_it := ev_it e |}
    else e) l.

Definition r2_apply (s : AState) (i m : nat) : option AState :=
  match find_ev (st_evs s) i with
  | None => None
  | Some e =>
      if orb (Nat.leb m (istart (ev_it e))) (Nat.leb (iend (ev_it e)) m)
      then None                              (* m 不在 (start, end) 內 *)
      else Some {| st_evs := r2_remap (st_evs s) (ev_storage e)
                                    (S (max_storage (st_evs s))) m;
                   st_runtime := st_runtime s |}   (* max+1(Rust unwrap_or(0)+1) *)
  end.

Fixpoint pos_of (l : list Ev) (i : nat) : option nat :=
  match l with
  | [] => None
  | e :: l' => if Nat.eqb (ev_id e) i then Some 0 else option_map S (pos_of l' i)
  end.

(* R3:交換「位置 ia 與 ib」的區間(Rust:按第一個出現位置交換;兩者不同)。 *)
Definition swap_at (e : Ev) (n ia ib : nat) (ta tb : Interval) : Ev :=
  if Nat.eqb n ia then {| ev_id := ev_id e; ev_storage := ev_storage e;
                          ev_kind := ev_kind e; ev_it := tb |}
  else if Nat.eqb n ib then {| ev_id := ev_id e; ev_storage := ev_storage e;
                               ev_kind := ev_kind e; ev_it := ta |}
  else e.

Fixpoint r3_swap (l : list Ev) (n ia ib : nat) (ta tb : Interval) : list Ev :=
  match l with
  | [] => []
  | e :: l' => swap_at e n ia ib ta tb :: r3_swap l' (S n) ia ib ta tb
  end.

Definition r3_apply (s : AState) (a b : nat) : option AState :=
  match pos_of (st_evs s) a, pos_of (st_evs s) b with
  | Some ia, Some ib =>
      if Nat.eqb ia ib
      then None
      else match nth_error (st_evs s) ia, nth_error (st_evs s) ib with
           | Some ea, Some eb =>
               Some {| st_evs := r3_swap (st_evs s) 0 ia ib (ev_it ea) (ev_it eb);
                       st_runtime := st_runtime s |}
           | _, _ => None
           end
  | _, _ => None
  end.

(* R4:紅邊標記為運行期借用(runtime 順序:append 於尾部)。 *)
Definition r4_apply (s : AState) (x y : nat) : option AState :=
  let x' := Nat.min x y in
  let y' := Nat.max x y in
  if rt_mem (red_edges s) x' y'
  then Some {| st_evs := st_evs s;
               st_runtime := st_runtime s ++ [(x', y')] |}   (* Rust push=尾插 *)
  else None.

Definition apply_rule (s : AState) (r : Rule) : option AState :=
  match r with
  | R1Shorten i c =>
      match r1_apply (st_evs s) i c with
      | Some evs' => Some {| st_evs := evs'; st_runtime := st_runtime s |}
      | None => None
      end
  | R2Split i m => r2_apply s i m
  | R3Swap a b => r3_apply s a b
  | R4Runtime a b => r4_apply s a b
  end.

(* --------------------------------------------------------------------- *)
(* applicable —— 兩菜單 × 兩政策                                          *)
(* --------------------------------------------------------------------- *)

(* CommutativeTrim:對每個事件 a,cut = 所有「起點更晚」衝突 b 的最小 start。 *)
Definition cut_for (evs : list Ev) (a : Ev) : option nat :=
  let cands := filter (fun b =>
      andb (negb (Nat.eqb (ev_id b) (ev_id a)))
        (andb (Nat.eqb (ev_storage b) (ev_storage a))
          (andb (k_conflict (ev_kind a) (ev_kind b))
            (andb (Nat.ltb (istart (ev_it a)) (istart (ev_it b)))
                  (i_overlap (ev_it a) (ev_it b)))))) evs
  in fold_left (fun acc b =>
         match acc with
         | None => Some (istart (ev_it b))
         | Some c => Some (Nat.min c (istart (ev_it b)))
         end) cands None.

Fixpoint ct_applicable (l : list Ev) : list Rule :=
  match l with
  | [] => []
  | a :: l' =>
      (match cut_for l a with
       | Some c => [R1Shorten (ev_id a) c]
       | None => []
       end) ++ ct_applicable l'
  end.

(* [a, b) 的所有整數切點(Rust (it.start+1)..it.end 半開)。 *)
Definition range (k : nat) : list nat :=
  let fix go (k : nat) :=
    match k with O => [] | S k' => 0 :: map S (go k') end
  in go k.

Definition cuts_between (a b : nat) : list nat :=
  map (fun d => a + d) (range (b - a)).

Fixpoint r1_naive (l : list Ev) : list Rule :=
  match l with
  | [] => []
  | e :: l' =>
      map (fun c => R1Shorten (ev_id e) c)
          (cuts_between (S (istart (ev_it e))) (iend (ev_it e)))
        ++ r1_naive l'
  end.

Fixpoint r2_naive (l : list Ev) : list Rule :=
  match l with
  | [] => []
  | e :: l' =>
      map (fun m => R2Split (ev_id e) m)
          (cuts_between (S (istart (ev_it e))) (iend (ev_it e)))
        ++ r2_naive l'
  end.

Fixpoint r3_naive (l : list Ev) : list Rule :=
  match l with
  | [] => []
  | a :: l' => map (fun b => R3Swap (ev_id a) (ev_id b)) l' ++ r3_naive l'
  end.

Definition r4_naive (s : AState) : list Rule :=
  map (fun p => R4Runtime (fst p) (snd p)) (red_edges s).

Definition applicable (s : AState) (m : Menu) (p : Policy) : list Rule :=
  let base := match m with
              | CommutativeTrim => ct_applicable (st_evs s)
              | Naive =>
                  r1_naive (st_evs s) ++ r2_naive (st_evs s)
                    ++ r3_naive (st_evs s) ++ r4_naive s
              end in
  match p with
  | Raw => base
  | Guarded =>
      filter (fun r => match apply_rule s r with
                       | Some s2 => sd (measure s2) (measure s)
                       | None => false
                       end) base
  end.

(* --------------------------------------------------------------------- *)
(* 狀態枚舉(rep.rs enumerate_states;D6:對帳只比計數)                      *)
(* --------------------------------------------------------------------- *)

(* 起點 s 的所有區間 [s, e),e = s+1..m(共 m - s 個,若 s < m)。 *)
Definition ends_for (m s : nat) : list Interval :=
  map (fun d => {| istart := s; iend := s + S d |}) (range (m - s)).

(* 全部起點(0..m-1)的區間。 *)
Definition starts_for (m : nat) : list Interval :=
  flat_map (fun s => ends_for m s) (range m).

Definition all_intervals (m : nat) : list (K * Interval) :=
  map (fun iv => (Mut, iv)) (starts_for m)
    ++ map (fun iv => (Sh, iv)) (starts_for m).

(* 笛卡兒積:每事件選一個 (kind, interval),順序 = per[0] 為主。 *)
Fixpoint product (l : list (list (K * Interval))) : list (list (K * Interval)) :=
  match l with
  | [] => [[]]
  | per :: l' =>
      flat_map (fun e => map (fun xs => e :: xs) (product l')) per
  end.

Fixpoint build_evs (pos : nat) (l : list (K * Interval)) : list Ev :=
  match l with
  | [] => []
  | (k, iv) :: l' =>
      {| ev_id := pos; ev_storage := 0; ev_kind := k; ev_it := iv |}
        :: build_evs (S pos) l'
  end.

Definition mk_state (combo : list (K * Interval)) : AState :=
  {| st_evs := build_evs 0 combo; st_runtime := [] |}.

Definition enumerate_states (n m : nat) : list AState :=
  map mk_state (product (repeat (all_intervals m) n)).

Definition count_states (n m : nat) : nat :=
  length (enumerate_states n m).

(* 計數契約(經驗公式,由對帳驗證;數學證明留待 Phase 2):
 *   每事件候選數 = 2 × m(m+1)/2 = m(m+1) ⇒ 狀態數 = (m(m+1))^n。
 * 參見 docs/ROCQ-TRACE.md §三 對帳矩陣。 *)
