#!/usr/bin/env python3
"""`tools/bench_gate.py` 的判別力自測(2026-09-02 修 gate 時一併入庫)。

為什麼需要它:重寫前的 gate 在 main HEAD 長期紅,而重寫後很容易「修到永遠綠」
——那比永遠紅更糟。本腳本用**合成資料**驗十三個情境,每個都斷言確定的 exit code,
跑在 CI 的 `bench` job 裡(先自測 gate、再判真實代碼)。

情境分三組:
  A. 基本判別力(1–9,2026-09-02):無變化/整機慢/只有代碼慢/變快/tiny 內核…;
  B. 污染度單向放寬(10–11,2026-09-03 健檢 P2-4):同樣的 best 慢 25%,
     污染度低 → 紅,污染度高 → 綠。這對情境同時釘住兩件事:污染度**真的**
     參與判定(舊版算了卻沒用),而且只會**放寬**、不會放緊;
  C. null 漂移顯著性門檻(12–13,2026-09-03 健檢 P2-5):同一個「代碼慢 28%」,
     null 漂移落在噪聲帶內 → 不校正 → 紅;null 漂移遠超噪聲 → 視為環境 → 綠。
     防止 16 ns 的底噪單方面挪動紅線。

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

# 各內核的基線 median(ms/樣本)
MED = {"lex": 0.0007, "parse": 0.0044, "laminar": 0.0092, "named_sexp": 0.0098,
       "l7b_evaluate": 0.0077, "r0_lex": 0.0006, "r0_parse": 0.0028, "newman_3x4x6": 6.9}


def _stat(med, mad, reps, samples, mn=None, mx=None):
    return {
        "median_ms_per_sample": med,
        "mad_ms": mad,
        "min_ms": mn if mn is not None else med * 0.97,
        "max_ms": mx if mx is not None else med * 1.05,
        "reps": reps,
        "samples_per_rep": samples,
    }


def payload(scale_k=1.0, scale_n=1.0, scale_tiny=1.0, base=False):
    """造一份 hotpaths-v2 的統計;scale_* 為相對基線的放大倍率。"""
    k = {"null": _stat(0.0002 * scale_n, 0.00001 * scale_n, 9, 1000)}
    for name in KERNELS:
        f = scale_tiny if name in TINY else scale_k
        s = _stat(MED[name] * f, MED[name] * 0.02 * f, 9, 1000)
        k[name] = s
    if base:  # 基線不帶 null 的漂移資訊:與 cur 同值即可
        k["null"] = _stat(0.0002, 0.00001, 9, 1000)
    return json.dumps({"kernels": k, "harness": "hotpaths-v2"})


def payload_contam(scale_k=1.25, scale_n=1.0, contam=0.20):
    """造一份「best(min) 確實慢了 `scale_k`,但本趟污染度為 `contam`」的語料。

    與 `payload()` 的差別:污染度由 `min` 與 `median` 的比值**獨立**控制,
    `min` 仍舊精確地慢 `scale_k` 倍 —— 這樣才能把「污染度對門檻的影響」
    從「best 的回歸幅度」裡分離出來(健檢 P2-4)。
    """
    k = {"null": _stat(0.0002 * scale_n, 0.00001 * scale_n, 9, 1000)}
    for name in KERNELS:
        f = scale_k if name not in TINY else 1.0
        mn = MED[name] * 0.97 * f          # min:確實慢 f 倍(這是判定用的量)
        md = mn * (1.0 + contam)           # median:污染度 = md/mn − 1
        k[name] = _stat(md, md * 0.02, 9, 1000, mn=mn, mx=md * 1.05)
    return json.dumps({"kernels": k, "harness": "hotpaths-v2"})


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
    # ── A. 基本判別力(2026-09-02)────────────────────────────────────────
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

    # ── B. 污染度單向放寬(2026-09-03 健檢 P2-4)──────────────────────────
    #     同一個 best(慢 25%),只有污染度不同 ⇒ 門檻只許變鬆、不許變緊。
    ("污染度低(3%):best 慢 25% → 紅(基準有效容差 20%)",
     payload_contam(scale_k=1.25, contam=0.03), [], 1),
    ("同一個 best(慢 25%)但本趟污染度 20% → 門檻單向放寬,綠(不護短也不誤紅)",
     payload_contam(scale_k=1.25, contam=0.20), [], 0),

    # ── C. null 漂移顯著性門檻(2026-09-03 健檢 P2-5)─────────────────────
    #     同一個「代碼慢 28%」,只差 null 漂移是否在其自身噪聲之內。
    ("null 漂移 +8%(在其自身 2× 噪聲內)→ 不校正:代碼慢 28% 仍紅",
     payload(scale_k=1.28, scale_n=1.08), [], 1),
    ("null 漂移 +30%(遠超噪聲)→ 校正:同樣慢 28% 視為環境,綠",
     payload(scale_k=1.28, scale_n=1.30), [], 0),
]


def main():
    bad = 0
    with tempfile.TemporaryDirectory() as tmp:
        for desc, spec, extra, want in CASES:
            log_text = payload(**spec) if isinstance(spec, dict) else spec
            rc, out = run(tmp, log_text, extra)
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
