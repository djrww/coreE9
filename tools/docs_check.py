#!/usr/bin/env python3
"""文件一致性稽核(N2;健檢 §P3-14 #5「數字單一來源」的執行端)。

搭檔:`tools/gen_status.py`(N1)負責**量測**並寫出 `docs/STATUS.json`;
本腳本負責**稽核** —— 把文件裡那些「人手抄進 markdown」的數字,逐條對回實測值。

    docs/STATUS.json  ──量測──  cargo / llvm-cov / rocq / status_probe
          │
          └──稽核──>  docs/*.md, ROADMAP.md   (本腳本)

用法:
    python3 tools/docs_check.py [--status docs/STATUS.json] [--root .]

退出碼:0 = 全部一致;1 = 有不一致(列出「檔案:行號 · 文件寫 X · 實測 Y」);
       2 = 用法錯誤或 STATUS.json 不可讀。

## 三條設計紀律(與 `cov_gate.py` / `bench_gate.py` 同源)

1. **每條規則至少要命中一次**(`min_hits`,預設 1)。命中 0 次視為**失敗**而非
   跳過 —— 文件被重整、句子被改寫而規則靜默失效,正是 §P3-8 那類漂移的溫床。
   規則失準必須讓 CI 變紅,逼人更新規則,而不是讓它 quietly 不再守。
2. **不對歷史敘事判紅**,但也不因此放寬:被排除的檔案/語境在 `EXCLUDES` 與
   各規則的 `files` 裡寫明,`release-v0.1.1/RELEASE.md` 等凍結產物整體排除。
3. **條件式規則**:「無 `Admitted`/`Axiom`」這類宣稱只在程式碼**說謊時**才判紅
   (程式碼乾淨時它是真話,不該因為文件提到它就紅)。

判別力自測:`tools/docs_check_selftest.py`(合成 STATUS.json + 合成文件,
斷言 exit code)—— 稽核器本身也必須證明自己**還能變紅**。
"""
import json
import os
import re
import sys
from collections import namedtuple

SCHEMA = "cl0r0-status/v1"

ROOT_DEFAULT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# 整體排除:凍結的歷史產物(發行說明描述的是**當時**的狀態,不是現況)
EXCLUDES = ("release-v0.1.1/", "target/", ".git/", "node_modules/")

Rule = namedtuple("Rule", "id files regex expects min_hits")


def dig(status, path):
    cur = status
    for part in path.split("."):
        cur = cur[part]
    return cur


def num(s):
    return int(str(s).replace(",", ""))


# ------------------------------------------------------------------ 規則表
RULES = [
    # ── 測試總數(§P3-15b:同一份文件三種數字)────────────────────────
    Rule(
        "tests/coverage-md",
        ["docs/COVERAGE.md"],
        r"全部庫測試\(\*\*(\d+)\*\* 具名測試\).*?集成矩陣\([^)]*?\*\*(\d+)\*\* 條\)\s*=\s*\*\*(\d+)\*\*",
        ("tests.unit", "tests.integration", "tests.total"),
        1,
    ),
    Rule(
        "tests/spec-trace-toc",
        ["docs/SPEC-TRACE.md"],
        r"## 〇、測試全量清單\((\d+) 具名測試\)",
        ("tests.total",),
        1,
    ),
    Rule(
        "tests/spec-trace-ci",
        ["docs/SPEC-TRACE.md"],
        r"上表 \*\*(\d+)\*\* 具名測試",
        ("tests.total",),
        1,
    ),
    Rule(
        "tests/roadmap",
        ["ROADMAP.md"],
        r"\*\*(\d+) 具名測試\*\*",
        ("tests.total",),
        1,
    ),
    Rule(
        "tests/roadmap-slash",
        ["ROADMAP.md"],
        r"`cargo test` \*\*(\d+)/(\d+)\*\*",
        ("tests.total", "tests.total"),
        1,
    ),
    Rule(
        "tests/r3-research",
        ["docs/R3-RESEARCH.md"],
        r"cargo test --all\s+#\s*(\d+) 具名測試",
        ("tests.total",),
        1,
    ),
    Rule(
        "tests/rocq-plan",
        ["docs/ROCQ-PLAN.md"],
        r"已有 (\d+) 具名測試",
        ("tests.total",),
        1,
    ),
    # 這一條刻意只錨定「實跑 = N」的結論,不碰同句中的「「46 具名測試」」——
    # 那是被引用的**歷史口徑**,不是當下的宣稱。
    Rule(
        "tests/rocq-trace",
        ["docs/ROCQ-TRACE.md"],
        r"cargo test --all` = (\d+)",
        ("tests.total",),
        1,
    ),

    # ── 宇宙規模(§P3-8:文件把「探針規模」當「CI 規模」)──────────────
    Rule(
        "universe/r3-table-3x6",
        ["docs/R3-RESEARCH.md"],
        r"\|\s*3×6 \+ distinct-start 過濾[^|]*\|\s*([\d,]+)\s*\|[^|]*\|\s*([\d,]+)\s*\|",
        ("universes.newman_3x6.states", "universes.newman_3x6.critical_pairs"),
        1,
    ),
    Rule(
        "universe/r3-table-4x5",
        ["docs/R3-RESEARCH.md"],
        r"\|\s*4×5 \+ 過濾\s*\|\s*([\d,]+)\s*\|[^|]*\|\s*([\d,]+)\s*\|",
        ("universes.newman_4x5.states", "universes.newman_4x5.critical_pairs"),
        1,
    ),
    Rule(
        "universe/r3-table-4x6",
        ["docs/R3-RESEARCH.md"],
        r"\|\s*4×6 \+ 過濾[^|]*\|\s*([\d,]+)\s*\|[^|]*\|\s*([\d,]+)\s*\|",
        ("universes.newman_4x6.states", "universes.newman_4x6.critical_pairs"),
        1,
    ),
    Rule(
        "universe/rocq-trace-3x6",
        ["docs/ROCQ-TRACE.md"],
        r"\|\s*3×6\(\+過濾\)\s*\|\s*([\d,]+)\s*\|\s*([\d,]+)\s*\|",
        ("universes.newman_3x6.states", "universes.newman_3x6.critical_pairs"),
        1,
    ),
    Rule(
        "universe/rocq-trace-4x5",
        ["docs/ROCQ-TRACE.md"],
        r"\|\s*4×5\(\+過濾\)\s*\|\s*([\d,]+)\s*\|\s*([\d,]+)\s*\|",
        ("universes.newman_4x5.states", "universes.newman_4x5.critical_pairs"),
        1,
    ),
    Rule(
        "universe/rocq-trace-4x6",
        ["docs/ROCQ-TRACE.md"],
        r"\|\s*4×6\(\+過濾,=CI\)\s*\|\s*([\d,]+)\s*\|\s*([\d,]+)\s*\|",
        ("universes.newman_4x6.states", "universes.newman_4x6.critical_pairs"),
        1,
    ),
    # 「N 狀態 × M 臨界對」的散句引述(SPEC-TRACE / ROCQ-PLAN / BENCH / ROCQ-TRACE)
    Rule(
        "universe/inline-4x6",
        ["docs/SPEC-TRACE.md", "docs/ROCQ-PLAN.md", "docs/BENCH.md", "docs/ROCQ-TRACE.md"],
        r"([\d,]+) 狀態\s*×\s*([\d,]+) 臨界對",
        ("universes.newman_4x6.states", "universes.newman_4x6.critical_pairs"),
        1,
    ),

    # ── 對帳樣本點(§P3-14 #2)────────────────────────────────────────
    # 排除 HARD-ITEMS.md(它寫的是「19 → ≥800」的**目標**,不是現況)
    # 與 ROCQ-TRACE.md(§二提到的是歷史上的 5 個樣本點)。
    Rule(
        "rocq/reconcile-samples",
        ["docs/SPEC-TRACE.md", "docs/R3-RESEARCH.md", "docs/EXTERNAL-XCHECK.md", "ROADMAP.md"],
        r"(\d+) 樣本點",
        ("rocq.reconcile_facts",),
        1,
    ),
]

# 覆蓋率表(docs/COVERAGE.md)：每行一個模組,需逐列對回 lcov 實測。
COVERAGE_ROW = re.compile(
    r"\|\s*`([A-Za-z0-9_]+\.rs)`[^|]*\|\s*(\d+)/(\d+)\s*\|\s*([\d.]+)%"
)
COVERAGE_FILES = ["docs/COVERAGE.md"]

# 「不含 Admitted/Axiom」的宣稱 —— 僅在程式碼說謊時判紅(條件式規則)。
HONESTY_CLAIM = re.compile(
    r"不含任何\s*`Admitted`|無\s*`Admitted`|Admitted`/`Axiom`\s*為\s*0|0\s*Admitted"
)
HONESTY_FILES = [
    "docs/ROCQ-PLAN.md",
    "docs/ROCQ-TRACE.md",
    "docs/HARD-ITEMS.md",
    "docs/SPEC-TRACE.md",
]


def read_docs(root, rel_paths):
    """回傳 [(rel_path, line_no, text)]。"""
    out = []
    for rel in rel_paths:
        p = os.path.join(root, rel)
        if not os.path.exists(p):
            continue
        for i, line in enumerate(open(p, encoding="utf-8").read().splitlines(), 1):
            out.append((rel, i, line))
    return out


def main(argv):
    root = ROOT_DEFAULT
    status_path = None
    # ★ 先解析 `--root`:`--status` 的預設值依賴它。兩者一起解析會讓
    #   `--root <tmpdir>` 仍去讀真實的 docs/STATUS.json —— 自測情境 8/9
    #   就是這樣抓到這個 bug 的(`tools/docs_check_selftest.py`)。
    if "--root" in argv:
        i = argv.index("--root")
        root = argv[i + 1]
        del argv[i : i + 2]
    if "--status" in argv:
        i = argv.index("--status")
        v = argv[i + 1]
        del argv[i : i + 2]
        status_path = v if os.path.isabs(v) else os.path.join(root, v)
    if status_path is None:
        status_path = os.path.join(root, "docs", "STATUS.json")
    if not os.path.exists(status_path):
        print(f"docs_check: 找不到 {status_path}(先跑 tools/gen_status.py)", file=sys.stderr)
        return 2
    status = json.load(open(status_path, encoding="utf-8"))
    if status.get("schema") != SCHEMA:
        print(
            f"docs_check: STATUS.json schema 不符(期望 {SCHEMA},實為 "
            f"{status.get('schema')!r})",
            file=sys.stderr,
        )
        return 2

    bad = []

    # ---- 1. 一般規則 ----
    for r in RULES:
        lines = read_docs(root, r.files)
        hits = 0
        for rel, ln, text in lines:
            for m in re.finditer(r.regex, text):
                hits += 1
                for gi, path in enumerate(r.expects, start=1):
                    want = dig(status, path)
                    got = num(m.group(gi))
                    if got != want:
                        bad.append(
                            f"{rel}:{ln} · 規則 {r.id} · 第 {gi} 欄 "
                            f"文件寫 {m.group(gi)}、實測 {want}({path})"
                        )
        if hits < r.min_hits:
            bad.append(
                f"規則 {r.id} 只命中 {hits} 次(門檻 {r.min_hits})—— "
                "文件可能被重整而規則已失效;這視為失敗,請更新規則或文件"
            )

    # ---- 2. 覆蓋率表 ----
    cov_hits = 0
    for rel, ln, text in read_docs(root, COVERAGE_FILES):
        for m in COVERAGE_ROW.finditer(text):
            module, lh, lf, pct = m.group(1), int(m.group(2)), int(m.group(3)), float(m.group(4))
            got = status.get("coverage") or {}
            if module not in got:
                continue  # 表裡可能有不屬於 lcov 的列(如 bin),交由 cov_gate 管
            cov_hits += 1
            c = got[module]
            if lh != c["lh"] or lf != c["lf"]:
                bad.append(
                    f"{rel}:{ln} · 覆蓋率表 `{module}` 文件寫 {lh}/{lf}、"
                    f"實測 {c['lh']}/{c['lf']}"
                )
            want_pct = round(c["lh"] / c["lf"] * 100, 1) if c["lf"] else 100.0
            if abs(pct - want_pct) > 0.05:
                bad.append(
                    f"{rel}:{ln} · 覆蓋率表 `{module}` 文件寫 {pct}%、實測 {want_pct}%"
                )
    if status.get("coverage") and cov_hits == 0:
        bad.append("覆蓋率表規則命中 0 次 —— docs/COVERAGE.md 的表格格式變了?")

    # ---- 3. 條件式誠實宣稱 ----
    if status.get("rocq", {}).get("admitted") or status.get("rocq", {}).get("axiom"):
        for rel, ln, text in read_docs(root, HONESTY_FILES):
            if HONESTY_CLAIM.search(text):
                bad.append(
                    f"{rel}:{ln} · 宣稱「無 Admitted/Axiom」,但實測 "
                    f"Admitted={status['rocq']['admitted']}、Axiom={status['rocq']['axiom']}"
                )

    if bad:
        print("DOCS CONSISTENCY FAILED:", file=sys.stderr)
        for b in bad:
            print("  " + b, file=sys.stderr)
        return 1
    print(
        f"docs_check: ok({len(RULES)} 條規則 + 覆蓋率表 "
        f"{cov_hits} 列 + 誠實宣稱,全部與 {os.path.relpath(status_path, root)} 一致)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
