#!/usr/bin/env python3
"""覆蓋率門檻(P1 #6):解析 lcov,按模組檢查行覆蓋率。

門檻(本迭代固化):
  * 核心載體模組(span/lex/parse/tree/gen/shrink/rep/r0/l9newman)>= 90%;
  * 豁免模組各自帶**自己的硬門檻**(不是共用一個),理由逐條記於 EXEMPT:
    - ast.rs  ≥75% —— §3.2-3.3 語義面:
      (a) llvm-cov 對 3 行內的小函數(match 直接返回 &str)存在計數器歸屬失真;
      (b) 其餘未覆蓋行對應 CL0 語法不可達的保留槽(Ctx::Lhs / killer 防禦分支)。
    - edit.rs ≥85% —— 2026-09-03 CI 實測:本機量 96/97 = 99.0%,GitHub runner
      量 86/97 = 88.7%。同一份碼、同一 rustc 1.98.0 / cargo-llvm-cov 0.9.0 /
      x86_64、同一組 36 個測試全過。差的那 10 行(`Edit::is_empty` 53-55、
      `Edit::shift` 58-66)在 runner 上被 inline 掉後歸因給呼叫者,並非沒執行
      —— 已證明:本機只跑 tests/laws.rs,`edit.rs:58` 就有 count=603。
      **真實缺口只有第 93 行這 1 行。** 門檻取 85% 而非 75%,是為了仍抓得到
      掉 ≥4 行的真實退化(88.7% → 84.5% 即紅),不因豁免而開天窗。
  * bin 的 main(fuzz.rs / cl0r0.rs / l9newman.rs)不計入(由執行驅動)。

## 覆蓋率數字不可跨平台移植(2026-09-03,已知且無法由配置消除)

已實測排除:stale profraw(`rm -rf target` 重建後一樣)、toolchain 漂移(鎖
1.98.0 後一樣)、隨機性(fuzzer 固定種子)。行表兩邊**完全一致**,差異在哪些行
顯示為 0。關掉增量編譯後 `rep.rs` 兩邊一致(95.9%),但 `edit.rs` 與 `span.rs`
仍**反向翻轉**(本地 99.0%/88.5%,runner 88.7%/100.0%)。`-Ccodegen-units=1/256`、
`-Cllvm-args=--inline-threshold=0` 皆無效。
⇒ **以 CI 為 canonical**;本機若量到不同的數字,屬已知現象,不是回歸。

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

CORE = ["span.rs", "lex.rs", "parse.rs", "tree.rs", "gen.rs",
        "shrink.rs", "rep.rs", "r0.rs", "l9newman.rs"]
CORE_TOL = 0.90

# 豁免模組自帶硬門檻:name -> (floor, 書面理由)
EXEMPT = {
    "ast.rs": (0.75, "語義面(§3.2-3.3):行映射失真 + 保留槽"),
    "edit.rs": (0.85, "llvm-cov 對 is_empty/shift 的行歸因跨平台不一致;真實缺口僅 1 行"),
}
# CLI 旗標可覆寫個別門檻(自測用):--ast / --edit


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
    core_tol = CORE_TOL
    floors = {name: tol for name, (tol, _) in EXEMPT.items()}
    overrides = {"--core": None, "--ast": "ast.rs", "--edit": "edit.rs"}
    for flag, target in overrides.items():
        while flag in args:
            i = args.index(flag)
            v = float(args[i + 1])
            del args[i : i + 2]
            if target is None:
                core_tol = v
            else:
                floors[target] = v
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
            tol = floors[name]
            ok = ratio >= tol
            tag = ("ok(exempt)" if ok else "FAIL(exempt)")
            tag += f"  [{EXEMPT[name][1]},硬門檻 {tol:.0%}]"
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
        f"= {len(CORE)} 核心 ≥{core_tol:.0%} + {len(EXEMPT)} 豁免["
        + ", ".join(f"{n} ≥{floors[n]:.0%}" for n in sorted(EXEMPT))
        + "])"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
