# 貢獻指南(Contributing Guidelines)

> 本檔定義 **CL0/R0(cl0r0)** 的貢獻政策。請在操作本庫前先讀完。
> 一句話:**這是封閉、機械自証的儲存庫 —— 不開放外部 issue、PR、fork。**
> 決策依據見 `docs/adr/0002-closed-contribution-model.md`。

---

## 0. 本專案的性質(為何封閉)

cl0r0 是「可執行載體 × 可證件載體」的**雙載體**代碼庫:

* **載體一(Rust)**:CL0/R0 的可執行語義與九條定律(L1–L9)的實作;可執行、可測試、可基準。
* **載體二(Rocq / Coq)**:同一語義的鏡像(`rocq/theories/`),由 `tools/rocq_reconcile.py`
  與 Rust 實例做**交叉對帳**;定理經 `Print Assumptions` 驗證無 Axiom/Admitted。
* **接納契約 = CI**:`.github/workflows/ci.yml` 的四道閘門(fmt/clippy/test/doc、
  rocq build+reconcile、coverage≥90%、bench ±25%)是**唯一**驗收管道。

因為「正確性」由機器對帳與 kernel 驗證擔保,而非社群審查,
所以本庫**不需要**公開議題/PR 的社群討論通道 —— 開放反而增加雜訊與供應鏈風險。

---

## 1. 明令禁止(Forbidden)

| 行為 | 政策 | 說明 |
|---|---|---|
| **開 issue** | ❌ 禁止 | 不設 public issue 討論。有安全疑慮請走 §2 的私下通道。 |
| **發 PR(pull request)** | ❌ 禁止 | 任何外部/內部 pull request 都不予處理。見 §3 的例外。 |
| **fork 後提交** | ❌ 禁止 | fork 本庫以供參考/學習**可以**,但**不得**將其變更回送上游。 |

> **例外(Dependabot)**:由 `.github/dependabot.yml` 驅動的**第一方自動化更新**、
> GitHub Dependabot 依賴升級 PR,以及 Workflow 觸發的 bot 提交,是唯一的 PR 來源。
> 這是**自動維護**,不是外部貢獻。若要完全關閉,把該檔 `open-pull-requests-limit` 設為 `0`。

---

## 2. 安全疑慮 / 勘誤(私下通道)

若發現**安全**或**正確性**問題(例如 reconcile 對不上、定理有漏洞、bench gate 失效):

1. **不要在公開處張貼**可被利用的細節。
2. 以**不公開**的方式聯絡維護者(例如 GitHub 的 Security advisories / private vulnerability
   report,或專案指定的電子郵件)。
3. 描述「最小重現 + 預期 vs 實際」,附上最精簡的觸發(可復用到 `examples/` / `tools/`)。

我們會按嚴重度處理,並在 `docs/ROCQ-TRACE.md` / `docs/STATUS-*.md` 記錄。

---

## 3. 開發與提交(僅限內部/自動化)

本庫的變更**只來自**(a)內部維護者直接推送,(b)自動化(bot / Dependabot / CI)。流程:

1. **不經 PR**:變更直接提交到對應分支(如 `p2.1`),由 CI 驗收。
2. **分支命名**:語意化小數點遞增(`p2` → `p2.1`)或任務碼(如 `ci-gate`)。避免長壽分支。
3. **提交訊息**:一句話「做了什麼 + 為何」(原因導向),不超過 72 字元;重大變更附 `docs/` 記錄。
4. **本機先行**:推送前,在本地執行與 CI 相同的階段:
   `cargo fmt --all --check`、`cargo clippy --all-targets -- -D warnings`、
   `cargo test --all`、`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`、
   `make -C rocq`、`make -C rocq reconcile`、`python3 tools/cov_gate.py target/coverage.lcov`。
5. **驗收閘**:以上任何一項紅 → 退回修正;不得以「CI 環境差異」為由豁免。

---

## 4. 環境(本地重現)

* 工具:Rust 1.98+ / rustfmt / clippy;Rocq(Coq)8.20 + MathComp 2.3;python3;maude(選配,外部交叉驗證)。
* 一鍵初始:`sh scripts/setup_dev.sh`(apt 需 sudo;Rust 工具鏈重裝約 10 秒)。
* 注意:沙箱/CI 快照**不保留** `~/.rustup`、`/usr/local` 下的工具;每次請重跑 setup。

---

## 5. 風格與契約

* 雙語註記:適度中文(專案習慣),核心契約/公有 API 附英文。
* `#[warn(missing_docs)]` 已開啟:新公有項需文件。
* 九律矩陣(L1–L9)是規格**不是**建議:任何改動不得使其退縮;reconcile 以 `tools/rocq_reconcile.py`
  對 Rust 執行期事實與 Rocq 鏡像做逐點對照,是「捷徑為假」的證物。

---

## 6. 結語

> 想貢獻?最好的「貢獻」是:**指出一條**能被機器驗證的**反例或缺口**,而不是發起爭論。
> 若你對論文/形式的想法有可行證據,請先自行在本機的 `examples/` / `tools/` 建立最小驗證,
> 再走 §2 的私下通道 —— **以證據代替議題**。
