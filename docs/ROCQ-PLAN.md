# P3 #12/#13 —— Rocq(Coq)重寫系統形式化:可行性與可行計劃

> 2026-09-02 規劃(聯網文獻比對後)。狀態:**規劃完成,待指示開工**。
> 範圍:重證核心 —— Newman 引理(現為測試級/窮舉級)、L7b 迭代淨化終止、
> T2 區間圖完美性;與 Rust 側對帳。含 P3 #13「fuzz 統計型 → 生成式證明」是否前置的裁決。

---

## 〇、決策摘要(先讀)

1. **Rocq 上重證本專案核心:可行,且文獻先例充分** —— 抽象 Newman 引理自 1985 年起
   就有 Coq 形式化(Coquand & Huet);終止+重寫庫(CoLoR)與「認證檢查器」工作流
   (IsaFoR/CeTA)都是成熟對標。難點不在抽象引理,而在**把菜單規則的 WCR 變成
   可證明的數學**(下面 Phase 3)。
2. **不需要先把 fuzz 統計型檢查升級為「生成式證明」(P3 #13),它不是前置**。
   理由見 §5.1。真正的前置是 §5.2 的四件事(敘述凍結 / 內核抽取 / 對帳框架 / 環境 CI)。
3. **選型:推薦「路線 A —— Rocq 規格鏡像 + 反射式檢查器」**(§4);rocq-of-rust
   不推薦為主路線(§4.2 說明),可作 Phase 6 選配小驗證。
4. **本文件即 P3 #12 的開工依據**;每階段完成線 = 「Rocq 定理 + Rust 對帳(具名測試/
   差分)」,維持專案「定律先於程式碼、機械自証」紀律。

---

## 一、引理文獻對照(聯網查證)

| # | 文獻 | 與本專案的關係 | 要點 |
|---|---|---|---|
| 1 | M. H. A. Newman, *On theories with a combinatorial definition of "equivalence"*, Annals of Math. 35 (1942) 223–243 | §4.3 L9 的理論基石 | 原始證明(引理本身) |
| 2 | G. Huet, *Confluent reductions: abstract properties and applications to term rewriting systems*, JACM 27(4), 1980 | L9 的證明策略(現有測試用的「簡化證明」) | 用局部合流的「strip」論證 + SN 上良基歸納,證明 Newman 引理 |
| 3 | B. Gramlich, *Termination and confluence properties of structured rewriting* (PhD 1996, TU Wien) | §4.3 L9 策略的第二支柱 | SN ⇒ [CR ⇔ JCP](臨界對定理);支撐「SN + 臨界對可回合 ⇒ 合流」的具體版 |
| 4 | Coquand & Huet(1985 前後)— 見 Galdino & Ayala-Rincón 綜述(ResearchGate) | **首次在 Coq 形式化 Newman 引理** | 證明「抽象重寫系統定理在 CIC 可機械化」的開山先例 |
| 5 | Sternagel & Thiemann, *Abstract-Rewriting**(Isabelle/AFP,IsaFoR 家族;CeTA 認證終止分析器) | 工作流對標 | ① Isabelle 內 Newman 引理/Church–Rosser 已形式化;② **將終止證明檢查器提取成可執行碼,由工具產生證明、機器驗證** —— 正是我們要的「生成式證明」形態 |
| 6 | Blanqui & Koprowski, *CoLoR: a Coq library on well-founded rewrite relations…*, MSCS 21(4):827–859, 2011(+ Koprowski PhD 2008, Ducas 2007) | Rocq 側的庫先例 | Rocq(Coq)重寫與終止理論庫:良基關係、終止序(多項式解釋、依賴對)、**終止證書的自動驗證**;本專案 Phase 2 的 µ 字典序可對照其「良基(order)」基礎 |
| 7 | Singh & Natarajan, *A Constructive Formalization of the Weak Perfect Graph Theorem*(2019)+ Coq 完美圖系列 | T2 的「大定理」先例 | 弱完美圖定理(含 Lovász 複製引理)已在 Coq 構造性證明;但**我們不需要**該級別 —— T2 只需「區間圖 = 弦圖 ⇒ 完美」中可演算法化的一小段(見 §3-R6) |
| 8 | Metatheory(Lean 4,arXiv 2512.09280,2025) | 反向對標 | 在 Lean 4 做了「泛型 ARS 框架 + Newman/Hindley-Rosen」並跨 6 個案例;證明「泛型化 Newman 可複用」,但本專案只需一個實例,不必建框架 |
| 9 | Formal Land,**rocq-of-rust**(原 coq-of-rust) | 選型對照(路線 B) | 從 Rust THIR 譯到 Rocq;工作流 = 翻譯 → 記憶體 → 模擬函數 → 等價驗證 → 性質證明;適合「程式碼安全性質」,對「數學定理」繁重(§4.2) |
| 10 | Rocq Prover 9.2.0(2026-03-27 穩定版;9.0 於 2025-03 更名) | 工具鏈 | 標準庫命名空間已由 `Coq.*` 改為 `Stdlib.*`;MathComp 套件同步更名 `rocq-mathcomp-*` |
| 11 | docker-coq-action / docker-opam-action(rocq-community,arXiv 2510.19089) | Phase 0 與 CI | Rocq 專案的 GitHub Actions 慣用方案(`rocqorg/rocq:9.2` 鏡像);本地沙箱目前**無 rocq/opam/dune**(已查證),Phase 0 需安裝 |

> 另見既有外部來源(§4.3 已引用,不重複):Gramlich 1996 論文原文(logic.at)、
> Wikipedia Newman's lemma(證明珠,證明架構與 Phase 1 一致)。

---

## 二、目標形式化清單(定理敘述凍結 — 與 Rust 律一一對照)

> 定位原則:**Rust 具名測試 = 證人(樣本/窮舉);Rocq 定理 = 證明(所有情形)**。
> 兩者同列於 SPEC-TRACE 對照表。

| 編號 | Rocq 定理(敘述) | Rust 側對應 | 定位 |
|---|---|---|---|
| R1 | **抽象 Newman**:若 ARS (A,→) 強正規化(SN)且局部合流(WCR),則合流(CR);且每一元素有唯一正規形 | L9a/L9b/L9c(機器窮舉) | 純抽象,無專案型別 |
| R2 | **具體 SN**:CommutativeTrim 菜單(Guarded 政策)每規則的每個合法施用,µ = (|E_red|, |Err_rustc|) 字典序嚴格遞減 | L8a/L8b(逐規則斷言) | 逐規則引理 |
| R3 | **具體 WCR**:CommutativeTrim 的任意兩步(重疊或並列)可回合(臨界對可回合;核心引理:「端點只取決於他人 start,且 start 不變」) | L9b 窮舉(623,616 狀態 × 635,424 臨界對) | **數學核心,Phase 3** |
| R4 | **唯一正規形 + 規範化函數**:由 R1–R3 推出:每個可達狀態的正規化函數 `norm` 良定義且唯一 | L9a | 直接推論 |
| R5 | **L7b 終止**:迭代淨化(反覆移除最大錯誤跨度)在「殘餘錯誤跨度總長」測度上嚴格遞減 ⇒ 終止;且不動點中無非空錯誤跨度 | L7b(rounds<8 上限) | 純結構;「切縫/EOF 殘留」的刻畫框定在語法面(§7 風險) |
| R6 | **T2(演算法形式)**:對任意有限半開區間集,`max_overlap(ω) = greedy_chromatic(χ)`(按右端點排序貪婪著色) | `test_theorem_T2_interval_graphs_are_perfect`(400 樣本) | 不需圖論庫;等價於「區間圖完美」的可計算片段(§3-R6) |
| R7(選配) | **反例存在性**:存在具體狀態與兩個規則應用形成不可回合臨界對(NaiveMenu) | L9c「機器找反例」 | 存在性命題,可計算證人 |

**敘述凍結的副產品**:把法則內「µ 側條件 = 規則載體」的哲學(§4.1 `Policy::Raw` 對照)
變成 Rocq 的定義式:Guarded 政策即「規則 + µ 遞減證明」;Raw 政策在 Rocq 中
定義為同型別不帶證明,再證「Raw ⊃ Guarded 的反例存在」(R7)。

---

## 三、可行性分析(逐定理)

### R1 · 抽象 Newman — 易,先例充分
- 做法:SSReflect 或純 Stdlib;定義 `red : A -> A -> Prop`(或重關閉包)、
  `SN`(良基)、`WCR`、`CR`;定理 `newman : sn r -> wcr r -> confluent r`,證明
  用 `well_founded_induction`(Stdlib `Wf_nat` / `Acc` 內建)+ Huet 的 strip 引理。
- 規模:~300–600 行;**為 Phase 1 里程碑(也是熱身)**。
- 風險:近零(眾多先前工作可對照;Coquand–Huet 1985 即此)。

### R2 · 具體 SN — 中;工作量在「內核鏡像」不在證明
- 前置:把 `Interval / K / Ev / AState / Rule / Menu / Policy / apply / µ` 鏡像成
  Rocq 型別(全部有限、一階、可判定相等;`log` 域在 Rust 中「不參與相等性」→
  Rocq 鏡像直接剔除,並在對照表申明)。
- 證明:µ 用「長度對的字典序」;每規則一條引理 `rule_step_decreases`(case 分析)。
  Rust 側 guard 本就是「通過 ⟺ µ 遞減」的機械定義 ⇒ **Rocq 化就是把斷言變定理**,
  語義零遷移。
- 規模:型別/規則鏡像 400–700 行 + 逐規則 300–600 行。
- 風險:µ 的 `|Err_rustc|` 分量在 §6.3 記 0(判定權不轉移)→ Rocq `µ s = (|E_red s|, 0)`,
  不形式化 rustc oracle(**誠實申報**,不做假模型)。

### R3 · 具體 WCR — 中高,是本計劃的數學核心
- Rust 註解已給出關鍵論證輪廓:「修剪只縮短端點,端點只取決於他人 *start*,
  而 *start* 不變」。形式化即:
  1. 引理「**start 不變**」:CommutativeTrim 所有規則不改變任何事件的 `it.start`
     (只縮短 `end`);—— 逐規則檢查,容易。
  2. 交換引理:兩條規則應用 `s -r1-> t1`,`s -r2-> t2`;若作用區間不相交則並列
     應用可交換(各自步驟互不干擾 —— 由「start 不變 + 只動端點」),若相交則
     構成臨界對,規則集小 ⇒ 有限種類的臨界對逐一驗證。
  3. WCR 因而成立(對**任意**狀態,非僅有限空間)。
- 對照:Rust 側的窮舉(4×6 = 623,616 狀態 × 635,424 臨界對,**0 違反**)保留為證人;
  證明使其成為定理。
- 規模:800–1500 行;**困難點**:把「端點只取決於他人 start」寫成可用的引理。
  若發現規則其實有隱式依賴(Rule 的 guard 讀了日誌/次序)→ 誠實回報,與週期
  敘述不符處以「改規則」或「縮小範圍」二擇一(§7)。
- 可替代路線(若 R3 卡住):退而求其次證明「對所有 *有限* 狀態空間,窮舉檢查器
  是 sound 的」+ 在此空間跑 native_compute —— 得到「有限空間的機器可檢查證明」
  (反射式,Phase 6),而「任意狀態的 WCR」標記為開放。

#### R3 開工實測(2026-09-02):捷徑被證偽,改走 trim1 成對歸納

上面第 1–2 點那個「start 不變 ⇒ 他人候選集不變」的捷徑,本輪在 Rocq 裡
**實測為假**(不是證不出,是命題本身不成立):

```
ct_pred a b  =  id b ≠ id a ∧ storage b = storage a ∧ k_conflict (kind a) (kind b)
                ∧ istart a < istart b ∧ i_overlap (it a) (it b)
i_overlap a b = (istart a <? iend b) ∧ (istart b <? iend a)
```

`cut_for` 的候選集確實只由 (id, storage, kind, istart) 與**雙方 iend**決定;
其中 `istart b <? iend a` 這一项讓「剪 a 自己」會縮小 a 的候選集。
`rocq/theories/WCRUtil.v` 以三個 `vm_compute` 事實把此事釘死(已 kernel 驗):
`ct_pred evA evB = true`、`ct_pred (trim_ev evA 2) evB = false`、
`ct_pred evA (trim_ev evB 1) = true`(剪被觀者的 iend 在本例不改判定,
因為 `istart a <? iend b` 在合法區間下恆真 —— 這正是捷徑**看起來**成立的原因)。

**修正後的 R3 證明形狀**(逐點不變 → 對稱收縮):

1. `trim1 i c l l'`:成對走兩列表的歸納謂詞,精確刻畫 `trim_at`
   (已入庫:`trim1_spec`、`trim1_length`、`trim1_nth_other`)。
   用它是因為 `nth_error` 按 nat 遞歸,遇未約簡的 `trim_at q i c` 會卡死
   —— 這是本輪最大的時間黑洞,後續請勿重蹈。
2. 交換性的正確論證:**兩步各自剪掉一個事件**,而「剪 b」對「非 a 的第三方
   候選集」是同一個變換(與先剪 a 或先剪 b 無關)⇒ 兩次得到的候選集**同構**,
   其 min-start 相同 ⇒ `cut_for` 給出同一個 `c` ⇒ 狀態逐字相等
   (與探針的「精確交換」一致,不是僅「可回合」)。
3. 由 2 直接得到的**可證事實**(已逐字核對定義,非憑記憶):
   `ct_pred a x` 的五个合取項中,只有 `istart a <? iend x` 讀 **x 的 iend**;
   其餘(`ev_id x`、`ev_storage x`、`ev_kind x`、`istart x`、`istart x <? iend a`)
   對 x 的 iend 不變 ⇒ **對「未被剪的那一位 x」,候選判定逐點不變**;
   被剪的那一位(`x = b`)則可能由 true 翻成 false。
   故 R3 可用的引理形狀是「**候選集的差異恰好是被剪者本身**」,不是不變性;
   而 t1、t2 兩側的差異是**同一個** b(同理同一個 a),這是對稱性所在的關鍵。
4. 尚**未**完成的部分(誠實申報,屬下一輪):把 3 對稱性收斂成
   `cut_for` 層等式(`cut_for t1 a = cut_for t2 b'` 之類),以及
   `R3_ct_wcr` / `R4_ct_confluent` 主定理。目前 `rocq/theories/WCRUtil.v`
   只入庫結構層(① trim1 三引理 + ② 三個 vm_compute 語義事實),
   不含任何 `Admitted`/`Axiom`,`make -C rocq` 全綠。

### R4 · 唯一正規形 — 易
- R1–R3 拼上即得;規範化函數用 `Fix`(在 SN 的良基關係上)。
- 規模:<200 行(若 Phase 1 已把 Newman 引出為定理)。

### R5 · L7b 終止 — 低–中,純結構
- 測度:每次迭代移除之錯誤跨度總長(或錯誤數)嚴格遞減 → 良基 Fixpoint
  `normalize`;證明 Rust 的 `rounds < 8` 上限永不觸發(用測度上界)。
- 不動點性質:「無非空錯誤跨度」的刻畫需 CL0 語法(什麼是「最大錯誤跨度」、
  「空跨度」)—— **範圍裁決**:Rocq 側證明「normalize 終止 + 不動點(迭代不變
  式)」,「切縫/EOF 殘留的形式刻畫」留給語法面(或 Phase 5 與 T2 並行的選配)。
- 規模:300–600 行。
- 注意:l7b_evaluate 在 Rust 是「str 進出」;Rocq 鏡像用 `list nat` / `string`
  (Rocq `String` 或 `list byte`),對照層用同一語料比對輪數與殘留。

### R6 · T2 χ = ω — 低–中,且**比「圖」的版本簡單得多**
- 關鍵:**我們的 T2 測試直接作用於區間集**(`max_clique` = 最大重疊數、
  `greedy_chromatic` = 按右端點排序的貪婪著色),**不建圖**。因此不必走
  「弦圖 ⇒ 完美」的圖論路線(那要 MathComp 圖論庫),只需組合論證:
  - χ ≥ ω:任何 ω 重疊點上的區間兩兩相交 ⇒ 兩兩異色(trivial);
  - χ ≤ ω:歸納於按右端點排序的區間序列 —— 每步著色 ≤ 當前重疊數 ≤ ω
    (需「當前活躍集」= 與新區間相交且右端點 ≤ 其右端點者,皆兩兩相交 ⇒ 大小 ≤ ω)。
  - 半開區間 + nat 算術,Rocq 側用 `nat`(`Interval := (nat × nat)` 加 proof
    `start < end` 或直接容許空),SSReflect `finset`/`seq` 即可。
- 規模:400–800 行;任何經典組合教材都有此論證(如 Erdős–Kleitman 的區間著色
  引理;本質上等價於「區間圖完美」的習題形式)。
- 對照:Rust 400 樣本測試保留;另用抽出的 Ocaml 檢查器跑同語料。

### R7 · NaiveMenu 反例存在性 — 易
- 具體狀態 + 兩個規則應用 + 不可回合證明(native_compute 檢查或手工 refine);
  證人直接從 L9c 的機器反例搬。

---

## 四、選型:三條路線

### 4.1 路線 A(推薦):Rocq 規格鏡像 + 反射式檢查器
- 做法:`rocq/cl0r0/` 子專案鏡像核心定義(§2 清單),證明 R1–R6;
  另寫「檢查器」`check_joinable : list AState -> bool` + 定理
  `checker_sound : check_joinable ss = true -> 該空間內一切臨界對可回合`;
  用 `native_compute` 對 3×4×6 / 4×6 具體空間求值 → **得到可由 rocq 重新驗證
  的證書(證明由計算產生)**。
- 優:定理是「數學的」(與 Rust 實作解耦,但以 SPEC-TRACE 對照表錨定);
  每一步都有里程碑;失敗成本低;與專案「定律先於程式碼」哲學同構。
- 缺:需要對帳(差分/具名測試)維持「兩個語言的定義一致」—— 但這正是
  P3 #12 的「與 Rust 側對帳」條文,且本專案已有 46 具名測試作為錨。

### 4.2 路線 B(對照):rocq-of-rust 直接把 rep.rs/tree.rs 譯到 Rocq
- Formal Land 的流程:翻譯(THIR)→ 記憶體模型 → 模擬函數 → 等價驗證 → 性質。
- **不推薦為主路線**,理由:
  1. 本專案的定理是**語義數學定理**(合流、終止、χ=ω),不是「程式碼安全性質」;
     翻譯後的 monadic 表示難以表述與證明這些定理(逆向工作)。
  2. 每次重構需重譯;我們的律是規格級,鏡像一次成本更低,且不因 Rust 內部
     重構(如 P0 #4 的並行化)而失效。
  3. 對應的「程式級」性質我們已有:窮舉 + 測試 + fuzz(無 panic/總性)。
- 保留為 Phase 6 選配:用 rocq-of-rust 驗 `rep::apply` 的總性/無 panic(小範圍),
  作為「實作層」對「語義層」的橋。

### 4.3 路線 C(教育性):只做 R1 抽象 Newman
- 最小投入(一週),但未觸及本專案實質(菜單規則的 WCR)。可作為 Phase 1
  的熱身,不作終點。

---

## 五、前置準備評估(直接回答「是否先做生成式證明」)

### 5.1 P3 #13(fuzz 統計型 → 生成式證明建議報告):**不是前置,建議擱置**
- 本專案的「機械自証」已是三層:**具名測試(樣本證據)→ 窮舉機器檢查
  (有限空間全量)→ 定理(所有情形)**。fuzz 是**搜尋工具**(在無界空間找反例),
  它的「統計性」是特性不是缺陷;生成式證明(每輪記錄證人 + REPORT.md)是
  **審計報告**,不改變任何陳述,也不為 Rocq 證明提供素材。
- 成本/收益:成本約半天至一天(確定性種子 + 證人傾印 + 報告格式);
  對 Rocq 前置的貢獻 ≈ 0。因此:**不做前置**。
- 若仍要「極小版」(建議排在 Phase 5 之後):P0 #3 的縮小 + fixtures 已具備
  證人存檔;只差「每輪檢查清單 + 機器可讀 REPORT」,約半天。

### 5.2 真正的前置四件(按此順序)
| 前置 | 內容 | 交付 |
|---|---|---|
| P0-a 敘述凍結 | §2 對照表定稿:`Rocq 定理 ↔ 律 ↔ 具名測試`;SPEC-TRACE 增「Rocq 欄」 | §2 表入庫;每定理標「證人/證明」定位 |
| P0-b 內核抽取 | 確定 Rocq 鏡像的最小定義集;逐項裁定:剔除 `log`、`Err_rustc` 記 0、Interval 用 `nat`、runtime 邊是否參與規則 | `rocq/` 骨架 + 對照表「鏡像 vs 實作」欄 |
| P0-c 對帳框架 | Coq 抽取 checker → OCaml(或 native_compute 輸出),與 Rust 同語料差分(小空間起步);CI job | 差分測試 + CI `rocq` job |
| P0-d 環境 | 沙箱安裝 rocq 9.2(opam 或官方二進位;已查證沙箱無 rocq/dune);CI 用 `rocqorg/rocq:9.2` 鏡像(docker-coq-action / docker-opam-action) | 本地 + CI 可 `dune build @runtest` |

---

## 六、分階段計劃(完成線 = Rocq 定理 + Rust 對帳)

| Phase | 內容 | 完成線(驗收) | 預估 |
|---|---|---|---|
| 0(0.5–1 週)| 環境 + 內核鏡像(型別/AState/菜單/µ)+ 對帳骨架(P0-a…d) | `dune build` 綠;鏡像 × Rust 小空間差分過;CI job 綠 | 0.5–1 週 |
| 1(1 週)| R1 抽象 Newman + 唯一正規形(抽象層) | `newman` 定理入庫;SPEC-TRACE 增列 | 1 週 |
| 2(2–3 週)| R2 具體 SN(µ 字典序,逐規則)| 每規則 `step_decreases_µ` 定理;Rust L8 測試保留對照 | 2–3 週 |
| 3(2–4 週)| R3 WCR(交換引理 + 臨界對)→ R4 CR 定理 | `confluent`(任意狀態)定理;4×6 窮舉同結論 | 2–4 週 |
| 4(1–2 週)| R5 L7b 終止 + 不動點 | `normalize` 總函數 + 終止定理;與 Rust l7b 同語料比對 | 1–2 週 |
| 5(1–2 週)| R6 T2(χ=ω,SSReflect)| 定理 + 400 樣本對照 | 1–2 週 |
| 6(1 週,選配)| 反射式證書(native_compute 對 3×4×6/4×6)+ R7 反例存在性 + (如要)P3 #13 極小版 | 證書可複驗;R7 證人 | 1 週 |
| 合計 | — | — | **8–14 人週**(證明互動,±50%) |

**紀律**:每 Phase 結束即「Rocq 定理 + Rust 對帳綠」才可宣稱完成;任何
「Rocq 定義與 Rust 行為不一致」的發現 = 回到 P0-b 對照表,修定義或修 Rust
(二擇一,不得默認)。

---

## 七、風險與誠實申報

1. **R3 是唯一高風險點**:原計劃的捷徑「start 不變 ⇒ 他人 cut 不變」已於
   2026-09-02 實測**作廢**(詳見 §三-R3「開工實測」:`ct_pred` 含
   `istart a <? iend b`,故被剪者本身會從他人候選集移除)。改走「候選集差異
   恰為被剪者本身 + 兩側對稱」路線。若該路線仍卡住,則 WCR 證明需改走
   「有限空間反射證書」路(Phase 6 提前),並在 SPEC-TRACE 如實標註
   「任意狀態 WCR:開放」。
2. **語義與數字表示**:Rocq 用 `nat`/`list` 鏡像,實作用 `u32`/`Vec` —— 同構性
   靠對帳(差分/具名),文件明示,不假設自動。
3. **rustc oracle**:µ 的 `|Err_rustc|` 分量按 §6.3 記 0(判定權不轉移),不建假模型;
   若未來引入 oracle,µ 需重寫 —— 屆時 R2 範圍變更,按規格流程提交。
4. **L7b 的「切縫/EOF 殘留」刻畫**依賴 CL0 語法面;Rocq 範圍裁決為「終止 +
   不動點性質」,語法面刻畫列為選配(或 Phase 5 並行項)。
5. **工具鏈**:Rocq 9.2 + MathComp(rocq-mathcomp-*)為新命名系列;若 CI 鏡像
   供應滯後,退 `9.0`/`8.20` 亦可(敘述不變,只有命名空間差異)。
6. **P3 #13 裁決**:不做前置;已於 §5.1 記錄理由(若您不同意,可推翻)。

---

## 八、參考文獻(本計劃引用的具體來源)

- Newman, *Annals of Math.* 35 (1942) 223–243;Huet, *JACM* 27(4) (1980) 797–821;
  Knuth & Bendix (1970);Gramlich, PhD thesis (1996), TU Wien
  (https://www.logic.at/staff/gramlich/papers/thesis96.pdf)。
- Galdino & Ayala-Rincón(comprehensive survey citing Coquand–Huet 1985 Coq formalization
  of Newman's lemma): https://www.researchgate.net/publication/240137699
- Sternagel & Thiemann, IsaFoR / AFP *Abstract-Rewriting* + CeTA (certified termination
  analyzer): https://cl-informatik.uibk.ac.at/software/ceta/ ;IsaFoR:
  https://github.com/sternagel/IsaFoR(見 web_search 結果 #1 之二)。
- Blanqui & Koprowski, *CoLoR*, MSCS 21(4) (2011) 827–859:
  https://hal.science/inria-00543157 ;https://github.com/fblanqui/color 。
- Singh & Natarajan, *A Constructive Formalization of the Weak Perfect Graph Theorem*
  (2019): https://www.researchgate.net/publication/338762364(亦見圖論 Coq 庫概覽)。
- Metatheory(Lean 4),arXiv 2512.09280(2025):
  https://www.arxiv.org/pdf/2512.09280 。
- Formal Land, *rocq-of-rust*: https://formal.land/docs/tools/rocq-of-rust/introduction ;
  https://formal.land/blog/tags/translation(方法/限制系列)。
- Rocq Prover 9.2.0(release 2026-03-27;9.0 更名 2025-03):
  https://rocq-prover.org/ ;https://en.wikipedia.org/wiki/Rocq 。
- MathComp 套件更名 `rocq-mathcomp-*`(opam):
  https://github.com/math-comp/math-comp 。
- docker-coq-action / docker-opam-action(Rocq/OCaml CI):
  https://github.com/coq-community/docker-coq-action(arXiv 2510.19089 綜述)。
- Wikipedia, *Newman's lemma*: https://en.wikipedia.org/wiki/Newman_lemma 。
