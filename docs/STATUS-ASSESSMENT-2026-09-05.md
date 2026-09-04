# cl0r0 專案現況評估 / 技術債對帳 / 後續建議 — 2026-09-05

> 本次為**跨環境重建後的全量驗證**：快照不保留工具鏈（Coq、Rust）與 `.git` 歷史，
> 故先以 `scripts/setup_dev.sh` 重建環境，再對**全部內部門檻**逐一重跑，據實打分。
> 驗證日期：2026-09-05。環境：Coq 8.20.1 / mathcomp 2.3.0 / rustc 1.98.1 / cargo 1.98.1。

---

## 0. 摘要（TL;DR）

- **架構**：`cl0r0` 是一個**零依賴**、機械自證的雙載體代碼庫（CL0 九律載體 + R₀ 實用載體），
  並以 **Rocq/Coq** 形式化把「測試級」定律升為「定理級」。
- **驗證矩陣全綠**（本輪實跑）：`cargo fmt --check` ✅、`cargo clippy --all-targets -D warnings` 0 ✅、
  `cargo test --all` **51/51** ✅、`RUSTDOCFLAGS=-D warnings cargo doc --no-deps` 0 ✅、
  `make -C rocq` 五理論全建 ✅、`make -C rocq reconcile` 19 樣本點 kernel 複驗 ✅。
  主要定理 `Print Assumptions` 全部「Closed under the global context」（**無 Axiom / Admitted**）。
- **本輪已清除的技術債**：CI gate 的 `cargo fmt` 破壞（6 檔）、`setup_dev.sh` 的 sudo 所有權 bug、
  14MB 發布產物未 gitignore、雜散 scratch 產物、文檔過時計數（47/47 → 51/51）。
- **尚未閉環的核心**：`R3_ct_wcr`（Guarded WCR）與 `R4_ct_confluent` 仍未證 ——
  已收斂到「嚴格紅邊遞減」的最後 4 個小步（見 §3.5 與 §5-P0）。

---

## 1. 六維現況評估（打分）

| 維度 | 分數 | 證據 | 失分理由 |
|---|---|---|---|
| **代碼質量** | **9.0/10** | 零依賴；fmt/clippy/doc 全 0；51/51 測試；無 `todo!`/`unimplemented!` | `parse.rs` 1,504 行、`r0.rs` 2,243 行（單檔偏大）；`ast.rs` 覆蓋 76.9%（豁免）；內部不變量處用 `panic!`（可接受，但屬防禦式） |
| **九律覆蓋** | **8.5/10** | L1–L9 + M1/M2/M4/M5 + T2 + R₀ 語義面 + shrink 皆具名測試（`tests/laws.rs` 27 個 `fn test_law_*` + `r0.rs` 8 + 其他） | ROADMAP.md §一「誠實缺口」仍開放：L7 全化無**獨立**具名測試（被合併吸收）、R₀ 無完整 `r0_parse`（CST）、L9 反例通道是「機器找反例」非「機器證明」 |
| **重寫系統 / Newman 通道** | **9.0/10** | §4.2 良基測度 µ；§4.3 臨界對/L9 Newman 通道（Naive 反例對照 + L9b′ 精確交換）；Rust 窮舉 **623,616 狀態 × 635,424 臨界對 0 違反**；`l9newman` 模組 | 反例通道非「∀ 狀態」層級（由 Rocq 補，見 §1.5） |
| **增量 / 編輯器應用**（P2 #9 #10） | **9.0/10** | `bin/demo`（一次按鍵=一次增量重析）+ `bin/lsp`（JSON-RPC pubDiag）+ `bin/cl0r0 --check`（exit 0/1/2）；3 條集成測試；曾抓出並修復 `clone_subtree` 自環與 `uri_of` 二次索引兩 bug | 僅 demo 級；LSP 只做診斷，未做補全/重命名/跳轉 |
| **Rocq 形式化**（P3 #12） | **7.0/10** | 完成：鏡像 `Mirror.v`（19 點 reconcile）、抽象 Newman（`newman`/`newman_unf`/`exists_normal_form`）、具體 SN（`R2_sn_step_ct`）、**Raw WCR**（`R3_ct_wcr_raw`）、§⑤ 紅邊幾何+邊結構+無重複+子集+單調（全部 kernel 封閉） | **`R3_ct_wcr`（Guarded）與 `R4_ct_confluent` 開放**；且「未 commit 的工作在快照 .git 遺失時會流失」的風險仍存在 |
| **工程化 / 可重現性** | **7.5/10** | 完整 CI（4 job：verify/rocq/coverage/bench）；`setup_dev.sh` 一鍵重建；四層門檻（fmt/clippy/coverage/bench）；bench gate 有判別力自測 | 外部交叉驗證（Maude/NaTT/z3）**不進 CI** 且本環境未裝；`target/` 284MB、`release-v0.1.1` 14MB 無版本管理；`.git` 不持久 ⇒ 歷史易流失 |

**加權綜合約 8.3/10**（重寫+形式化+代碼質量為主軸）。核心「機器自證」的骨架（測試層 51/51 + Rocq 定理層 5 理論全封閉 + 對帳 19 點）是**真實且可重建**的，不是一次性成果；唯一的重大懸而未決項是**全空間 Guarded 合流**。

---

## 2. 本輪「除錯 + 清債」清單（已完成）

| # | 項目 | 破壞面 | 處置 |
|---|---|---|---|
| D1 | `cargo fmt --all --check` 失敗 | **CI `verify` job 會紅**（`cl0r0.rs` / `diag.rs` / `lsp.rs` / `editor_tooling.rs` / `laws.rs` 6 檔） | `cargo fmt --all`（純排版改動：換行/括號；`src` 約 27 行，`tests` 約 8 行）；重跑 `--check` 通過 |
| D2 | `setup_dev.sh` 以 sudo 執行 → rustup 以 **root** 所有權寫入用戶 `$CARGO_HOME`/`$RUSTUP_HOME` | 之後**非 root** 的 `cargo`/`rustup` 寫 `settings.toml` 或 `component add` → `Permission denied`；且 `rustup default stable` 報「no default configured」 | 在 root 上下文補 `chown -R "$SUDO_USER":"$SUDO_USER" "$REAL_HOME/.cargo" "$REAL_HOME/.rustup"` |
| D3 | `release-v0.1.1/`（14MB，95 檔，含 4MB×3 二進位 + rustdoc HTML）未 gitignore | 版本庫常駐 14MB 二進位/文檔產物 | 加入 `.gitignore`；建議改以 **GitHub Releases 資產**交付 |
| D4 | 雜散 scratch：`rocq/theories/TmpEdge.glob`、`TmpPA.glob` | 早期調試殘留產物 | 刪除 |
| D5 | 文檔過時計數：`47/47`、`15 單元+32 集成` | 與實際 51/51 不符 | 更正為 `15 單元 + 36 集成 = 51/51`（`R3-NEXT-STEP.md`、`ROCQ-TRACE.md`） |
| D6 | `R3-NEXT-STEP.md` §四仍把 `in_red_char`/`red_edges_aux_nodup` 標為 ⬜ | 與已驗證事實不符 | 更新為 ✅（並補 `red_p_not_overlap`/`trim_at_preserves_ids`/`trim_at_nth_here`） |

> 上述 D1–D6 已在本輪落地。ROCQ ／ Rust 門檻全部重跑、並補齊 `setup_dev.sh` 的 `chown` 後，
> **同一條命令流可在新環境直接重現**。

---

## 3. 詳細進度對帳

### 3.1 代碼庫規模
- Rust：`src/`+`examples/`+`tests/` ≈ **10,605 行**；15 個庫模組 + 5 個 bin。
- Rocq：`rocq/theories/` 5 個理論 ≈ **2,037 行**。
- 工具/腳本（`tools/`+`scripts/`，py/sh/maude）≈ 2,880 行。
- 文檔：11 份 `docs/*.md`。

### 3.2 Rust 門檻（全綠）
- `cargo fmt --check` ✅（D1 修後）
- `cargo clippy --all-targets -- -D warnings` ✅ 0
- `cargo test --all` ✅ **51/51**（src 單元 15 + `tests/laws.rs` 33 + `tests/editor_tooling.rs` 3）
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` ✅ 0
- `cargo build --bins`（cl0r0 / l9newman / fuzz / demo / lsp）✅

### 3.3 Rocq 定理庫（全建 + 無公理）
| 理論 | 內容 | 狀態 |
|---|---|---|
| `Mirror.v` | Rust↔Rocq 鏡像（19 樣本點对帳） | ✅ |
| `WCRUtil.v` | 修剪語義、`ct_pred`、`cut_for_gt_start`、紅邊幾何 + §⑤ | ✅（含 §⑤ 全封閉） |
| `AbstractArs.v` | `star/joinable/wcr/confluent/nf/sn` + `newman`/`newman_unf`/`exists_normal_form` | ✅ |
| `ConcreteSN.v` | `R2_sn_step_ct`（CT×Guarded 強的 step_ct 強正規化）+ `ct_guarded_has_nf` | ✅ |
| `ConcreteWCR.v` | 不變量機器 + 交換/規則存活 + **`R3_ct_wcr_raw`** | ✅（Guarded 版未建） |

`Print Assumptions R3_ct_wcr_raw / R2_sn_step_ct / r1_trim_red_le / red_edges_aux_nodup / red_edges_aux_trim_subset / in_red_char / cut_for_gt_start / ct_cut_min_drops` **全部 "Closed under the global context"**。

### 3.4 §⑤ 紅邊結構（本輪已完成——嚴格遞減的支援件）
- `r1_trim_red_le`：剪一事件 ⇒ 紅邊計數**不增**（單調，幾何主體）。
- `red_edges_aux_trim_subset`：**剪後邊集 ⊆ 剪前邊集**（逐點 `red_p_trim_left/right` + `mem_trim_at_pos`）。
- `red_edges_aux_nodup`：`NoDup (map ev_id l) → NoDup (red_edges_aux l rt)`（**紅邊清單無重複**）。
- `in_red_char`（強化）：邊由**相異**兩事件產生且互紅。
- `in_red_edges_aux` / `red_p_sym` / `pk_minmax` / `nodup_map_ev_id_inj` / `NoDup_map_inj`：無關順序的配邊、唯 id ⇒ 同事件。
- `red_p_rt_ct_pred`：候選 `b`（`ct_pred a b`）+ runtime 空 ⇒ 是 a 的**紅邊**。
- 剪作件：`red_p_not_overlap` / `trim_at_preserves_ids` / `trim_at_nth_here`。

### 3.5 尚缺（`R3_ct_wcr` / `R4_ct_confluent` 的最後一哩）
仍只需把 `r1_trim_red_le` 的**單調 `≤` 升為嚴格 `<`**：
1. `pk_comp_in_ab`：`pk a b = pk x y → 分量 id 屬 {id_a, id_b}`；
2. 候選 `b0` 的「剪後消失」：`~ In (pk a b0) (red_edges_aux (trim_at l p c) rt)`（`i_overlap` 消失 ⇒ `red_p … = false`）；
3. `length_lt_nodup`：`NoDup l1 → NoDup l2 → incl l1 l2 → (∃x, In x l2 ∧ ~In x l1) → length l1 < length l2`；
4. 組裝 `r1_apply_red_edges_strict` → `ct_guard_redundant`（可達類，`st_runtime=[]`）→ `step_ct ≡ step_ct_raw` → `R3_ct_wcr` → `R4_ct_confluent`。

---

## 4. 技術債「殘留」清單（未清，供後續排序）

| 優先 | 債項 | 影響 | 建議 |
|---|---|---|---|
| H | **無持久 `.git`／未 commit**：快照不保留 git 歷史，上輪 commit 隨 .git 消失 | 版本歷史/回溯能力流失；「哪個狀態被驗證過」難以追蹤 | 立即 `git init` + 首次 commit + **推到遠端**；每次里程碑即 commit |
| H | **`R3_ct_wcr` / `R4_ct_confluent`** | 核心學術宣稱未達（∀ 狀態合流） | 續 §3.5 最後 4 步；若卡 >4 人日讀 `HARD-ITEMS #1` 的備援（peak-decreasingness / 有限反射證書） |
| M | `parse.rs` 1,504 行 / `r0.rs` 2,243 行 | 可維護性 | 依模組職責拆 `mod`；非這次重構重點 |
| M | `ast.rs` 覆蓋 76.9%（**豁免** 75%） | 語義面行覆蓋偏低（報告工具失真 + 保留槽） | 加 30+ 語法樣本矩陣（`test_law_semantic_extract_breadth` 既有）；把豁免原因寫進 COVERAGE.md |
| M | L7 全化無**獨立**具名測試 | 「任何輸入皆出樹」缺直接斷言 | 重建 `test_law_L7_error_totalization` |
| M | R₀ 無完整 `r0_parse`（CST） | R₀ 載體「樹級」未完成 | 依附錄 B 補 CST；`unsupported` 精確到節點 |
| L | 外部交叉驗證（Maude / NaTT / z3）不進 CI 且本環境未裝 | 第三方佐證非自動 | 以 GitHub Actions 裝 maude；NaTT 需編譯（可選） |
| L | `target/`（284MB）、`release-v0.1.1`（14MB）、roqc `.vo/*.aux/*.glob` 殘餘 | 磁碟/倉庫肥大 | `target/` 已 gitignore；`release-v0.1.1` 已 gitignore（D3）；rocq 產物由 `make clean` 管 |

---

## 5. 後續開發建議（按優先序）

### P0 — 閉環形式化（當前主軸，最高價值）
1. 完成 §3.5 的 4 步 → `R3_ct_wcr`（Guarded）→ `R4_ct_confluent` → `confluent step_ct`。
2. 同時把「未 commit 即流失」列為硬性紀律：**每達成一個 kernel 封閉的引理就 commit 一次**。

### P1 — 工程紀律（讓「自證」成為持續綠，而非一次性）
3. `git init` + 推遠端；`.gitignore` 已更新（`release-v0.1.1/`）。
4. 補 `test_law_L7_error_totalization`，補 R₀ `r0_parse`（CST）。
5. 在 CI 增一 job 用 `tools/run_external_checks.sh` 裝 maude 做交叉對帳（可選，naTT 線外）。

### P2 — 應用化（若想被真正使用）
6. LSP 從「僅診斷」向上：補全 / 跳轉 / 重命名；demo 接真實編輯器。
7. `cargo publish` 庫化（metadata + 文檔首頁示例），版本語義 0.2 起。

### P3 — 價值敘事（學術 / 對外）
8. 完成 R3/R4 後，產出「Rust 窮舉 0 違反 ∧ Rocq 定理 0 公理」的對照報告（`SPEC-TRACE` R3 行 ⬜→✅，保留 635,424 計數作證人）。
9. 把 `tools/rocq_reconcile.py` 從固定樣本改成 `gen.rs` 接線的**隨機抽樣差分**（`HARD-ITEMS #2`），降低鏡像漂移風險。

---

## 6. 結論

這是一個**紮實、可重建、非一次性的自證系統**：
- 測試層 51/51、靜態/文檔 0 warning、Rocq 定理層 5 理論全封閉、鏡像对帳 19 點。
- 唯一的重大空白是「∀ 狀態 Guarded 合流」，且已收斂到可執行的最後 4 步，不是未知的深淵。
- **最大風險不是數學，而是工程**：git 歷史不持久、外部驗證不自動、`target/`/`release` 肥大。這些在 P0 完成前也值得先修（至少 commit + 推到遠端）。
- 誠實申報：外部交叉驗證（Maude/NaTT）本環境**未裝未跑**；coverage/bench 數字沿用 `docs/COVERAGE.md` 與 `docs/BENCH.md` 的既有紀錄，本輪**未重採集**。
