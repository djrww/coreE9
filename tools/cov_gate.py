#!/usr/bin/env python3
"""覆蓋率門檻(P1 #6):解析 lcov,按模組檢查行覆蓋率。

門檻(本迭代固化):
  * 核心載體模組(span/lex/parse/tree/edit/gen/shrink/rep/r0/l9newman)>= 90%;
  * ast.rs(§3.2-3.3 語義面)豁免於 90%,硬門檻 75% —— 理由見 docs/COVERAGE.md:
    (a) llvm-cov 對 3 行內的小函數(match 直接返回 &str)存在計數器歸屬失真;
    (b) 其餘未覆蓋行對應 CL0 語法不可達的保留槽(Ctx::Lhs / killer 防禦分支)。
  * bin 的 main(fuzz.rs / cl0r0.rs / l9newman.rs)不計入(由執行驅動)。

## 為什麼要有「fail-open 防線」(2026-09-03 健檢 P2-6)

舊版只要在 lcov 裡**看不到**某個模組,就當它不存在:
  * 報告解析出 0 個區段(llvm-cov 沒跑起來 / 路徑被 `/bin/`、`benches/`
    規則全數濾掉)⇒ 什麼都不檢查 ⇒ 印 `coverage gate: ok`、**exit 0**;
  * 某個核心模組從報告中消失(改名、被 cfg 掉)⇒ 同上,靜默放行。
門檻因此只在「剛好有報告」時才存在 —— 這比沒有門檻更危險,因為它會**假裝**在守。
本版改為:
  1. `cov` 為空 ⇒ 直接判紅;
  2. 任一 `CORE` / `EXEMPT` 模組缺席 ⇒ 直接判紅(列出缺席者);
  3. 同名 basename 互相覆蓋(如 `src/l9newman.rs` vs `src/bin/l9newman.rs`
     沒被過濾掉時)⇒ 直接判紅,而不是讓後者默默贏;
  4. 正常路徑額外印出「已檢查 N 個模組,其中受門檻約束 M 個」,讓門檻範圍可觀測。

判別力自測:`tools/cov_gate_selftest.py`(9 情境,斷言 exit code),已掛 CI:
四條 fail-open 防線 + 門檻本身仍要能判紅(合成 lcov,不依賴真實採樣)。

用法:python3 tools/cov_gate.py <coverage.lcov> [--core 0.90] [--ast 0.75]
"""
import re
import sys

CORE = ["span.rs", "lex.rs", "parse.rs", "tree.rs", "edit.rs", "gen.rs", "shrink.rs", "rep.rs", "r0.rs", "l9newman.rs"]
EXEMPT = {"ast.rs": "語義面(§3.2-3.3):行映射失真 + 保留槽,硬門檻 75%"}
CORE_TOL = 0.90
AST_TOL = 0.75


def parse_lcov(path):
    """回傳 `(cov, collisions, malformed)`。

    * `cov`: basename -> (LF, LH) —— 只含**計入**的模組;
    * `collisions`: 兩個不同路徑卻同名(basename 會互相覆蓋)的清單;
    * `malformed`: 找不到 `LF:` / `LH:` 的 `SF:` 區段路徑。

    `SF:` 路徑含 `/bin/` 或 `benches/` 者一律略過(bin main 與 bench 不計入)。
    """
    data = open(path, encoding="utf-8").read()
    cov = {}
    origins = {}
    collisions = []
    malformed = []
    for sf, body in re.findall(r"SF:(.*?)\n(.*?)(?=SF:|\Z)", data, re.S):
        if "/bin/" in sf or "benches/" in sf:
            continue
        m_lf = re.search(r"LF:(\d+)", body)
        m_lh = re.search(r"LH:(\d+)", body)
        if not m_lf or not m_lh:
            malformed.append(sf)
            continue
        name = sf.split("/")[-1]
        if name in cov and origins.get(name) != sf:
            collisions.append((name, origins[name], sf))
        cov[name] = (int(m_lf.group(1)), int(m_lh.group(1)))
        origins[name] = sf
    return cov, collisions, malformed


def fail(msg, *detail):
    print("COVERAGE GATE FAILED: " + msg, file=sys.stderr)
    for d in detail:
        print("  " + d, file=sys.stderr)
    return 1


def main(argv):
    args = list(argv)
    core_tol, ast_tol = CORE_TOL, AST_TOL
    for flag in ("--core", "--ast"):
        if flag in args:
            i = args.index(flag)
            v = float(args[i + 1])
            del args[i : i + 2]
            if flag == "--core":
                core_tol = v
            else:
                ast_tol = v
    if not args:
        print(__doc__)
        return 2
    path = args[0]
    cov, collisions, malformed = parse_lcov(path)
    required = sorted(set(CORE) | set(EXEMPT))

    # ---- fail-open 防線:門檻只在「真的檢查到東西」時才算數 ----
    if malformed:
        return fail(
            f"{len(malformed)} 個 SF: 區段缺少 LF:/LH:,報告不完整",
            *malformed[:5],
        )
    if not cov:
        return fail(
            "lcov 報告中沒有任何可計入的模組 —— 門檻無從檢查(不可以是綠)。",
            "可能原因:cargo llvm-cov 沒跑起來;或 crate 路徑含 /bin/、benches/ "
            "而被過濾規則全數濾掉。",
            f"報告檔:{path}",
        )
    missing = [m for m in required if m not in cov]
    if missing:
        return fail(
            f"下列 {len(missing)} 個受門檻約束的模組未出現在報告中(缺席 ≠ 通過):",
            *missing,
        )
    if collisions:
        return fail(
            "報告中出現同名檔案 —— 以 basename 為鍵會互相覆蓋,覆蓋率不可信:",
            *[f"{n}: {a}  vs  {b}" for n, a, b in collisions],
        )

    # ---- 正常判定 ----
    bad = []
    checked = 0
    print("module        covered     ratio  verdict")
    for name in sorted(cov):
        lf, lh = cov[name]
        ratio = lh / lf if lf else 1.0
        if name in CORE:
            ok = ratio >= core_tol
            tag = "ok" if ok else f"FAIL(<{core_tol:.0%})"
        elif name in EXEMPT:
            ok = ratio >= ast_tol
            tag = ("ok(exempt)" if ok else "FAIL(exempt)")
            tag += "  [" + EXEMPT[name] + "]"
        else:
            continue
        checked += 1
        print(f"{name:<12} {lh:>5}/{lf:<5} {ratio:>8.1%}  {tag}")
        if not ok:
            bad.append(name)
    if bad:
        print("COVERAGE GATE FAILED: " + ", ".join(bad), file=sys.stderr)
        return 1
    print(
        f"coverage gate: ok(已檢查 {checked} 個模組;受門檻約束 {len(required)} 個 "
        f"= {len(CORE)} 核心 ≥{core_tol:.0%} + {len(EXEMPT)} 豁免 ≥{ast_tol:.0%})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
