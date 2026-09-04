# 外部工具交叉驗證(External Cross-Check)— 2026-09-03

> 對應 `docs/R3-RESEARCH.md` §三 的既定裁決:外部自動判定工具「**採納為測試對照(不進 CI)**」。
> 本輪把該裁決落實為兩套可執行檢查。一鍵複現:`sh tools/run_external_checks.sh`。

## 一、為什麼需要第三、第四次實作

本專案的核心宣稱(R3 前測的「精確交換」「Guarded≡Raw」)此前由**同一團隊哲學**的兩份
實作支撐:Rust 探針(`r3_probe.rs`/`r3_swap.rs`/`r3_wf.rs`,窮舉至 3,111,696 狀態)與 Rocq
鏡像(`vm_compute` 複驗 19 樣本點)。兩者同源 ⇒「沉默漂移」風險對兩者**同時**成立的可能
不能排除(ROCQ-TRACE §誠實申報 的精神)。Maude 模型是**第三份**獨立語義實作(重寫邏輯,
方程語義);NaTT 是**外部競賽級工具**(終止側)。它們的共識是對 Rust⇔Rocq 對帳的獨立校準。

## 二、Maude(`tools/ct_maude.maude`)

* 環境:Debian trixie `apt maude` 3.4;純功能模塊(全部命令式方程,**不用** Maude 的
  `rl`/search —— CT 菜單是確定性函數,等式語義即可,杜絕「模型悄悄超出被測系統」)。
* 鏡像範圍:`ct_pred` / `cutfor` / `menu`(= ct_applicable)/ `r1app`(= r1_apply)/
  `redct`(紅邊)/ Guarded 過濾 / `enumerate_states` 生成式,均逐行對應 Rust/Rocq;
  差異點(右 fold-min)在檔頭註記並由 Coq `fold_min_mem` 保證結果一致。
* 每狀態檢查 `stateOK` = P1(cut > istart,Coq `cut_for_gt_start` 的資料版)∧
  P2(**等式級精確交換**菱形)∧ P3(Guarded 菜單 ≡ Raw 菜單,逐序相等)∧
  P4(istart 向量逐字不變)∧ P5(wf 在一步前後成立)。

| 宇宙(良構) | 狀態數 | 結果 |
|---|---|---|
| 3×3 | 1,728(對帳:Rust reconcile count(3,3)=1728 ✅) | allOK **true** |
| 3×4 | 8,000(對帳:count(3,4)=8000 ✅) | allOK **true** |
| 3×5 | 27,000(對帳:Rust r3_wf 同口徑 27,000 ✅) | allOK **true** |

| 倒掛宇宙(end 任取 0..m,r3_wf.rs 第二節同款) | 狀態數 | 只驗 P2 | 結果 |
|---|---|---|---|
| 3×3 | 32,768(對帳:Rust 32,768 ✅) | allDia | **true**(同 Rust:0 個非精確交換) |

**結論**:Rust 探針的四個關鍵觀測(含「倒掛不破壞精確交換」這條反直覺結果)
在獨立語義框架下**全部重現**;「倒掛不能當定理前提」的口徑不變(見 ROCQ-TRACE §一-R3)。

## 三、NaTT(`tools/natt/r1_sup.xtc`)

* 環境:NaTT 2.3(OCaml 源碼,沙箱現編;後端 z3)。
* 被證系統是**誠實標註的過度近似超集**:
  `ev(I,S,K,i(X,s(E))) → ev(I,S,K,i(X,E))` —— 任何事件、任意語境(cons 列表內部自動
  可達,TRS 語義內建)、右端點嚴格減一。R1 真實一步(剪到 `cut ≥ start+1`)是
  「減一」的有限複合 ⇒ 超集終止 ⊇ 真系統 R1 側終止。
* 結果:**YES**(0.02s;權重 `ev → x1+x2+x3+2·x4`,`s → 1/4 + x`)。
* 口徑:這是 R2(紅邊測度 µ 的精細 SN,`ConcreteSN.R2_sn_step_ct`)的**粗外證**,
  不依賴 `cut_for` 的精細性;NaTT 不管合流——合流屬 Rocq 主線(R3/R4)。

### 重現備忘(踩坑記錄,省後人時間)

1. NaTT 只有 OCaml 源碼;Debian 可直接裝齊依賴:
   `apt install ocaml-nox ocaml-findlib libocamlgraph-ocaml-dev libre-ocaml-dev libxml-light-ocaml-dev z3`。
2. **NaTT 2.3 的大小寫 bug**:`Makefile` 引用 `myXML.ml`,源檔為 `MyXML.ml`
   (上游於大小寫不敏感檔案系統開發);`sed -i 's/myXML/MyXML/' Makefile` 後 `make` 即過。
3. NaTT 的 XTC 讀取器要求 `<rules>` 在前、`<signature>` 在後(順序固定)。
4. `www.trs.css.i.nagoya-u.ac.jp` 的 TLS 鏈不完整(只送葉憑證,Let's Encrypt YE2 鏈);
   由葉憑證 AIA(`http://ye2.i.lencr.org/`)取回中繼憑證併入信任庫即可**不繞過驗證**下載。
5. NaTT 版本自述檔與二進位不一致(README 稱 2.3,`--help` 自報 2.2)——如實記於此。

## 四、狀態對帳(2026-09-03)

| 層級 | 宣稱 | 證據 | 狀態 |
|---|---|---|---|
| 定理 | `cut_for_gt_start` | `WCRUtil.v` ③,`Print Assumptions` = Closed | ✅ kernel |
| 定理 | 三不變量步層保持(start/uniq/wf) | `ConcreteWCR.v` ④ | ✅ kernel |
| 宇宙級(全窮舉) | P1..P5 @ 3×3/3×4/3×5;P2 @ 倒掛 3×3 | Maude | ✅ true |
| 宇宙級(全窮舉) | 精確交換 @ 未過濾 4×6(2,443,506 對) | Rust r3_swap | ✅ 0 違反 |
| 終止(超集) | R1-sup TRS SN | NaTT YES | ✅ 外證 |
| 開放 | `ct_join_exact`/`R3_ct_wcr`/`R4_ct_confluent` | — | ⬜ 下一輪 |
