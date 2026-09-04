# cl0r0

CL0 / R0 雙載體：Rust 執行層 + Rocq (Coq) 鏡像，機器自証。

## 1. Overview

### 1.1 Rust

以純 Rust 實現，零依賴（`Cargo.toml` 無第三方 crate）。語義、九條定律（L1–L9）、測試與基準全部落在 Rust 側；可作庫（`lib.rs`）或工具（`bin/`）。

### 1.2 Next Generation Engine

cl0r0 是 source-tooling engine：語法樹為常駐、可增量重析、可驗證的資料結構。增量重析、編輯單體 M5、常駐診斷（LSP）、自証引擎、合流通道；同一份語義另有 Rocq 鏡像，由 `tools/rocq_reconcile.py` 逐點對帳。

## 2. Highlights

### 2.1 Features

- 九條定律（L1–L9）具名測試矩陣。
- 雙載體自我驗證：Rust 執行層 ↔ Rocq 鏡像。
- 定理級合流（`R3_ct_wcr` → `R4_ct_confluent`），無 Axiom / Admitted。
- CI 四道閘作為驗收契約。

### 2.2 Fast, Stable, 擴充

**Fast**：零依賴、無 GC、無運行時；熱點以 best-of-n 基準衡量。
**Stable**：九律測試、fuzz、反例縮小、Rocq 對帳；覆蓋率 ≥ 90%、基準 ±25% 回歸門。
**擴充**：菜單／政策可換、事件宇宙參數化、定律可逐條激活，可作 crate 使用。

## 3. Installation

```bash
cargo build --release
cargo run --bin cl0r0 -- --check <file>|-
cargo test --all

make -C rocq
make -C rocq reconcile
```

> **Warning**：目前屬內部研究載體，介面與行為可能隨時更動，不承諾穩定 API。

## 4. Contribution（全部謝絕）

本庫為封閉自証模型：**不開放 issue / PR / fork**。正確性由機器擔保（Rocq 鏡像 + reconcile），不依賴外部貢獻；任何改動走內部通道。唯一自動化來源為 Dependabot。詳見 `CONTRIBUTING.md`。

## 5. License

MIT，見 `LICENSE`。
