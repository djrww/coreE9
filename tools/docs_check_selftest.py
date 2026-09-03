#!/usr/bin/env python3
"""`tools/docs_check.py` 的判別力自測(N2;2026-09-03 入庫)。

為什麼需要它:稽核器最危險的失效模式和門檻一樣,是**靜默失效** ——
文件被重整、句子被改寫,規則從此命中 0 次,而 CI 依然全綠。那時「文件一致性」
只剩一個 job 名字。

本腳本用**合成 STATUS.json + 合成文件**跑 9 個情境,每個都斷言確定的 exit code,
並掛在 CI `docs-consistency` job 的第一個檢查(與 `cov_gate_selftest.py` /
`bench_gate_selftest.py` 同一紀律:**gate 不只能變綠,必須仍能變紅**)。

情境分三組:
  A. 一致時必須綠(1);
  B. 文件數字與實測不符時必須紅(2–6):測試總數、宇宙規模、覆蓋率表、
     對帳樣本點、百分比;
  C. 稽核器自身的 fail-closed(7–9):規則命中 0 次、STATUS.json 缺席、
     以及「程式碼說謊時文件仍在宣稱」的條件式規則。

用法:python3 tools/docs_check_selftest.py
"""
import copy
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CHECK = os.path.join(HERE, "docs_check.py")
SCHEMA = "cl0r0-status/v1"


def base_status():
    return {
        "schema": SCHEMA,
        "generated_at_utc": "2026-09-03T00:00:00Z",
        "note": "selftest fixture",
        "toolchain": {"rustc": "rustc 1.98.0", "coqc": "8.20.1"},
        "tests": {"unit": 17, "integration": 33, "total": 50},
        "coverage": {
            "r0.rs": {"lf": 1480, "lh": 1337, "ratio": 0.9034},
            "ast.rs": {"lf": 415, "lh": 319, "ratio": 0.7687},
        },
        "rocq": {
            "files": 6,
            "lines": 1536,
            "admitted": 0,
            "axiom": 0,
            "reconcile_facts": 19,
            "theory_files": ["Mirror.v"],
        },
        "universes": {
            "newman_3x6": {"states": 35280, "critical_pairs": 10668},
            "newman_4x5": {"states": 105216, "critical_pairs": 100392},
            "newman_4x6": {"states": 623616, "critical_pairs": 635424},
            "enumerate_3x6_unfiltered": 74088,
            "enumerate_4x5_unfiltered": 810000,
        },
    }


def docs_for(st):
    """渲染一份**與 STATUS.json 完全一致**的最小文件集(覆蓋每條規則的錨點)。"""
    t, u, c, r = st["tests"], st["universes"], st["coverage"], st["rocq"]
    n36, n45, n46 = u["newman_3x6"], u["newman_4x5"], u["newman_4x6"]
    cov_rows = "\n".join(
        f"| `{m}`(§x) | {v['lh']}/{v['lf']} | {v['lh'] / v['lf'] * 100:.1f}% | ≥90% | ✅ |"
        for m, v in sorted(c.items())
    )
    return {
        "docs/COVERAGE.md": (
            f"> 執行面:全部庫測試(**{t['unit']}** 具名測試)"
            f"+ 集成矩陣(`tests/laws.rs`,**{t['integration']}** 條)= **{t['total']}**。\n"
            "\n| 模組 | 覆蓋 | 比例 | 閾值 | 狀態 |\n|---|---|---|---|---|\n"
            + cov_rows
            + "\n"
        ),
        "docs/SPEC-TRACE.md": (
            f"## 〇、測試全量清單({t['total']} 具名測試)\n"
            f"→ `cargo test --all`(上表 **{t['total']}** 具名測試即驗收合同)\n"
            f"| 3 | 4 事件 × 6 座標({n46['states']:,} 狀態 × "
            f"{n46['critical_pairs']:,} 臨界對,0 違反) |\n"
            f"`tools/rocq_reconcile.py`(kernel 複驗 {r['reconcile_facts']} 樣本點;\n"
        ),
        "ROADMAP.md": (
            f"`cargo test` **{t['total']}/{t['total']}** · "
            f"L1–L9 + T2 + R₀:**{t['total']} 具名測試**(見 SPEC-TRACE §〇)\n"
        ),
        "docs/R3-RESEARCH.md": (
            "| 宇宙 | 狀態數 | 單步 | 不同後繼之臨界對 | 探針結果 |\n|---|---|---|---|---|\n"
            f"| 3×6 + distinct-start 過濾(=CI 宇宙族) | {n36['states']:,} | — | "
            f"{n36['critical_pairs']:,} | 精確交換 |\n"
            f"| 4×5 + 過濾 | {n45['states']:,} | — | {n45['critical_pairs']:,} | 精確交換 |\n"
            f"| 4×6 + 過濾(**CI 現有規模**) | {n46['states']:,} | 1,053,672 | "
            f"{n46['critical_pairs']:,} | P1 違反 0 |\n"
            "```\n"
            f"cargo test --all                               # {t['total']} 具名測試\n"
            "```\n"
        ),
        "docs/ROCQ-TRACE.md": (
            "| 宇宙 | 狀態 | 不同後繼臨界對 | start 不變 | 精確交換 | Guarded≡Raw | 紅邊增加 |\n"
            "|---|---|---|---|---|---|---|\n"
            f"| 3×6(+過濾)| {n36['states']:,} | {n36['critical_pairs']:,} | 0 違反 | x | 0 | 0 |\n"
            f"| 4×5(+過濾)| {n45['states']:,} | {n45['critical_pairs']:,} | 0 違反 | x | 0 | 0 |\n"
            f"| 4×6(+過濾,=CI)| {n46['states']:,} | {n46['critical_pairs']:,} | 0 違反 | x | 0 | 0 |\n"
            f"L9b 窮舉(4×6 共 {n46['states']:,} 狀態 × {n46['critical_pairs']:,} 臨界對,0 違反)\n"
            f"(a)「46 具名測試」→ 實跑 `cargo test --all` = {t['total']}\n"
            "已 kernel 驗證(無 `Admitted`/`Axiom`,`make -C rocq` 全綠)\n"
        ),
        "docs/ROCQ-PLAN.md": (
            f"L9b 窮舉({n46['states']:,} 狀態 × {n46['critical_pairs']:,} 臨界對)\n"
            f"且本專案已有 {t['total']} 具名測試作為錨。\n"
            "**不含任何 `Admitted`/`Axiom`**\n"
        ),
        "docs/BENCH.md": (
            f"(623,616 狀態 × 635,424 臨界對)  ← 由 {n46['states']:,} 狀態 × "
            f"{n46['critical_pairs']:,} 臨界對取代\n"
        ),
        "docs/EXTERNAL-XCHECK.md": f"鏡像(`vm_compute` 複驗 {r['reconcile_facts']} 樣本點)。\n",
    }


def run(status, docs, extra=()):
    tmp = tempfile.mkdtemp()
    try:
        for rel, text in docs.items():
            p = os.path.join(tmp, rel)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            open(p, "w", encoding="utf-8").write(text)
        if status is not None:
            json.dump(status, open(os.path.join(tmp, "docs", "STATUS.json"), "w"))
        r = subprocess.run(
            [sys.executable, CHECK, "--root", tmp, *extra],
            capture_output=True,
            text=True,
        )
        return r.returncode, r.stdout + r.stderr
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def case(desc, status, docs, want):
    return (desc, status, docs, want)


def mutated_docs(st, rel, old, new):
    d = docs_for(st)
    assert old in d[rel], f"selftest 錨點失效: {rel} 找不到 {old!r}"
    d[rel] = d[rel].replace(old, new, 1)
    return d


def mutated_status(fn):
    st = base_status()
    fn(st)
    return st


CASES = [
    case("完全一致 → 綠", base_status(), docs_for(base_status()), 0),

    # ── B. 文件數字與實測不符,必須紅 ────────────────────────────────
    case(
        "文件寫 47 具名測試、實測 50 → 紅",
        base_status(),
        mutated_docs(base_status(), "docs/SPEC-TRACE.md", "(50 具名測試)", "(47 具名測試)"),
        1,
    ),
    case(
        "文件把 4×6 寫成 4×5 的 105,216 → 紅(§P3-8 原缺陷)",
        base_status(),
        mutated_docs(base_status(), "docs/ROCQ-TRACE.md", "623,616", "105,216"),
        1,
    ),
    case(
        "覆蓋率表 r0.rs 寫 1287/1430、實測 1337/1480 → 紅",
        base_status(),
        mutated_docs(base_status(), "docs/COVERAGE.md", "1337/1480", "1287/1430"),
        1,
    ),
    case(
        "覆蓋率百分比與分子分母不符(90.3% → 91.0%)→ 紅",
        base_status(),
        mutated_docs(base_status(), "docs/COVERAGE.md", "| 90.3% |", "| 91.0% |"),
        1,
    ),
    case(
        "對帳樣本點寫 18、實測 19 → 紅",
        base_status(),
        mutated_docs(base_status(), "docs/EXTERNAL-XCHECK.md", "19 樣本點", "18 樣本點"),
        1,
    ),

    # ── C. 稽核器自身的 fail-closed ─────────────────────────────────
    case(
        "規則命中 0 次(文件被重整/刪除)→ 紅,不是靜默跳過",
        base_status(),
        {k: v for k, v in docs_for(base_status()).items() if k != "docs/ROCQ-PLAN.md"},
        1,
    ),
    case("STATUS.json 缺席 → 用法錯誤(exit 2)", None, docs_for(base_status()), 2),
    case(
        "程式碼出現 Admitted,文件仍宣稱「不含任何 Admitted」→ 紅",
        mutated_status(lambda s: s["rocq"].update(admitted=1)),
        docs_for(base_status()),
        1,
    ),
]


def main():
    bad = 0
    for desc, status, docs, want in CASES:
        rc, out = run(status, docs)
        ok = rc == want
        bad += 0 if ok else 1
        print(f"{'✅' if ok else '❌'} {desc:<44} rc={rc}(期望 {want})")
        if not ok:
            for line in out.strip().splitlines()[:8]:
                print("   " + line)
    print("----")
    if bad:
        print(f"docs_check selftest: {bad} FAIL")
        return 1
    print(f"docs_check selftest: ok({len(CASES)} 個判別力情境全中)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
