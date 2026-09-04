# 架構圖(Architecture Diagram)

> 使用 **Mermaid** 語法。在支援 Mermaid 的檢視器(GitHub、VS Code、Typora、Obsidian 等)中會渲染成圖;
> 在純文字/靜態預覽中顯示為原始碼。字型/繪圖引擎隨檢視器而異,結構以本檔為準。

## 1. 雙載體總覽(flowchart)

```mermaid
flowchart TB
    subgraph RUST["載體一 · Rust 可執行層 (crate cl0r0)"]
        direction TB
        L["lex：CL0 詞法 DFA（trivia 保留）"]
        P["parse：表面語法樹 + 增量重析 + ERROR 全化"]
        T["tree：連續性公理 / laminar / CW 復形"]
        A["ast：liveness 三軌 + 衝突圖（區間圖 ⊂ 弦圖 ⊂ 完美圖）"]
        E["edit：編輯單體（位移函數 / 複合 / 結合律）"]
        G["gen：合法程式 + 髒輸入生成器"]
        R["rep：修法菜單重寫系統（L8 遞減測度、L9 合流）"]
        R0["r0：R₀ Rust 子集（附錄 B 覆蓋面契約）"]
        N["l9newman：機械 Newman 通道（終止 + WCR ⇒ 合流 ⇒ 唯一正規形）"]
        S["shrink：ddmin 反例最小化"]
        BINS["bins：cl0r0 / l9newman / fuzz / demo / lsp"]
    end

    subgraph ROOQ["載體二 · Rocq (Coq) 證件層 (Cl0r0.*)"]
        direction TB
        M["Mirror.v：鏡像語義（步驟 / 菜單 / 紅邊 / 測度）"]
        W["WCRUtil.v：修剪幾何 + 紅邊組合（③⑤⑥）"]
        AA["AbstractArs.v：抽象 Newman（sn + wcr ⇒ cr / unf）"]
        SN["ConcreteSN.v：R2 強正規化（step_ct 每步 µ 遞減 ⇒ SN）"]
        WCR["ConcreteWCR.v：R3 Raw 局部合流（精確交換）"]
        G3["R3Guard.v：Guarded 合流 + R4 可達類合流（新）"]
    end

    subgraph CI["驗證閘門 · GitHub Actions (.github/workflows/ci.yml)"]
        direction LR
        V["verify：fmt → clippy → test → doc"]
        RC["rocq：make + reconcile（交叉對帳）"]
        CV["coverage：core ≥ 90% / ast ≥ 75%"]
        BN["bench：hotpaths best-of-n ±25%"]
    end

    L --> P --> T --> A
    A --> E
    E --> G
    G --> R
    R --> R0
    R0 --> N
    N --> S
    BINS --> R

    M --> W --> AA --> SN --> WCR --> G3

    R -. "reconcile：Rust 執行期事實 ↔ Rocq 鏡像逐點對帳" .-> RC
    G3 -. "定理/推論接龍參考" .-> RC

    BINS --> V
    R --> V
    R --> CV
    R --> BN
    RC --> V
```

## 2. 語意分層（由上而下）

| 層 | 載體 | 責任 | 對應契約 |
|---|---|---|---|
| 語法面 | Rust `lex`→`parse`→`tree` | 表面語法樹、增量重析、連續性 | §1–§2.3 |
| 語義面 | `ast` | liveness 三軌、衝突圖（完美圖） | §3.2–3.3 |
| 變換面 | `edit`→`gen`→`rep` | 編輯單體、生成器、修法菜單 | §2.1 / §4 |
| 規約面 | `r0` | R₀ 子集覆蓋面 | 附錄 B |
| 終止/合流 | `l9newman` | Newman 通道（定理級） | §5–§6 |
| 反例工程 | `shrink` | ddmin 最小反向例 | §8 |
| 鏡像/證件 | `rocq/theories/` | 語義鏡像 + 定理，kernel 驗證 | Phase 0–3 |
| 閘門 | `ci.yml` | 可執行驗收（四道閘） | 本檔 §1 |

## 3. 為什麼用 Mermaid

* **純文字可版本化**:與 `docs/` 其他 `.md` 一樣進 `git` 可 diff,無需外部繪圖工具。
* **不依賴二進位**:任何支援 Mermaid 的檢視器即時渲染;CI 無需額外安裝。
* **語法可驗證**:可用 `npx @mermaid-js/mermaid-cli` 或 `.github/workflows/` 中的靜檢工具做「語法綠」。
