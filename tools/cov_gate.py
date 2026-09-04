#!/usr/bin/env python3
"""覆蓋率門檻(P1 #6):解析 lcov,按模組檢查行覆蓋率。

門檻(本迭代固化):
  * 核心載體模組(span/lex/parse/tree/edit/gen/shrink/rep/r0/l9newman)>= 90%;
  * ast.rs(§3.2-3.3 語義面)豁免於 90%,硬門檻 75% —— 理由見 docs/COVERAGE.md:
    (a) llvm-cov 對 3 行內的小函數(match 直接返回 &str)存在計數器歸屬失真;
    (b) 其餘未覆蓋行對應 CL0 語法不可達的保留槽(Ctx::Lhs / killer 防禦分支)。
  * bin 的 main(fuzz.rs / cl0r0.rs / l9newman.rs)不計入(由執行驅動)。

用法:python3 tools/cov_gate.py <coverage.lcov> [--core 0.90] [--ast 0.75]
"""
import re
import sys

CORE = ["span.rs", "lex.rs", "parse.rs", "tree.rs", "edit.rs", "gen.rs", "shrink.rs", "rep.rs", "r0.rs", "l9newman.rs"]
EXEMPT = {"ast.rs": "語義面(§3.2-3.3):行映射失真 + 保留槽,硬門檻 75%"}
CORE_TOL = 0.90
AST_TOL = 0.75


def parse_lcov(path):
    data = open(path, encoding="utf-8").read()
    cov = {}
    for sf, body in re.findall(r"SF:(.*?)\n(.*?)(?=SF:|\Z)", data, re.S):
        if "/bin/" in sf or "benches/" in sf:
            continue  # bin main 與 bench 不計入庫覆蓋
        lf = int(re.search(r"LF:(\d+)", body).group(1))
        lh = int(re.search(r"LH:(\d+)", body).group(1))
        cov[sf.split("/")[-1]] = (lf, lh)
    return cov


def main(argv):
    path = argv[0]
    cov = parse_lcov(path)
    bad = []
    print("module        covered     ratio  verdict")
    for name in sorted(cov):
        lf, lh = cov[name]
        ratio = lh / lf if lf else 1.0
        if name in CORE:
            ok = ratio >= CORE_TOL
            tag = "ok" if ok else "FAIL(<90%)"
        elif name in EXEMPT:
            ok = ratio >= AST_TOL
            tag = ("ok(exempt <75%)" if ok else "FAIL(exempt)")
            tag += "  [" + EXEMPT[name] + "]"
        else:
            continue
        print(f"{name:<12} {lh:>5}/{lf:<5} {ratio:>8.1%}  {tag}")
        if not ok:
            bad.append(name)
    if bad:
        print("COVERAGE GATE FAILED: " + ", ".join(bad))
        return 1
    print("coverage gate: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
