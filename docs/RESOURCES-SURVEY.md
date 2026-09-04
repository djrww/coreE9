# 資源搜查報告(聯網)—— 論文 · 教科書 · 工具 · 演算法 · 邏輯定理

> 2026-09-03。應「聯網搜查相關論文、教學、資源、工具、演算法、邏輯定理」之要求整理。
> 對照本專案 `docs/ROCQ-PLAN.md`(P3 #12:把核心重證成 Rocq 定理 R1–R7)與 `docs/ROADMAP.md`。
> 每項附「與本專案的關係」一欄,方便直接取用。

---

## 1. 理論基礎:教科書與經典論文

* **Franz Baader & Tobias Nipkow, *Term Rewriting and All That*(Cambridge UP, 1998)** —— 本領域標準教科書,統一自足地cover抽象歸約系、終止、合流、完成(completion)、組合問題,外加萬有代數、合一論、Gröbner 基。第 7 章正是關鍵對與 Knuth–Bendix 完成的教科書化論述。
  預覽: https://api.pageplace.de/preview/DT0400.9781316043851_A23888933/preview-9781316043851_A23888933.pdf
  Google Books: https://books.google.com/books/about/Term_Rewriting_and_All_That.html?id=N7BvXVUCQk8C
  **(本計畫)**:R1(抽象 Newman)、R7(KB 關鍵對)、R5(R6 區間著色)的教科書出處。

* **Newman(1942)** 原始引理;**Huet(1980, JACM 27(4))** 的 strip 論證;**Knuth–Bendix(1970)** 關鍵對;**Gramlich(1996, TU Wien PhD)**「SN ⇒ [CR ⇔ 臨界對可回合]」。
  口徑 see nLab/百度: https://ncatlab.org/nlab/show/Newman%27s+lemma
  Wikipedia: https://en.wikipedia.org/wiki/Newman_lemma
  **(本計畫)**:R1/R3 的直接理論錨;`AbstractArs.v` 的 `newman` 即 Huet 式。

---

## 2. Newman 引理 & ARS 在 Coq/Rocq 的形式化(先例充分)

* **`rocq-archive/coq-in-coq`(Barras)** —— 在 Coq 形式化 calculus of constructions 的元理論,收錄 `newman.coc`(Newman 引理的 Coq 證明),並抽取可信 proof-checker。
  https://github.com/rocq-archive/coq-in-coq
  **(本計畫)**:證明「抽象 ARS 定理在 CIC 可機械化」的開山先例,與 `AbstractArs.v` 同層。

* **Galdino & Ayala-Rincón, *Newman's lemma — a case study in proof automation and geometric logic*(LICS 2003)** —— 統整 Newman 引理在 ACL2/Coq/Isabelle/Boyer-Moore/Otter 的形式化;並用 coherent logic 自動重建 Huet 的歸納步。
  https://www.researchgate.net/publication/240137699_Newman%27s_lemma-a_case_study_in_proof_automation_and_geometric_logic
  **(本計畫)**:方法論對照:「把歸約序列當可變換物件」+「自動生成證明再由 kernel 驗」,正是本計畫三層自証的哲學。

* **Kathrin Stark, *Mechanising Syntax with Binders in Coq*(2020, Saarland)** —— 文中直接給出 Newman 引理在 Coq 的形式化(附 Lemma A.4),並用它從「局部合流 + 終止」推出合流。
  https://www.ps.uni-saarland.de/Publications/documents/Stark_2020_Mechanising.pdf
  **(本計畫)**:R1(抽象 Newman)在 Coq 的可取用現成證法骨架。

* **Ivanov, *Generalized Newman's Lemma for Discrete and Continuous Systems*(FSCD 2023)** —— 把 Newman 推廣到「不需終止」的嚴格歸納前序集,並在 **Isabelle/HOL** 機器驗證;普通 Newman 是其推論。
  https://drops.dagstuhl.de/storage/00lipics/lipics-vol260-fscd2023/LIPIcs.FSCD.2023.9/LIPIcs.FSCD.2023.9.pdf
  **(本計畫)**:與 R3 的 peak-decreasingness 互補;當「任意狀態 WCR」卡住時,這種「不需終止的合流判準」是備援。

---

## 3. 遞減圖 / peak-decreasingness(R3 的備援路線)

* **van Oostrom, *Confluence by Decreasing Diagrams*(TCS 126, 1994)** —— 完備合流判準(標號 + 局部菱形遞減 ⇒ 合流)。已在 R3-RESEARCH §一 標為備援。
* ***Confluence by Decreasing Diagrams — Formalized*(arXiv 1210.1100)** —— 在 **Isabelle/HOL** 形式化 decreasing diagrams(valley 與 conversion 兩版),證明骨架清晰。
  https://arxiv.org/html/1210.1100
* ***Labelings for Decreasing Diagrams*(arXiv 1406.3139)** —— 自動化:字典序組合標號、把 left-linear 結果擴到 linear(δ條件)。
  https://arxiv.org/abs/1406.3139
* **Endrullis 的 Confluence 研究頁** —— 弱菱形性質與 decreasing diagrams **強度相等**(不需選擇公理),兩個標號即完備於可數系統,並給「約一頁」的新證明。
  https://joerg.endrullis.de/research/confluence/
* ***Completeness of Decreasing Diagrams for the Least Uncountable Cardinality*(Isabelle/AFP)** —— decreasing diagrams 在「最小不可數基數」上的**完備性**。
  https://devel.isa-afp.org/entries/Completeness_Decreasing_Diagrams_for_N1.html
- CWI:***A geometric proof of confluence by decreasing diagrams*(2000)**。
  https://ir.cwi.nl/pub/4437
  **(本計畫)**:ROCQ-PLAN §七-1 的備援:若 R3 交換引理卡在一般狀態空間,改走 peak-decreasingness(1 標號)或有限空間反射證書。上列 Isabelle 形式化可直接對照標號骨架。

---

## 4. 關鍵對定理(KB/Huet)+ 其機械化

* **Knuth–Bendix(–Huet) Critical Pair Theorem 在 PVS 的形式化(JAR 45, 2010)** —— 「TRS 局部合流 ⇔ 所有臨界對可合流」,證法沿用 Huet 結構(去掉 SN 前提)。
  https://link.springer.com/article/10.1007/s10817-010-9165-2
* cstheory 對「關鍵對與 KB 完成」的清晰導論性問答(指向 Baader–Nipkow 第 7 章):
  https://cstheory.stackexchange.com/questions/47789/a-clear-and-rigorous-explanation-of-critical-pairs-and-the-knuth-bendix-completi
  **(本計畫)**:R3 用「臨界對」正名;`R3_ct_wcr_raw` 正是「臨界對可合流」在 CT 菜單的特化。

---

## 5. 演算法:合一(本計畫用到的 Martelli–Montanari)

* **Wikipedia: Unification / Martelli–Montanari** —— 簡明規則表;指出線性時間算法由 Martelli–Montanari(1976)與 Paterson–Wegman(1976)各自獨立發現。
  https://en.wikipedia.org/wiki/Martelli-Montanari_algorithm
* **Martelli & Montanari 原始論文, *An Efficient Unification Algorithm*(ACM TOPLAS 4(2), 1982)** —— 把 acyclicity(occur-check)內嵌;PASCAL 實作。
  https://dl.acm.org/doi/pdf/10.1145/357162.357169
* **`xrchz/MM76`** —— 在 **HOL4** 機械化「Unification in Linear Time and Space: A Structured Presentation」(Martelli–Montanari '76),含 term reduction / multiequation / memory model。
  https://github.com/xrchz/MM76
* **`pulmro/martelli-montanari`** —— Prolog 版實作(基於 1982 文);含 occur-check。
  https://github.com/pulmro/martelli-montanari
* **ALE 參考手冊節錄(Unification Algorithm)** —— 以 union-find 角度描述 MM 規則(拆函數項 / 刪等式 / 變元綁定 / occur-check fail)。
  http://www.ale.cs.toronto.edu/docs/ref/ale_trale_ref/ale_trale_ref-node4.html
  **(本計畫)**:`WCRUtil.fold_min_mem` 的註記已把「min 左折結果 ∈ 輸入 ∪ {初值}」描述為 Martelli–Montanari 式**構造性見證抽取**(不需排中律);若要「產生式證明/抽出 witness」,MM 的 multiequation 表示可資取法。另可用於 R6 的匹配/合一(construction v.syntactic)。

---

## 6. 工具:終止 / 合流 / 交叉驗證

* **NaTT(Nagoya Termination Tool)** —— 終止證明器,特徵:weighted path order(WPO)+ 與外部 SMT 求解器(z3)協作 + DP framework 做相對終止。本計畫用它做 R1 側超集 TRS 的終止外證。
  官方: https://www.trs.css.i.nagoya-u.ac.jp/NaTT/
  論文: https://arxiv.org/abs/1404.6626 (Springer 版: https://link.springer.com/chapter/10.1007/978-3-319-08918-8_32)

* **Maude(rewriting logic)** —— 基於重寫邏輯的宣告式/反射式語言,內建 Knuth–Bendix 完成、AC 匹配、Church-Rosser 判定;本計畫用**純函數模塊(方程式語義)**交叉驗證 CT 宇宙性質,刻意不用 `rl`/search。
  系統總覽: https://grokipedia.com/page/maude_system
  **Maude Termination Tool(MTT)+ Church-Rosser Checker(CRC)**:
  https://link.springer.com/chapter/10.1007/978-3-642-24933-4_17
  https://www.researchgate.net/publication/227111023_Towards_a_Maude_Formal_Environment

* **CSI(confluence tool)+ CoCo(confluence competition)** —— 自動合流證明/反駁器,參與年度 CoCo 競賽;可當「把 CT 菜單丟給自動工具看它怎麼證/找反例」的對照。
  CoCo 2024: https://project-coco.uibk.ac.at/2024/participants/papers/infChecker.pdf
  CSI 論文: https://www.researchgate.net/publication/262357990_CSI_a_confluence_tool
  自動策略發明(Grackle 為 CSI 生成策略,超越人類設計): https://arxiv.org/html/2411.06409 | https://www.ijcai.org/proceedings/2025/0526.pdf

* **CiME / CoLoR / MU-TERM / AProVE**(termination & confluence 工具總覽):
  http://rewriting.loria.fr/systems.html
  **(本計畫)**:把 `ct_maude.maude` / NaTT 這套「外部交叉驗證」對照到競賽級工具;若要求「自動合流證明」,可把 CT 菜單寫成 CSI/CoCo 可讀格式交叉檢驗。

---

## 7. Coq/Rocq 戰術(omega / lia / congruence)與學習資源

* **Micromega(arithmetic tactics over ordered rings)** —— `lia`/`nia`/`lra`/`nra`;`lia` 是 **Z 線性整數算術的判定程序**,其權力 = `ring_simplify` + `omega` 之併,且能解 omega 解不出的 `omega nightmare`;支援 Z/nat/positive/N(經 zify 預處理)。
  Rocq 9.x: https://rocq-prover.org/doc/V8.10.2/refman/addendum/micromega.html
  Coq 8.10: https://coq.inria.fr/doc/V8.10.0/refman/addendum/micromega.html
  Coq current: https://coq.inria.fr/refman/addendum/micromega.html?highlight=omega+lia
* **`congruence`** —— 完備於 EUF(無詮釋函數 + 等式);與 `lia` 依 **Nelson–Oppen 組合**(EUF+LIA)的 `smt` tactic:
  https://coq-workshop.gitlab.io/2021/abstracts/Coq2021-02-02-congruence-lia-lra.pdf
  Coq tactics 速查表: https://rand.cs.uchicago.edu/cufp_2015/reference.html
  **(本計畫)**:本計畫 `WCRUtil.v`/`ConcreteWCR.v` 大量用 `lia`(如 `i_overlap_after_cut`、`cut_bounds` 的收尾)、`congruence`(記錄/構造的相等閉包,如 `r1_apply2_comm` 的 `congruence`)、`f_equal`/`injection`;R6 的 nat 半開區間算術正是 `lia` 主場。

---

## 8. Isabelle/HOL 的 Sledgehammer(自動化補償器)

* **Paulson & Blanchette, *Three Years of Experience with Sledgehammer*** —— Sledgehammer 把 Isabelle/HOL 接到自動定理證明器(E/SPASS/Vampire/Metis),不需使用者配置;用它當「lemma finder」,產出 replays 於 Isabelle kernel 的 proof scripts。
  https://www.researchgate.net/publication/228880433_Three_Years_of_Experience_with_Sledgehammer_a_Practical_Link_between_Automatic_and_Interactive_Theorem_Provers
  https://easychair.org/publications/paper/Mzp/open
* ***Extending Sledgehammer with SMT Solvers*(JAR 2013/2011)** —— 加入 SMT(z3 等)與其一階 ATP 互補。
  https://link.springer.com/article/10.1007/s10817-013-9278-5
  https://link.springer.com/chapter/10.1007/978-3-642-22438-6_11
  **(本計畫)**:本計畫採用 Rocq(Coq)路線,不用 Isabelle;但「證明由自動工具找、由 kernel 複驗」的**工作流形態**正是 Sledgehammer/CeTA 的做法(見 ROCQ-PLAN §四 路線 A 的反射式證書、Phase 6)。Rocq 側若想同型,可對照 Coq 的 `Hammer`(CoqHammer)概念。

---

## 9. 區間圖 / 完美圖(T2 律 χ = ω)

* **區間圖是完美圖**(χ = ω;且 chordal ⇒ 完美);用「按左端點排序 + 貪婪著色」即可達最優,且任何導出子圖仍是區間圖 ⇒ 完美。
  證明直觀: https://stackoverflow.com/questions/67200872/proof-about-interval-graphs-being-perfect
  課程講義: http://www.cs.toronto.edu/~bor/373f11/L4-373f11.pdf
  性質總覽: https://grokipedia.com/page/Interval_graph
* 線上著色 / 帶權著色之外(佐證 ω 是硬下界): https://www.sciencedirect.com/science/article/abs/pii/S0195669824000040
  **(本計畫)**:R6(T2)只作用在區間集(`max_clique` = 最大重疊數、`greedy_chromatic` = 按右端貪婪著色),不建圖 ⇒ 不必走圖論「完美」路線,只需組合論證(χ ≥ ω 平凡;χ ≤ ω 用「當前活動集兩兩相交 ⇒ ≤ ω」)。上列就是該論證的教科書形式。

---

## 10. 領域動機:Rust 借用檢查 / Polonius(為何是「區間衝突」)

* **rustc_borrowck** —— `places_conflict`、`Overlap`(無重疊/重疊/放鬆的 monoid 判定)、Polonius 分析模塊。
  https://doc.rust-lang.org/nightly/nightly-rustc/rustc_borrowck/index.html
* **Polonius**(Rust 新一代借用檢查模型): https://rust-lang.github.io/polonius/
  遷移現況: https://daily.dev/posts/rust-s-new-borrow-checker-is-coming--llpxltaj6
  **(本計畫)**:本計畫的讀寫衝突 = `&mut`(Mut)支配 `&`(Sh),重疊 = 半開區間 `start < o.end ∧ o.start < end`;衝突圖 ⊂ 區間圖 ⊂ 弦圖 ⊂ 完美圖。這是 `K::Mut`/`Sh`、`i_overlap`、`k_conflict`、`red_edges` 背後的領域模型對照。

---

## 一句總結(可直接引為「下一步」的搜尋依據)

> 泛型難點(Newman / decreasing diagrams / 關鍵對)**先例充沛且已機械化**(Coq: coq-in-coq、CoLoR/rocq-color;Isabelle: 1210.1100、AFP);本專案的真正硬點不在抽象引理,而在**把 CT 菜單的 WCR 變成可證數學** —— 這正是 `docs/R3-NEXT-STEP.md` 標記的開放缺口,上面的 `red_edges` 遞減 + `ct_guard_redundant` 組裝,可先對照 **van Oostrom decreasing diagrams / peak-decreasingness**(備援)與 **CSI/CoCo**(外部對照)。
