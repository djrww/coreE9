# cl0r0

> **CL0 / R0 雙載體** — 表面語法樹 + 九條定律 (L1–L9) + 機械自証。
>
> Built in **Rust** · **zero dependencies** · verified by a **Rocq (Coq) mirror** + machine cross-check.

[![CI](https://github.com/djrww/coreE9/actions/workflows/ci.yml/badge.svg)](https://github.com/djrww/coreE9/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`cl0r0` 是「可執行載體 × 可證件載體」的代碼庫:以 Rust 把 CL0/R0 的語義與**九條定律**寫成可執行、可測試、可基準的庫;
再以一份 Rocq (Coq) 鏡像把同一語義寫成**定理**,由 `tools/rocq_reconcile.py` 逐點對帳。**正確性由機器擔保,不是由人擔保。**

---

## 1. Overview

### 1.1 Rust

cl0r0 以 **純 Rust、零依賴** 實現(見 `Cargo.toml`,`[dependencies]` 為空)。選擇 Rust 的原因:

- **速度與安全兼得**:無 GC、無運行時、裸指針隔離;熱點內核由 `benches/hotpaths.rs` 做 best-of-n 基準。
- **可當庫也可當工具**:`lib.rs` 輸出庫,`bin/cl0r0`、`bin/l9newman`、`bin/fuzz`、`bin/demo`、`bin/lsp` 輸出工具。
- **機械自証的載體**:`#[warn(missing_docs)]` 開啟;測試矩陣(L1–L9 具名測試)即規格。

六層基礎 → 九律 → 雙載體規格:

| 層 | 模組 | 責任 |
|---|---|---|
| 位址/區間 | `span` | σ(v) = [a,b) 半開區間 |
| 詞法 | `lex` | CL0 DFA,trivia 保留 |
| 語法 | `parse` | 表面語法樹 + 增量重析 + ERROR 全化 |
| 樹性質 | `tree` | 連續性公理、laminar、CW 復形 |
| 語義面 | `ast` | liveness 三軌 + 衝突圖(區間圖 ⊂ 弦圖 ⊂ 完美圖) |
| 編輯 | `edit` | 位移函數、複合、結合律 |
| 生成 | `gen` | 合法 + 髒輸入生成器 |
| 重寫 | `rep` | 修法菜單(L8 遞減測度、L9 合流) |
| R₀ | `r0` | R₀ Rust 子集 + unsupported 申報 |
| 終止/合流 | `l9newman` | 機械 Newman 通道 |
| 反例工程 | `shrink` | ddmin 最小反例 |

> 輔助可執行檔(`bin/`):`cl0r0 --check <file>|-`(僅診斷)、`l9newman`(合流通道)、`fuzz`(屬性測試 + 縮小)、
> `demo`(watch 層原型:一次按鍵 = 一次增量重析)、`lsp`(最小診斷 LSP,Content-Length stdio)。

### 1.2 Next Generation Engine

cl0r0 不是普通解析器,而是一台 **下一代的原始碼工具引擎** (next-generation source-tooling engine)。它把「語法樹」當成常駐、可增量、可驗證的資料結構,而不是一次性丟棄的輸出:

- **增量重析** (`parse::reparse` + `ReuseData`/`ReparseOut`):一次編輯只重算受影響子樹;`reparse` 的「增量 ≡ 全量」結構等價由測試自動鎖死(`test_reparse_tree_structurally_sound`)。
- **編輯單體 M5**:並行編輯去抖歸併(`compose_seq`);由 `bin/demo` 呈現「一次按鍵 = 一次增量重析」。
- **常駐診斷層**:`bin/lsp` 把 span 族 + ERROR 節點輸出成 `publishDiagnostics`,讓編輯器即時看到語法錯誤與語義紅邊警告。
- **自証引擎**:同一份語義在 Rocq 鏡像中寫成**定理**,由 `make -C rocq reconcile` 對 Rust 執行期事實逐點複驗 —— 引擎的每一步「為什麼對」都有證件。
- **合流通道**:`l9newman` 機械地走「終止 + 局部合流 ⇒ 合流 ⇒ 唯一正規形」,把重寫系統的收斂性變成可執行的性質。

引擎的「輸入宇宙」由 `gen`(合法/髒輸入生成器)供給,失敗時由 `shrink`(ddmin)縮到最小反例並進 `tests/fixtures/` 防回歸。

---

## 2. Highlights

### 2.1 Features

- **九條定律 (L1–L9)** 具名測試矩陣:連續性、位址映象、增量重析、編輯單體、liveness、混合、全化、遞減測度、合流;另有 M1/M2/M4/M5、T2、R₀ 語義面、反例縮小。任何改動若使定律退縮,CI 即紅。
- **雙載體自我驗證**:Rust 執行層 + Rocq 鏡像;`make -C rocq reconcile` 產出 19 個對帳點,由 `vm_compute` 重算複驗 `=`,把「捷徑為假」逐點拆穿(見 `docs/ROCQ-TRACE.md`)。
- **定理級合流**:`R3_ct_wcr_raw` → `R3_ct_wcr` → `R4_ct_confluent`,落地於 `rocq/theories/R3Guard.v`;`Print Assumptions` 全部無 Axiom/Admitted。
- **外部交叉驗證**:Maude 對 CT 菜單宇宙級性質 + NaTT 對 R1 過度近似超集 TRS 的終止證明(`tools/run_external_checks.sh`,按 R3-RESEARCH §三採納為測試對照)。
- **純工具鏈交付**:`cargo build --release` + `bin/*`;Rocq 側 `make -C rocq` + `make clean`。
- **CI 作為驗收契約**:`.github/workflows/ci.yml` 四道閘(fmt/clippy/test/doc、rocq build+reconcile、coverage≥90%、bench ±25%)。

### 2.2 Fast, Stable, 擴充

**Fast.** 零依賴、無 GC、無運行時;熱點內核(`lex`/`parse`/`laminar_ok`/`newman_check`)以 **best-of-n** 基準衡量(見 `bench/BASELINE.json`),CI 以 ±25% + `null` 漂移校正做回歸門,並先自測 gate 的判別力(`tools/bench_gate_selftest.py`)以免「修到永遠綠」。

**Stable.** 正確性不靠運氣:
- 九律具名測試矩陣 + 屬性測試(fuzz)+ 反例最小化進 fixture。
- Rocq 鏡像與 Rust 執行期事實經 `reconcile` 逐點對帳;kernel 驗證缺 Axiom。
- CI 四道閘 + Dependabot 依賴維護 + 封閉貢獻模型(見 `CONTRIBUTING.md`)。

**擴充 (Extensible).** 引擎是「可成長」的:
- 菜單可換:`CommutativeTrim` / `Naive`;政策可換:`Guarded` / `Raw`(鏡像見 `Mirror.v`)。
- 事件宇宙由 `enumerate_states` 參數化(可推 4 事件 × 6 座標)。
- 定律可逐條激活:L3/L4 依規格排除,但 `reparse`/`compose` 基礎設施就緒,解除後只需補等價測試。
- R₀ 載體逐步擴充:詞法已綠,`r0_parse`(CST 樹產出)為下一站。
- 可對外庫化:作為 crate 使用,已有 `lib.rs` 公有面、`bin/*` 與文檔首頁。

---

## 3. Quickstart

```bash
# Rust 側:建構 + 工具
cargo build --release
cargo run --bin cl0r0 -- --check <file>|-
cargo test --all

# Rocq 側:定理庫 + 對帳(生 reconcile_gen.v 並複驗)
make -C rocq
make -C rocq reconcile

# 外部交叉驗證(選配;需 maude、NaTT、z3)
sh tools/run_external_checks.sh
```

## 4. Verification & Engineering

| 管道 | 命令 | 結果 |
|---|---|---|
| 格式化 | `cargo fmt --all --check` | 全綠 |
| 靜態 | `cargo clippy --all-targets -- -D warnings` | 0 警告 |
| 測試 | `cargo test --all` | 九律矩陣 + 屬性測試 |
| 文檔 | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | 0 警告 |
| 證件 | `make -C rocq` && `make -C rocq reconcile` | 6 定理庫 + 19 對帳點 |
| 覆蓋率 | `cargo llvm-cov --lcov` + `python3 tools/cov_gate.py` | core ≥ 90% / ast ≥ 75% |
| 基準 | `cargo bench --bench hotpaths` + `python3 tools/bench_gate.py` | ±25% 回歸門 |

> 工程細節、路線圖與誠實缺口見 `ROADMAP.md`、`docs/`；架構與序列圖見 `docs/ARCHITECTURE.md`、`docs/SEQUENCE.md`；
> 決策紀錄見 `docs/adr/`。

## 5. Licensing & Contribution

- **License**: MIT(見 `LICENSE`,若已存在;否則由維護者決定)。
- **貢獻**:本庫為**封閉自証**模型 —— 不開放 issue / PR / fork;安全與正確性疑慮請走私下通道。
  詳見 [`CONTRIBUTING.md`](CONTRIBUTING.md) 與 [`docs/adr/0002-closed-contribution-model.md`](docs/adr/0002-closed-contribution-model.md)。
  唯一的 PR 來源是 Dependabot 的第一方自動化依賴維護。
