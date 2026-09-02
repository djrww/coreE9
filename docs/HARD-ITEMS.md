# 後續工作的五個難點(與克服方案、完成證明)

> 2026-09-02。配套:`docs/R3-RESEARCH.md`(文獻 + 前測)、`docs/ROCQ-PLAN.md`(階段計劃)。
> 排序原則:**「會卡住整個路線圖的」在前**,不是「代碼量最大的」在前。
> 每項都給:難在哪 / 克服方案(可執行)/ 完成證明(機器可驗,不是口頭承諾)。

---

## #1 R3 的「一般性缺口」:窮舉 ≠ 定理,而交換引理要在 **∀ 狀態** 上成立

**難在哪**
CT 菜單的規則集是**狀態依賴**的:`cut` 由「與我衝突、起點比我晚」的事件集合決定。
形式化時真正的難點不是「交換」本身,而是要先證明*兩個相鄰步驟各自仍適用*
(否則 `a --rb-->` 根本不是一個步驟)。這類「適用性封閉性」在文獻裡通常靠
Huet 式「線性 + 無重疊變量」的結構假設;我們的狀態沒有變量,得自己搭。
若在此卡住,ROCQ-PLAN §七-1 的退路是「有限空間反射證書」,但那等於承認
「任意狀態 WCR:開放」—— 學術宣稱會縮水。

**克服(三層,按順序降檔)**
1. **不變量先於交換**:證 `ct_step_start_invariant`(CT 步只寫 `iend`);
   再證 `cut_for` 只讀 `(storage, kind, istart, overlap)`,而 `overlap` 在end 縮短下**單調不增**
   ⇒ 「他人的 cut 不變 or 消失」。前測已在 3.1M 狀態上確認等式級交換成立,
   故第 1 層大概率封閉。
2. **Peak decreasingness 降維**:若 (1) 卡 > 4 人日,改用
   Sternagel–Thiemann 的 *peak decreasingness*(它**取代 Newman 引理**的角色,
   只需局部 peak 的標籤遞減; Isabelle 已形式化 ⇒ 骨架可照抄)。Rocq 內寫
   `Definition peak_dec := forall peak, exists closing with labels ≺ peak`,
   再用它一行推出 WCR。
3. **反射式證書保底**:把檢查器 `check_joinable` 在 Rocq 內定義 +
   `checker_sound` 定理 + `native_compute` 對 4×6 求值 ⇒ 得到
   「有限空間的機器可檢查證明」,並在 SPEC-TRACE 標「∀ 狀態 WCR:開放(原因:X)」。

**我如何證明做到了**
- Rocq:`Require Import Cl0r0.ConcreteWCR.` 後 `Print Assumptions ct_wcr.`
  **必須只列出專案自身的公理**(不含 `classic`,除非明列),這是我對「構造性」的自我約束;
- 完成線:`Theorem R3_ct_wcr : forall s a b, step_ct s a -> step_ct s b -> joinable step_ct a b.`
  `Theorem R4_ct_confluent : confluent step_ct.`(由 `newman R2_sn_step_ct R3_ct_wcr`)
  兩者 kernel 檢查綠;
- 對帳:同一 4×6 宇宙,Rust 窮舉報告 0 違反 ∧ Rocq 定理存在 ⇒ 兩層同結論;
  SPEC-TRACE 的 R3 行由 ⬜ 改 ✅,並保留 635,424 計數作證人。

---

## #2 鏡像 ↔ 實作的語義漂移(兩個語言、兩份定義)

**難在哪**
`Mirror.v` 是手寫的 Rust 翻譯。任何 Rust 端重構(例如把 `applicable` 從 filter 改成
直接構造、或把 `runtime` 從 `Vec` 改成 `BTreeSet`)都會讓 Rocq 定理變成「關於舊碼的定理」。
本專案已有實證:R4Runtime 尾插/頭插的分歧就是這樣被抓到的。
更險的是**沉默漂移**——差分樣本沒打到的那條路徑(例如 `range (b - a)` 在 `b < a` 時 nat 截斷)。

**克服**
1. **樣本點由「行為」生成,不由「名詞」生成**:`tools/rocq_reconcile.py` 目前用固定樣本;
   改為把 `gen.rs` 的確定性生成器接上,每輪抽 200 個隨機狀態 × 4 規則,輸出
   `key=value` 讓 Rocq `vm_compute` 複驗(差分覆蓋规则實現,不是覆蓋測試清單)。
2. **鏡像契約測試(單邊也必須成立)**:新增具名測試把鏡像決策寫成 Rust 斷言,
   例如「`applicable s CT Guarded` 長度 == `Raw` 長度」(已入 `test_law_L9b_parallel_moves_exact_swap`),
   「start 在一步後不變」——這些同時是 Rocq 引理的陳述清單。
3. **每條 Rocq 定理配一個 `Example chk_*`**:把定理的關鍵實例化寫成 `eq_refl` 可判定的等式,
   編譯進 CI;定理改了而等式沒改 ⇒ CI 紅。
4. **凍結序**:`ROCQ-TRACE.md §二` 的 D1–D6 是契約;改 `rep.rs` 的 PR 必須同時改該表,
   否則 CI 的 `rocq` job 紅(把「鏡像一致性」從人的紀律變成機器的紀律)。

**我如何證明做到了**
- 對帳樣本點從 19 → **≥ 800**(全部 kernel 複驗綠),並在報告中列出「補獲的真實分歧數」
  (≥1 即證明框架有效;若為 0,我會明說「本輪未捕獲新分歧」而非宣稱「不會有分歧」);
- 引入一次「故意漂移」演練:把鏡像某函數改壞(不進主線),CI 必須紅 ⇒
  把這段輸出貼進 PR 描述,作為框架敏感性的證據。

---

## #3 反射式證書的計算複雜度(4×6 = 623k 狀態 × 635k 對)

**難在哪**
`native_compute`/`vm_compute` 在幾百萬級搜索上是「能跑但會爆記憶體/時間」的區間;
而 `checker_sound` 的定理若把複雜度寫進陳述(例如 `Nat.log` 上界)反而難證。
CI 裡一個 20 分鐘的 job 會把「每次提交都跑形式化」變成人人想繞過的關卡。

**克服**
1. **分級證書**:tier-0(每 commit)`2×4 / 3×5`(<10 萬對,秒級);
   tier-1(每晚/發版)`3×6 / 4×5`;tier-2(手動 dispatch)`4×6` 全量 + 證據檔 artifact。
2. **在 Rocq 內做「分段 + 折疊」而不是單次求值**:把狀態列表切成 k 塊,
   `check_chunk : chunk -> bool` + `checker_chunks_or`,用 `NativePins`/`native_compute`
   只在 tier-2 開(8.20 的 native compiler 已被標記 deprecated ⇒ 以 `vm_compute` 為保底,
   CI 兩條路徑都驗)。
3. **算法級優化而非算術級**:交換引理(§一-2.2 事實 1)成立 ⇒ 檢查器可只做
   **一步交换的等式檢查**(O(|rules|²) 每狀態,無深度搜索),
   把 4×6 的檢查從「可回合性搜索」降為「常數時間等式」——**前測已證明這一步安全**,
   因為等式級在 2.44M 對上全綠。
4. 時間預算寫進文件:`docs/ROCQ-TRACE.md` 記錄每 tier 的 wall-clock(回歸即警)。

**我如何證明做到了**
- 交付一份 `rocq/cert/` + `tools/cert_check.sh`,在本地與 CI 各跑一次,輸出
  `tier-0: PASS 3.1s / tier-1: PASS 47s / tier-2: PASS 14m20s`(實測數字,不預估);
- 若 tier-2 在 CI 超時,我**不會**偷偷降規模,而是把它標 `continue-on-error` + 開 issue,
  在 ROCQ-TRACE 誠實記「tier-2 僅本地可驗」。

---

## #4 環境與 CI 的可重建性(證明工程最大的非數學風險)

**難在哪(本輪已實測到)**
* 快照不保留工具鏈:`~/.cargo`、`~/.rustup`、apt 裝的 coq 全部消失 —— 每次換環境都要重建;
* 我實測 `sudo sh scripts/setup_dev.sh` 會把 rustup 裝進 `/root/.cargo`(因為 `$HOME=/root`),
  非 root shell 找不到 `cargo` —— **這就是 ROADMAP 說「一鍵重建」但實際半失效的地方**;
* CI 用 `apt` 的 coq 8.20.1,與官方 9.2 的 `Stdlib.*`/MathComp 更名不一致 ⇒ 敘述一樣但編譯可能斷;
* 本輪另發現:**CI 的 bench job 在 main HEAD `b904921` 是 failure**,本地重現同一失敗
  (`laminar` 1.262、`named_sexp` 1.351 超 +25%),說明不是環境噪声而是基線脆弱(見 #5)。

**克服**
1. 修 `scripts/setup_dev.sh`:偵測 `EUID`,`sudo` 只用於 apt,rustup 一律**用戶級**
   (`HOME` 顯式傳入;最後 `source $HOME/.cargo/env`);加 `--check` 模式只報版本不安裝。
2. `rocq/` 加 `rocq-project`/`_Project.toml`(Rocq 9.x)並保留 `Makefile` 保底;
   CI 的 `rocq` job 改 `coq-community/docker-coq-action@v1` + `custom_image: rocq/rocq-prover:9.2`
   與 `coqorg/coq:8.20` **兩格矩陣**(命名空間斷裂要在 PR 就紅,不在 merge 後紅)。
3. 把工具鏈版本寫進產物:CI artifact 存 `rustc --version`/`coqc --version`/`ocaml -version`;
   ROCQ-TRACE §〇 的環境表由 CI **自動回寫**(或至少在 job log 輸出可複製的行)。
4. 所有「證明綠」的宣稱一律附 **命令 + 退出碼 + 關鍵日誌行**(本檔 §五 已是此格式)。

**我如何證明做到了**
- 在一台「全新快照」的機器上執行 `sh scripts/setup_dev.sh --check && cargo test --all && make -C rocq reconcile`,
  退出碼 0,並把完整日誌(含版本行)收進 `docs/logs/<date>/`;
- PR 附帶 CI 截圖/日誌連結,證明 9.2 與 8.20 兩格都綠。

---

## #5 門檻的「可信度」問題:門太寬或太窄,都會讓全綠失去意義

**難在哪**
* bench gate:`BASELINE.json` 是單機單次採樣(2 核沙箱、未跑 PGO、0.0005 ms 級小數),
  ±25% 對 nanosecond 級指標是**統計上無效**的 —— 本輪 CI 紅即是證據;
* coverage gate:`ast.rs` 76.9% 走豁免、`rep.rs` 90.6% 貼線,門檻再動一下就紅;
  且 `llvm-cov` 對 3 行 `match` 的行計數器歸屬失真(已記在 COVERAGE §二),門檻把「工具失真」當「代碼缺陷」;
* 測試計數口徑:`cargo test --all` 實測 15 + 32 = 47,但 ROADMAP 寫 46/46、
  文檔另一處寫 31 條 —— 「幾條測試」這種最基本的宣稱都對不上,是誠信風險。

**克服**
1. **bench 改成「倍數 + 重複 + 統計」**:`--tol` 語義改為
   `cur_median / best_of_n > 1 + tol` 且要求 `n ≥ 7`、report `min/median/max` 與 bootstrap CI;
   絕對值門檻改成「與**上一版基線**比較,基線隨 release 更新」;tiny 指標(<10 µs)
   標記為 `informational`(不判紅),或改測 throughput(每萬樣本毫秒)。
2. **coverage 改「關鍵路徑覆蓋」**:除行覆蓋外,加「每條具名測試必須真的執行其聲稱的符號」
   —— 用 `#[cfg(test)]` 計數器或 `cov_gate.py` 的 per-symbol 檢查;豁免只准「列舉原因 + 行級白名單」。
3. **數字單一來源(single source of truth)**:新增 `tools/gen_status.py`,由
   `cargo test -- --list` + `cargo llvm-cov` + `make -C rocq` 產生
   `docs/STATUS.json`,CI 據此**回寫** ROADMAP/COVERAGE/SPEC-TRACE 的表格(人不再手抄)。
4. 「律不過,碼不合」升級為「**數字不對,PR 不合**」:CI 加一個 `docs-consistency` job,
   比對文件中的計數與 `STATUS.json`。

**我如何證明做到了**
- CI 上 **連續 20 個 commit 無 flap**(用 Actions API 統計 red/green 序列),並把
  本輪的 `laminar/named_sexp` 失敗轉成一個可重現的基線修復 PR(附新基線與 n=7 統計);
- ROADMAP 的測試計數由機器產生,我提交的 PR diff 中那幾行與 `STATUS.json` 逐字相同。

---

## 附:完成證明的總原則(貫穿五項)

1. **每個宣稱綁一個可執行命令**(本檔每節的最後一段就是驗收腳本);
2. **證人 ∧ 證明雙軌**:Rust 窮舉/具名測試是證人,Rocq 定理是證明,兩者由
   SPEC-TRACE + reconcile 差分锚住 —— 缺一邊就標 ⚠️,不標 ✅;
3. **失敗也是交付**:卡住時提交「如實縮小範圍的定理 + 開放問題 + 前測證據」
   (本檔 #1 的第 3 層),絕不用 `Admitted`、`Axiom`、或註解裡「顯而易見」混關;
4. **可重建性優先**:所有綠色都要能在一台乾淨機器上重跑出來(#4)。
