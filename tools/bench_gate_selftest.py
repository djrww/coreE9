#!/usr/bin/env python3
"""`tools/bench_gate.py` 的判別力自測(2026-09-02 修 gate 時一併入庫)。

為什麼需要它:重寫前的 gate 在 main HEAD 長期紅,而重寫後很容易「修到永遠綠」
——那比永遠紅更糟。本腳本用**合成資料**驗七個情境,每個都斷言確定的 exit code,
跑在 CI 的 `bench` job 裡(先自測 gate、再判真實代碼)。

用法:python3 tools/bench_gate_selftest.py
"""
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
GATE = os.path.join(HERE, "bench_gate.py")

KERNELS = ["lex", "parse", "laminar", "named_sexp", "l7b_evaluate", "r0_lex", "r0_parse", "newman_3x4x6"]


TINY = {"lex", "r0_lex"}          # <10 µs/樣本的两個內核


def payload(scale_k=1.0, scale_n=1.0, scale_tiny=1.0, base=False):
    """造一份 hotpaths-v2 的統計;scale_* 為相對基線的放大倍率。"""
    k = {"null": _stat(0.0002 * scale_n, 0.00001 * scale_n, 9, 1000)}
    med = {"lex": 0.0007, "parse": 0.0044, "laminar": 0.0092, "named_sexp": 0.0098,
           "l7b_evaluate": 0.0077, "r0_lex": 0.0006, "r0_parse": 0.0028, "newman_3x4x6": 6.9}
    for name in KERNELS:
        f = scale_tiny if name in TINY else scale_k
        s = _stat(med[name] * f, med[name] * 0.02 * f, 9, 1000)
        k[name] = s
    if base:  # 基線不帶 null 的漂移資訊:與 cur 同值即可
        k["null"] = _stat(0.0002, 0.00001, 9, 1000)
    return json.dumps({"kernels": k, "harness": "hotpaths-v2"})


def _stat(med, mad, reps, samples):
    return {
        "median_ms_per_sample": med,
        "mad_ms": mad,
        "min_ms": med * 0.97,
        "max_ms": med * 1.05,
        "reps": reps,
        "samples_per_rep": samples,
    }


def run(tmp, log_text, extra=()):
    lp = os.path.join(tmp, "bench.log")
    bp = os.path.join(tmp, "BASELINE.json")
    with open(lp, "w", encoding="utf-8") as f:
        f.write("some header\nBENCH_JSON:" + log_text + "\n")
    with open(bp, "w", encoding="utf-8") as f:
        f.write(json.dumps({"note": "t", "harness": "hotpaths-v2",
                            "kernels": json.loads(payload(base=True))["kernels"]}))
    r = subprocess.run([sys.executable, GATE, lp, bp, *extra],
                       capture_output=True, text=True)
    return r.returncode, r.stdout


CASES = [
    # (說明, 語料參數, 額外旗標, 期望 exit code)
    ("實測無變化 → 綠", dict(), [], 0),
    ("整台機慢 30%(null 同步)→ 不誤判", dict(scale_k=1.30, scale_n=1.30), [], 0),
    ("只有代碼慢 30%(null 不動)→ 紅", dict(scale_k=1.30, scale_n=1.00), [], 1),
    ("只有代碼慢 50% → 紅", dict(scale_k=1.50, scale_n=1.00), [], 1),
    ("代碼變快 40% → 綠(improved)", dict(scale_k=0.60, scale_n=1.00), [], 0),
    ("盲区:全部含 null ×2 + --assume-env-stable → 紅",
     dict(scale_k=2.0, scale_n=2.0), ["--assume-env-stable"], 1),
    ("同資料未加旗標 → 視為環境,綠(記錄此固有取捨)",
     dict(scale_k=2.0, scale_n=2.0), [], 0),
    ("大內核慢 60%:--tiny-us 10 仍紅(小內核轉 info 不護短)",
     dict(scale_k=1.60, scale_n=1.00), ["--tiny-us", "10"], 1),
    ("只有 tiny 內核慢:--tiny-us 10 → 綠(informational)",
     dict(scale_tiny=3.0, scale_n=1.00), ["--tiny-us", "10"], 0),
]


def main():
    bad = 0
    with tempfile.TemporaryDirectory() as tmp:
        for desc, kw, extra, want in CASES:
            rc, out = run(tmp, payload(**kw), extra)
            ok = rc == want
            bad += 0 if ok else 1
            print(f"{'✅' if ok else '❌'} {desc:<46} rc={rc}(期望 {want})")
            if not ok:
                print("   ---- gate 輸出 ----")
                for line in out.strip().splitlines():
                    print("   " + line)
    print("----")
    if bad:
        print(f"bench_gate selftest: {bad} FAIL")
        return 1
    print(f"bench_gate selftest: ok({len(CASES)} 個判別力情境全中)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
