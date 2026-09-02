#!/usr/bin/env python3
"""`tools/cov_gate.py` 的判別力自測(2026-09-03 健檢 P2-6 後入庫)。

為什麼需要它:門檻最危險的失效模式不是「判太嚴」,而是**靜默失效**
(fail-open)—— lcov 空的、模組缺席的、路徑被過濾規則吃掉的,舊版一律
印 `coverage gate: ok` 並 exit 0。那時門檻只在文件裡存在。

本腳本用**合成 lcov** 驗八個情境,每個都斷言確定的 exit code,跑在 CI 的
`coverage` job 裡(先自測 gate、再判真實覆蓋率)—— 與 `bench_gate_selftest.py`
同一紀律:**gate 不只能變綠,必須仍能變紅。**

用法:python3 tools/cov_gate_selftest.py
"""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
GATE = os.path.join(HERE, "cov_gate.py")

CORE = ["span.rs", "lex.rs", "parse.rs", "tree.rs", "edit.rs", "gen.rs",
        "shrink.rs", "rep.rs", "r0.rs", "l9newman.rs"]
EXEMPT = ["ast.rs"]

# 一份「全綠」的基線覆蓋率(貼近真實值)
GOOD = {
    "span.rs": (26, 26),
    "lex.rs": (206, 193),
    "parse.rs": (983, 951),
    "tree.rs": (60, 58),
    "edit.rs": (97, 96),
    "gen.rs": (260, 258),
    "shrink.rs": (77, 74),
    "rep.rs": (245, 222),
    "r0.rs": (1480, 1337),
    "l9newman.rs": (202, 197),
    "ast.rs": (415, 319),      # 76.9% → 走豁免(≥75%)
}


def lcov(entries, skip_bins=True):
    """把 {name: (lf, lh)} 渲染成 lcov 文字。"""
    out = []
    if skip_bins:
        for b in ("cl0r0", "fuzz", "l9newman"):
            out.append(f"SF:/tmp/crate/src/bin/{b}.rs\nLF:10\nLH:2\nend_of_record\n")
    out.append("SF:/tmp/crate/benches/hotpaths.rs\nLF:50\nLH:5\nend_of_record\n")
    for name, (lf, lh) in entries.items():
        out.append(f"SF:/tmp/crate/src/{name}\nLF:{lf}\nLH:{lh}\nend_of_record\n")
    return "\n".join(out) + "\n"


def run(tmp, text, extra=()):
    p = os.path.join(tmp, "coverage.lcov")
    with open(p, "w", encoding="utf-8") as f:
        f.write(text)
    r = subprocess.run([sys.executable, GATE, p, *extra],
                       capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def mutated(**over):
    d = dict(GOOD)
    d.update(over)
    return d


def dropped(*names):
    d = dict(GOOD)
    for n in names:
        d.pop(n, None)
    return d


CASES = [
    # (說明, lcov 文字, 額外旗標, 期望 exit code)
    ("正常報告(全模組在門檻內)→ 綠", lcov(GOOD), [], 0),

    # ---- fail-open 防線(舊版這四條全是 exit 0)----
    ("空報告(沒有任何 SF 區段)→ 紅", "", [], 1),
    ("報告只有被過濾掉的 bin/benches → 紅", lcov({}, skip_bins=True), [], 1),
    ("核心模組 r0.rs 缺席 → 紅(缺席 ≠ 通過)",
     lcov(dropped("r0.rs")), [], 1),
    ("豁免模組 ast.rs 缺席 → 紅", lcov(dropped("ast.rs")), [], 1),

    # ---- 門檻本身仍要能判紅 ----
    ("核心模組 r0.rs 掉到 89.9% → 紅", lcov(mutated(**{"r0.rs": (1000, 899)})), [], 1),
    ("ast.rs 76.9%(豁免區間內)→ 綠", lcov(GOOD), [], 0),
    ("ast.rs 掉到 74.9% → 紅(硬門檻 75%)",
     lcov(mutated(**{"ast.rs": (1000, 749)})), [], 1),

    # ---- 同名 basename 覆蓋 ----
    ("同名 basename(src/ 與 src/util/ 都有 r0.rs)→ 紅",
     lcov(GOOD) + "SF:/tmp/crate/src/util/r0.rs\nLF:1\nLH:0\nend_of_record\n", [], 1),
]


def main():
    bad = 0
    with tempfile.TemporaryDirectory() as tmp:
        for desc, text, extra, want in CASES:
            rc, out, err = run(tmp, text, extra)
            ok = rc == want
            bad += 0 if ok else 1
            print(f"{'✅' if ok else '❌'} {desc:<44} rc={rc}(期望 {want})")
            if not ok:
                print("   ---- gate 輸出 ----")
                for line in (out + err).strip().splitlines():
                    print("   " + line)
    print("----")
    if bad:
        print(f"cov_gate selftest: {bad} FAIL")
        return 1
    print(f"cov_gate selftest: ok({len(CASES)} 個判別力情境全中)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
