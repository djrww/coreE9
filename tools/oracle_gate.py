#!/usr/bin/env python3
"""oracle 對帳門(P4-0;docs/PIVOT-RUSTC-ORACLE.md §四、§九-R7)。

用法:
  python3 tools/oracle_gate.py <corpus_dir> <baseline.json> [--parity] [--update] [--quiet]

紀律(繼承 bench_gate 2026-09-02 重寫的教訓,oracle 版):
  1. 兩趟全等(null 紀律):判決沒有計時噪聲,同機同版兩趟必須逐案例一致;
     不一致 = 環境在動(工具鏈被切換/磁碟壞),先紅,不猜。
     —— 這是 bench「best-of-n + null 校正」在確定性輸出下的退化形式。
  2. BUG 候選 = 0:任何案例判決不符檔名期望 → 紅(律不過,碼不合)。
     --update 不能洗白:基線如實記錄 pass=false,gate 永遠對它報紅。
  3. 基線對帳:版本漂移只警告(rustc 升級是常態);**判決漂移**才是紅 ——
     逐案例列出,人工歸因 BUG / MODEL-DIFF / RUSTC-BUG(docs/ORACLE-TRACE.md)
     後 --update 入庫。語料集合變化同樣要求顯式 --update。
  4. 門檻只建立在 Tier-A(stable rustc);Tier-B(nightly/rustc_private)
     按 PIVOT-RUSTC-ORACLE §四 永不入主門檻。
  5. --parity(P4-1):rustc × 模型三軌 parity 跑者(`oracle parity`),
     違規(未註冊分歧 / stale 註冊 / lexical 漏報借用衝突類碼)→ 紅。
     判定邏輯單一源於 Rust 側(`cl0r0::model::parity_violations`),
     本腳本只中繼退出碼 —— 避免雙語言重複實作漂移。
"""
import json
import subprocess
import sys

JSON_TAG = "ORACLE_JSON:"


def run_once(corpus_dir):
    """跑一趟語料對帳;回傳 payload(dict)。"""
    cmd = ["cargo", "run", "--quiet", "--features", "oracle", "--bin", "oracle",
           "--", "run", "--json", corpus_dir]
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode not in (0, 1):
        sys.stderr.write(p.stderr[-4000:])
        sys.exit(f"oracle 跑者異常退出(code={p.returncode});上為 stderr 尾部")
    line = next((l for l in p.stdout.splitlines() if l.startswith(JSON_TAG)), None)
    if line is None:
        sys.stderr.write(p.stdout[-2000:])
        sys.exit("找不到 ORACLE_JSON 行(跑者輸出被污染?)")
    return json.loads(line[len(JSON_TAG):])


def run_parity(corpus_dir):
    """P4-1 parity 跑者中繼:判定在 Rust 側,此處只傳遞輸出與退出碼。"""
    cmd = ["cargo", "run", "--quiet", "--features", "oracle", "--bin", "oracle",
           "--", "parity", corpus_dir]
    p = subprocess.run(cmd, capture_output=True, text=True)
    sys.stdout.write(p.stdout)
    if p.returncode != 0:
        sys.stderr.write(p.stderr)
    return p.returncode


def fingerprint(cases):
    """判決指紋:file → (codes, spans, pass)。"""
    return {c["file"]: (c["codes"], [tuple(s) for s in c["spans"]], bool(c["pass"]))
            for c in cases}


def main():
    args = sys.argv[1:]
    update = "--update" in args
    quiet = "--quiet" in args
    parity = "--parity" in args
    pos = [a for a in args if not a.startswith("--")]
    if len(pos) != 2:
        sys.exit(__doc__)
    corpus_dir, baseline_path = pos

    a = run_once(corpus_dir)
    b = run_once(corpus_dir)
    red = []

    # ① 兩趟全等(確定性輸出的 null 紀律)
    fa, fb = fingerprint(a["cases"]), fingerprint(b["cases"])
    if fa != fb:
        for f in sorted(set(fa) | set(fb)):
            if fa.get(f) != fb.get(f):
                red.append(f"兩趟不一致:{f} {fa.get(f)} vs {fb.get(f)}"
                           " —— 環境在動(工具鏈被切換?),先修環境")

    # ② BUG 候選 = 0
    for c in a["cases"]:
        if not c["pass"]:
            red.append(f"BUG 候選:{c['file']} 期望 {c['expect']} 實得 {c['verdict']}"
                       + (f"({c['note']})" if c.get("note") else ""))

    # ③ 基線對帳(--update 時跳過:刷新模式只寫不比)
    if not update:
        try:
            with open(baseline_path) as fh:
                base = json.load(fh)
        except FileNotFoundError:
            red.append(f"無基線:{baseline_path}(首跑請先 --update 入庫)")
            base = None
        if base is not None:
            if base.get("rustc_version") != a["rustc_version"]:
                print(f"[warn] rustc 版本漂移:基線 {base.get('rustc_version')}"
                      f" → 本趟 {a['rustc_version']}(判決不變則綠)")
            old = base.get("cases", {})
            new = {c["file"]: c for c in a["cases"]}
            gone = sorted(set(old) - set(new))
            came = sorted(set(new) - set(old))
            if gone or came:
                red.append(f"語料集合變化:新增 {came} / 移除 {gone} —— triage 後 --update")
            for f in sorted(set(old) & set(new)):
                o, n = old[f], new[f]
                if (o.get("codes") != n["codes"]
                        or [tuple(s) for s in o.get("spans", [])]
                        != [tuple(s) for s in n["spans"]]
                        or o.get("expect") != n["expect"]):
                    red.append(f"判決漂移:{f} 基線 codes={o.get('codes')}"
                               f" spans={o.get('spans')} → 本趟 codes={n['codes']}"
                               f" spans={n['spans']}(歸因後 --update)")

    # ④ --update:以第一趟入庫(如實記錄,含 pass=false)
    if update:
        payload = {
            "generated_by": "tools/oracle_gate.py --update",
            "rustc_version": a["rustc_version"],
            "cases": {c["file"]: {"expect": c["expect"], "codes": c["codes"],
                                  "spans": c["spans"], "pass": c["pass"]}
                      for c in a["cases"]},
        }
        with open(baseline_path, "w") as fh:
            json.dump(payload, fh, ensure_ascii=False, indent=2, sort_keys=True)
            fh.write("\n")
        print(f"基線已刷新:{baseline_path}(rustc {a['rustc_version']},"
              f"{len(a['cases'])} 案例)")

    if red:
        prefix = "" if quiet else "[RED] "
        print("\n".join(prefix + r for r in red))
        sys.exit(1)

    if parity:
        print("── parity(P4-1:rustc × 模型三軌)──")
        if run_parity(corpus_dir) != 0:
            sys.exit(1)

    print(f"[GREEN] oracle gate 通過:{len(a['cases'])} 案例,BUG 候選 0,兩趟全等"
          + ("" if update else f";基線 {baseline_path} 對帳一致"))


if __name__ == "__main__":
    main()
