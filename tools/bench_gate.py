#!/usr/bin/env python3
"""基準回歸門(P1 #7):解析 hotpaths bench 的 BENCH_JSON 行,與基線比較。

用法:python3 tools/bench_gate.py <bench.log> <baseline.json> [--tol 0.25]
輸出:每項 當前/基線 比率;超出容差 → exit 1(回歸)。
"""
import json
import re
import sys

def main():
    args = sys.argv[1:]
    tol = 0.25
    if "--tol" in args:
        i = args.index("--tol")
        tol = float(args[i + 1])
        del args[i:i + 2]
    log_path, base_path = args[0], args[1]
    text = open(log_path, encoding="utf-8").read()
    m = re.search(r"BENCH_JSON:\{.*\}", text)
    if not m:
        print("NO BENCH_JSON FOUND", file=sys.stderr)
        return 2
    cur = json.loads(m.group(0)[len("BENCH_JSON:"):])
    base = json.load(open(base_path, encoding="utf-8"))["values"]
    bad = []
    print(f"{'kernel':<18}{'base':>10}{'cur':>10}{'ratio':>8}  verdict")
    for k, bv in base.items():
        cv = cur.get(k)
        if cv is None:
            print(f"{k:<18} missing in current run")
            bad.append(k)
            continue
        ratio = cv / bv if bv else float("inf")
        # 回歸門只攔「變慢」(ratio > 1+tol);變快是改善,不判失敗。
        ok = ratio <= 1.0 + tol
        verdict = "ok" if ok else "REGRESSED"
        if ratio < 1.0 - tol:
            verdict = "improved"
        print(f"{k:<18}{bv:>10.4f}{cv:>10.4f}{ratio:>8.3f}  {verdict}")
        if not ok:
            bad.append(k)
    if bad:
        print(f"REGRESSION beyond +{tol * 100:.0f}%: {', '.join(bad)}")
        return 1
    print("bench gate: ok")
    return 0

if __name__ == "__main__":
    sys.exit(main())
