# R3(具體 WCR)開工前的引理／演算法／工具搜查報告 + 規劃

> 2026-09-02。主計劃 `docs/ROCQ-PLAN.md`,對帳 `docs/ROCQ-TRACE.md`,
> 難點與克服策略 `docs/HARD-ITEMS.md`。
> 本檔 = 應「搜查引理相關論文、演算法、工具,先規劃後續工作」之要求所產出的
> **聯網文獻比對 + 沙箱實測前測(probe)**,兩者可交叉複現。

---

## 〇、一句話結論

**Phase 3(R3 具體 WCR)可以開工,而且比 ROCQ-PLAN 原先預估的「中高風險」低一档:**
沙箱實測顯示 CT 菜單滿足**精確交換引理**(不只 joinable),且**Guarded 側條件在 CT 上是冗餘的**
(與 Raw 給出同一規則集)—— 這把 R3 從「要靠測度運算」降為「一條可歸納的組合引理」。

---

## 一、引理文獻(本輪新增/複核,含用途裁定)

| 文獻 | 內容 | 對本專案的用途(裁定) |
|---|---|---|
| Newman, *Annals of Math.* 35 (1942) 223–243 | 引理原件:SN + WCR ⇒ CR | 已形式化(R1)。**保留為主敘述** |
| Huet, *JACM* 27(4) (1980) 797–821 | strip 引理 + 良基歸納的證明策略 | R1 的證明結構即採此(Huet 式),已入庫 |
| **van Oostrom, *Confluence by decreasing diagrams*, TCS 126(2) (1994) 259–280**;及 *Converted*, RTA'08 | 完備的合流判準:標號 + 局部菱形遞減 ⇒ 合流;conversion 版本比 valley 版本更好用 | **備援路線(降風險)**:若 R3 的交換引理在一般狀態空間卡住,改用「1 個標號(規則 id)的遞減圖」陳述,可處理非交換情形。Isabelle 已形式化(Zankl, RTA'13 / CoRR abs/1210.1100),Rocq 側需自寫,但其「局部 ⇒ 全域」的歸納骨架與我們 R1 相同 |
| **Sternagel & Thiemann, *A New and Formalized Proof of Abstract Completion*(RV 2015;JCIS/LMCS 版 2019)** | 用 **peak decreasingness** 取代 Newman 引理證明 abstract completion 正確性,並給出 critical pair 判準的嵌入方式;全文 Isabelle 形式化 | **直接可用的證明縮寫**:我們的場景(先有 SN,再證局部)可用 peak-decreasingness 陳述「每個局部 peak 可用標籤更小的步封頂」—— 在 Rocq 中寫成一條 `peak_decreasing` 定義 + 一條主引理,規模比通用 decreasing diagrams 小得多 |
| Gramlich, PhD 1996 (TU Wien) | SN ⇒ [CR ⇔ 臨界對可回合] | 已在 ROCQ-PLAN;R3 的「臨界對」語言由它正名 |
| **ACL2:Newman's lemma + Knuth–Bendix critical pair theorem 的形式化**(Ruiz-Reina et al., 2002 系列) | 以「abstract proofs as objects」的風格在 ACL2 證 Newman | 方法論對照:**把歸約序列當成可變換的物件**,正是我們對帳(鏡像 vs 實作)的哲學;不採用 ACL2 工具 |
| CoLoR,Blanqui & Koprowski,MSCS 21(4) (2011) 827–859;**現為 Rocq 套件 `rocq-color`(opam,rocq-released 倉)** | 關係/良基/終止/多項式解釋/臨界對/認證終止檢核 | **只借定義層**(SN、字典序、多集合序);不引整個庫:會把 CI 從「apt 裝 coq」升級成「opam switch」,見 §四 |
| IsaFoR / **CeTA**(Sternagel & Thiemann) |  Isabelle 內的 ARS 庫 + **把終止/合流證明提取成可執行認證檢查器** | 工作流藍本:ROCQ-PLAN §4.1 的「反射式檢查器 + 證書」就是 CeTA 形態;本專案已有一比一縮小版(`tools/rocq_reconcile.py`) |
| CSI / NCi / 併發合流競賽 CoCo(Strobel, Zankl, Middeldorp, *JAR* 2021) | 自動合流判定的**反例生成**、CNI、peak 不可滿足性判定 | 用作**測試對照**:把 CT 菜單寫成 `.trsrc` 丟給 CSI/CoCo 判,看自動工具怎麼證/怎麼找反例,可校準我們 Rocq 證明的陳述是否過強 |
| Singh & Natarajan 2019(Coq 弱完美圖定理) | 大定理先例 | **仍不需要**(R6 走區間組合路線) |
| 錯誤回復文獻:de Jonge et al., SLE'09(生成式解析器的自然錯誤回復);Medeiros & Mascarenhas,JVLC 2019(PEG labeled failures + **翻譯正確性證明**) | 與 L7/L7b 同類:「全化 + 極大錯誤跨度」的既有形式化先例 | R5(L7b)的**陳述對照**:Medeiros 那篇證的是「翻譯語義保持」,不是「淨化終止」—— 說明我們的 R5 敘述是新命題,不用擔心重覆;可借它的「farthest-failure / 同步點」措辭 |

---

## 二、演算法面(可執行側:本輪實測前測)

### 2.1 探針設計

兩個獨立小工具(零依賴、只用 `cl0r0::rep` 的公開 API,放在 `examples/`):

* `r3_probe.rs` —— P1 start 不變性、P2 深度 D 菱形(先試**精確交換**便宜路徑,不匹配時才跑深度搜索)、P3 Guarded≡Raw、多正規形;可選 `--nofilter` 關閉 CI 那條 distinct-start 過濾。
* `r3_swap.rs` —— 不做搜索,直接驗 **a --rb--> c ≡ b --ra--> c′(同一狀態)**,並統計紅邊是否曾增加。

### 2.2 實測數字(沙箱 2 核,`cargo run --release`,本輪全部跑過)

| 宇宙 | 狀態數 | 單步 | 不同後繼之臨界對 | 探針結果 |
|---|---|---|---|---|
| 3×6 + distinct-start 過濾(=CI 宇宙族) | 35,280 | — | 10,668 | 精確交換 **10,668/10,668** |
| 4×5 + 過濾 | 105,216 | — | 100,392 | 精確交換 **100,392/100,392** |
| 4×6 + 過濾(**CI 現有規模**) | 623,616 | 1,053,672 | 635,424 | P1 違反 0 / P2 違反 0 / Guarded≡Raw / 多正規形 0 |
| 4×6 **不加過濾**(2.98× 大) | 3,111,696 | 4,550,076 | 2,443,506 | 精確交換 **2,443,506/2,443,506**,紅邊增加 0 |
| 3×7 不加過濾 | 175,616 | 159,348 | 41,664 | 全綠 |

三個**新的結構性事實**(先前文件沒有記錄):

1. **精確交換成立**:對 CT 的任意兩條不同規則 `ra, rb`,`s→ra a→rb c` 與 `s→rb b→ra c′` 落在**同一狀態**
   (不只是「可回合」)。⇒ Rocq 側 R3 的主引理可以寫成等式級,不需搜索深度。
2. **Guarded 側條件在 CT 上恆真**:`applicable s CT Guarded == applicable s CT Raw`(所有宇宙、0 例外)。
   ⇒ µ 遞減不是 CT 的*假設*而是*定理*;R2 因此可從「filter 的內涵」升級為「紅邊計數在修剪下嚴格遞減」的組合證明。
   (註:這也意味著 ROCQ-PLAN §三-R2 中「語義零遷移」的說法可以更進一步——**guard 可省**。)
3. **紅邊數單調不增**(任何 CT 步驟,包括 Raw 集合):修剪只縮 end、不動 start ⇒ 候選集合只縮不長。
   ⇒ 交換引理的證明骨架即「`cut` 只讀別人的 **start**,而 start 是全局不變量」。

### 2.3 由前測直接產出的新具名測試(已入庫)

`tests/laws.rs::test_law_L9b_parallel_moves_exact_swap`(3×6 + 4×5 全宇宙、**不**加 distinct-start 過濾):
同時斷言 ①guard 冗餘 ②紅邊單調 ③start 不變 ④精確交換。
→ 這是 Rocq `ct_join_exact` 的**鏡面測試**:Rust 端跑窮舉,Rocq 端跑∀,兩者由 SPEC-TRACE 锚住。

### 2.4 一個如實修正:distinct-start 過濾並非定理所必需

`newman_check` 把宇宙過濾成「起點兩兩不同」(`src/l9newman.rs`)。本輪在**未過濾**的
3,111,696 狀態宇宙上,精確交換仍全綠。⇒ 該過濾是**規模控制**(狀態數 623k → 3.1M),
不是定理前提;但**目前** Rocq 鏡像 `enumerate_states` 不做此過濾,而 `newman_check` 做 ——
對帳時兩邊計數不同,這是 §四 的待辦(把「過濾只影響樣本、不影響定理」寫進 ROCQ-TRACE 的鏡像決策)。

---

## 三、工具面(选型與 CI 實況)

| 工具 | 现状/可選 | 裁定 |
|---|---|---|
| Rocq 本體 | 沙箱與 CI 目前用 **Debian trixie 的 coq 8.20.1**;官方 9.2(2026-03-27)已在 `Stdlib.*` 命名空間下,MathComp 改名 `rocq-mathcomp-*` | 敘述不變;CI 加一輪 `rocq` 鏡像矩陣(8.20 / 9.2)以防命名空間斷裂 |
| `docker-coq-action`(rocq-community,v1.5.2) | 用 `rocq/rocq-prover:9.2`(≥9.0)或 `coqorg/coq:8.20` | **採納**:比 `apt-get install coq` 新且可鎖版;`rocq` job 換成法即可(見 HARD-ITEMS #4) |
| `rocq-color`(CoLoR) | `opam repo add rocq-released …; opam install rocq-color` | **暫不引入**:會把 CI 變成 opam switch(建置時間、鏡像依賴);R1/R2 用不到它的術語層。等到需要「多項式解釋/依賴對」自動終止時(如 L7b 規則擴充)再評估 |
| `vm_compute` / `native_compute` / 提取到 OCaml | 現有對帳用 `vm_compute`(19 樣本點 kernel 複驗) | 保留 `vm_compute`;反射式證書(Phase 6)若走全量 4×6,**必須**先用 `native_compute` 計時,否則 kernel 檢查會爆 |
| CeTA / CSI / AProVE / CoCo | 外部自動判定的對照器 | **採納為測試對照**(不進 CI):把 CT 菜單寫成 COPS/`csi` 可讀格式,交叉看自動工具能否復現「合流」與 Naive 的「反例」;若工具給出更強的證法,抄它的標號 |
| LLM 輔助證明(Rocq 側 `CoqGym/lean-retrieval` 系、`roq`/Lia-auto) | 可加速策略搜尋 | **可選**:僅用於搜尋 tactic,產物一律由 kernel 驗;不得以「模型說可以」作為完成線 |
| 本地工具鏈重建 | `scripts/setup_dev.sh` 已可重建(`coq` + `libcoq-mathcomp-ssreflect` + rustup) | 本輪實測:**可用**;但注意脚本用 `sudo` 跑時 rustup 會裝到 `/root/.cargo`(下輪修成用戶級) |

---

## 四、後續工作規劃(修訂版;取代 ROCQ-PLAN §六 的 Phase 3 估時)

```
Iteration 4(Rocq Phase 3)—— 目標:R3 具體 WCR ⇒ R4 具體 CR 定理
 0. ★ 前置(2026-09-03 完成):`cut_for_gt_start` 封閉(WCRUtil ③,
    fold_min_mem 以「累加器 generalization」解掉 fold_left+cbn 死結)+
    `ConcreteWCR.v` 不變量機器(ct_applicable_spec / ct_step_start_invariant /
    ct_step_preserves_uniq / ct_step_preserves_wf;9 定理 Print Assumptions 全潔淨)+
    外部交叉驗證上線(docs/EXTERNAL-XCHECK.md:Maude 全宇宙五性質綠、
    倒掛宇宙 P2 綠、NaTT R1 超集終止 YES)。
 1. 鏡像收斂:把 §2.4 的「過濾僅限樣本」寫進 ROCQ-TRACE;鏡像補一條
    `ct_applicable_guard_redundant` 引理(Guarded = Raw on CT)。            [1 人日]
 2. ✅(2026-09-03)`ct_step_start_invariant` 已證 —— 見 ConcreteWCR.v ④。
 3. ✅(2026-09-03 第四輪)候選集單調的落地形式與原計劃不同:原敘述
    「filtered-list 逐字不變」經證實**數學為假**,改以 istart 值列層
    (`map_starts_filter_r1` + `fold_min_via_map`)刻畫「他人 cut 不變」
    (`cut_for_trim_other`)。紅邊遞減計量未建——Raw 路線不需要它。
 4. ✅(2026-09-03 第四輪)交換引理落地:apply 層 `r1_apply2_comm` +
    `apply_r1_comm_ev`,規則層 `ct_rule_survives`(uniq 不可省)。
 5. ✅/⬜ R3 = **`R3_ct_wcr_raw` 已證**(uniq_ids 前提,wf 不需要;
   Print Assumptions 潔淨)→ ⬜ Guarded 版 WCR、R4 = confluent
   (newman + R2 + R3)、唯一正規形定理仍待接出。
    同輪把 4×6 窮舉改成「同一結論」的對帳行,SPEC-TRACE 加列。                [2 人日]
 6. 備援(若 3/4 卡 > 4 人日):改寫成 peak-decreasingness(1 標籤)或
    「有限空間反射證書」;兩者都有先例(§一),且都在 Rocq 內可驗。            [3 人日]
 7. 工程:CI 的 rocq job 改 docker-coq-action(8.20 + 9.2 矩陣);bench 门改法
    (见 HARD-ITEMS #4/#5)。                                                  [1 人日]
 之後:Phase 4(R5 L7b 終止)、Phase 5(R6 T2 χ=ω)、Phase 6(證書 + R7 反例)。
```

預估:**8–16 人日**(ROCQ-PLAN 原估 2–4 人週;前測把不確定性消掉了一半)。

---

## 五、本檔所有數字的複現方式

```sh
sh scripts/setup_dev.sh                       # coq + mathcomp + rust
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --all                               # 50 具名測試(含本輪新增的 L9b′)
make -C rocq && make -C rocq reconcile         # 鏡像 + 19 樣本點 kernel 複驗
cargo build --release --examples
./target/release/examples/r3_probe  4 6 4 --nofilter
./target/release/examples/r3_swap   4 6 4 --nofilter   # 精確交換:2,443,506/2,443,506
```
