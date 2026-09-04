# 序列圖(Sequence Diagram)

> **Mermaid** 語法。兩個場景:
> 1. **CI 驗收管道** —— 推送 `p2.1` 後 GitHub Actions 依序執行四道閘門。
> 2. **交叉對帳(自我驗證)** —— `tools/rocq_reconcile.py` 把 Rust 執行期事實與 Rocq 鏡像逐點對照。
>
> 已用 `npx @mermaid-js/mermaid-cli` 渲染驗證(語法綠)。

## 1. CI 驗收管道(推送 p2.1)

```mermaid
sequenceDiagram
    autonumber
    participant Dev as 維護者
    participant GH as GitHub
    participant A as Actions runner
    participant C as Cargo / rustc
    participant Q as Rocq + MathComp
    participant P as gate 工具(python3)

    Dev->>GH: git push origin p2.1
    GH->>A: dispatch workflow (on: push)
    A->>A: concurrency cancel-in-progress
    A->>A: permissions contents read(最小權限)
    par verify job
        A->>C: cargo fmt --all --check
        A->>C: cargo clippy --all-targets -D warnings
        A->>C: cargo test --all(九律具名矩陣)
        A->>C: cargo doc --no-deps(-D warnings)
        C-->>A: 通過
    and rocq job(平行 runner)
        A->>Q: make -C rocq
        A->>Q: make -C rocq reconcile
        Q-->>A: 通過(無 Axiom/Admitted)
    and coverage job
        A->>C: cargo llvm-cov --lcov
        A->>P: python3 tools/cov_gate.py coverage.lcov
        P-->>A: core\geq90, ast\geq75
    and bench job
        A->>C: cargo bench --bench hotpaths --reps 11
        A->>P: python3 tools/bench_gate_selftest.py
        A->>P: python3 tools/bench_gate.py --tol 0.25
        P-->>A: 通過
    end
    A-->>GH: 狀態徽章(status check)
    GH-->>Dev: CI 綠(驗收合格)
```

> 註:`par / and` 表示四道閘在**各自的 runner 平行執行**(`runs-on` 各自獨立);
> 任一閘紅 → 整體失敗,即「驗收不合格」。

## 2. 交叉對帳(自我驗證;reconcile)

> 對應 `make -C rocq reconcile` ⇒ `tools/rocq_reconcile.py`。
> 對帳的 19 個「Rust 事實」直接在 Rust 側具名測試重算,再由 Rocq `vm_compute` 複驗 `=`。

```mermaid
sequenceDiagram
    autonumber
    participant RUST as Rust 可執行載體
    participant MIR as Mirror.v(Rocq 鏡像)
    participant REC as tools/rocq_reconcile.py
    participant COQ as coqc(Rocq 内核)

    RUST->>REC: 產出 19 個執行期事實(appli / count / measure / red_edges …)
    REC->>MIR: 生成 reconcile_gen.v(把事實寫成命題)
    REC->>COQ: Require Import + 複驗
    COQ->>COQ: vm_compute 重算事實
    COQ-->>REC: 命題全部相符(OK)
    REC-->>RUST: 全部樣本點一致(kernel 複驗通過)
    Note over RUST,COQ: 「捷徑為假」的證物：任何「他人 cut_for 不變」等捷徑，在此會被逐點拆穿。
```

## 3. 附註

* 兩個圖都屬**同步/依序**描述;`par / and` 僅反映 CI 的 job 平行。
* 若要驗證 Mermaid 語法,本地跑 `npx @mermaid-js/mermaid-cli -i <檔>.mmd -o out.png`(需系統 `libnss3 libnspr4` 等,見 `docs/ARCHITECTURE.md` §3)。
