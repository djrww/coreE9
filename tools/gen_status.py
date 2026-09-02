#!/usr/bin/env python3
"""`docs/STATUS.json` 產生器(N1;健檢 §P3-14 #5「數字單一來源」)。

為什麼需要它:本專案的文件裡散落著**只能靠人手重跑才會更新**的數字 ——
測試總數(46/47/50)、宇宙規模(105,216 / 623,616)、覆蓋率表、對帳樣本點。
健檢 §P3-8/§P3-9 就是這樣產生的:文件引用了「CI 現有規模 = 623,616」,
而 CI 實際上只跑 105,216。數字沒有單一來源,文件就會靜默說謊。

本腳本把這些數字全部**量測一次**,寫進 `docs/STATUS.json`;
`tools/docs_check.py` 再拿它回頭稽核文件(N2)。資料鏈是單向的:

    文件  ←  STATUS.json  ←  本腳本  ←  cargo / llvm-cov / rocq / status_probe
              (可提交,CI 每次重新產生)        (實測,不含任何手抄值)

用法:
    python3 tools/gen_status.py [--out docs/STATUS.json]
                                [--lcov target/coverage.lcov]
                                [--skip-coverage]   # 僅供本機快速迭代,CI 不得使用

★ 失敗即紅(fail-closed):任何一個來源量不到,就 exit 1 並說明原因,
  絕不寫出「部分欄位為 null」的 STATUS.json —— 半份真相比沒有真相更危險
  (`tools/cov_gate.py` 的 fail-open 教訓,見 `docs/COVERAGE.md` §三之二)。
"""
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import cov_gate  # noqa: E402  複用同一份 lcov 解析器,避免第二個解析器漂移

SCHEMA = "cl0r0-status/v1"


def run(cmd, cwd=ROOT, timeout=1800):
    """跑一條命令,回傳 (rc, stdout+stderr);失敗時不拋出,交由呼叫者判紅。"""
    try:
        p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout)
        return p.returncode, p.stdout + p.stderr
    except FileNotFoundError as e:
        return 127, f"command not found: {e}"
    except subprocess.TimeoutExpired:
        return 124, f"timeout after {timeout}s: {' '.join(cmd)}"


def die(msg, *detail):
    print(f"gen_status: FAILED — {msg}", file=sys.stderr)
    for d in detail:
        print("  " + d, file=sys.stderr)
    sys.exit(1)


# ---------------------------------------------------------------- 測試計數
def collect_tests():
    """分別量測單元 / 集成 / 夜間測試的**具名測試數**。

    刻意分開跑各 target:合併跑 `cargo test --all -- --list` 時,各 test binary
    的列表會串在同一個 stdout 裡,只能靠 cargo 自己的 stderr 分節,而那不是
    穩定介面。分開跑則每個數字都有確定的來源。

    ★ `total` 是 **CI 驗收合同的規模** = unit + integration,**不含** nightly。
      夜間測試(`tests/r3_nightly.rs`)全部 `#[ignore]`,由 nightly workflow
      執行;把它們算進 headline 會讓「50 具名測試即驗收合同」這句話失真。
      故另立 `nightly` 欄位,並在 note 裡寫明。
    """
    out = {}
    for key, extra in (
        ("unit", ["--lib"]),
        ("integration", ["--test", "laws"]),
        ("nightly", ["--test", "r3_nightly"]),
    ):
        rc, txt = run(["cargo", "test", *extra, "--", "--list"])
        if rc != 0:
            die(f"`cargo test {' '.join(extra)} -- --list` 失敗", txt[-800:])
        n = len(re.findall(r": test$", txt, re.M))
        if n == 0:
            die(f"`cargo test {' '.join(extra)}` 量到 0 條測試 —— 不合理,拒絕寫出")
        out[key] = n
    out["total"] = out["unit"] + out["integration"]
    out["note"] = (
        "total = unit + integration = CI 驗收合同(`cargo test --all`),"
        "不含 nightly(#[ignore],由 .github/workflows/nightly.yml 執行)"
    )
    return out


# ---------------------------------------------------------------- 覆蓋率
def collect_coverage(lcov_path, skip=False):
    if skip:
        return None
    if not os.path.exists(lcov_path):
        rc, txt = run(
            ["cargo", "llvm-cov", "--lcov", "--output-path", lcov_path]
        )
        if rc != 0:
            die(
                "無法取得 lcov(cargo-llvm-cov 未安裝或採集失敗)",
                f"路徑:{lcov_path}",
                txt[-800:],
            )
    cov, collisions, malformed = cov_gate.parse_lcov(lcov_path)
    if malformed:
        die("lcov 有缺 LF:/LH: 的區段", *malformed[:5])
    if collisions:
        die("lcov 有同名 basename 衝突", *[f"{n}: {a} vs {b}" for n, a, b in collisions])
    if not cov:
        die("lcov 沒有任何可計入的模組(與 cov_gate 同一條 fail-open 防線)")
    return {
        name: {
            "lf": lf,
            "lh": lh,
            "ratio": round(lh / lf, 4) if lf else 1.0,
        }
        for name, (lf, lh) in sorted(cov.items())
    }


# ---------------------------------------------------------------- Rocq
def collect_rocq():
    theories = os.path.join(ROOT, "rocq", "theories")
    if not os.path.isdir(theories):
        die(f"找不到 {theories}")
    files = sorted(f for f in os.listdir(theories) if f.endswith(".v"))
    if not files:
        die("rocq/theories 下沒有 .v 檔")

    lines = admitted = axiom = 0
    for f in files:
        text = open(os.path.join(theories, f), encoding="utf-8").read()
        lines += text.count("\n") + (0 if text.endswith("\n") or not text else 1)
        # `Admitted.` 才是「放棄證明」;註解裡提到 Admitted 不算。
        admitted += len(re.findall(r"^\s*Admitted\s*\.", text, re.M))
        axiom += len(re.findall(r"^\s*(?:Axiom|Parameter|Hypothesis)\b", text, re.M))

    # 對帳樣本點:跑一次 reconcile,從 `[reconcile] Rust 事實 N 項` 量測。
    rc, txt = run(["make", "-C", "rocq", "reconcile"])
    if rc != 0:
        die("`make -C rocq reconcile` 失敗", txt[-800:])
    m = re.search(r"Rust 事實 (\d+) 項", txt)
    if not m:
        die("無法從 reconcile 輸出量得樣本點數(輸出格式變了?)", txt[-800:])
    facts = int(m.group(1))

    return {
        "files": len(files),
        "lines": lines,
        "admitted": admitted,
        "axiom": axiom,
        "reconcile_facts": facts,
        "theory_files": files,
    }


# ---------------------------------------------------------------- 宇宙規模
def collect_universes():
    rc, txt = run(["cargo", "run", "--release", "--example", "status_probe"])
    if rc != 0:
        die("`cargo run --release --example status_probe` 失敗", txt[-800:])
    m = re.search(r"STATUS_PROBE_JSON:(\{.*\})", txt)
    if not m:
        die("status_probe 沒輸出 STATUS_PROBE_JSON(輸出格式變了?)", txt[-800:])
    return json.loads(m.group(1))


# ---------------------------------------------------------------- main
def main(argv):
    out_path = os.path.join(ROOT, "docs", "STATUS.json")
    lcov_path = os.path.join(ROOT, "target", "coverage.lcov")
    skip_cov = False
    for flag in ("--out", "--lcov"):
        if flag in argv:
            i = argv.index(flag)
            v = argv[i + 1]
            del argv[i : i + 2]
            if flag == "--out":
                out_path = v if os.path.isabs(v) else os.path.join(ROOT, v)
            else:
                lcov_path = v if os.path.isabs(v) else os.path.join(ROOT, v)
    if "--skip-coverage" in argv:
        skip_cov = True
        argv.remove("--skip-coverage")

    def ver(cmd):
        rc, txt = run(cmd)
        return (txt.strip().splitlines() or ["?"])[-1].strip() if rc == 0 else "?"

    status = {
        "schema": SCHEMA,
        "generated_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "note": (
            "由 tools/gen_status.py 產生,請勿手改。CI 每次重新產生後由 "
            "tools/docs_check.py 稽核文件;本地重生成:python3 tools/gen_status.py"
        ),
        "toolchain": {"rustc": ver(["rustc", "--version"]), "coqc": ver(["coqc", "--version"])},
        "tests": collect_tests(),
        "coverage": collect_coverage(lcov_path, skip_cov),
        "rocq": collect_rocq(),
        "universes": collect_universes(),
    }

    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(status, f, ensure_ascii=False, indent=2)
        f.write("\n")

    t = status["tests"]
    print(
        f"gen_status: wrote {os.path.relpath(out_path, ROOT)} — "
        f"tests {t['unit']}+{t['integration']}={t['total']} · "
        f"coverage {len(status['coverage']) if status['coverage'] else 'skipped'} 模組 · "
        f"rocq {status['rocq']['files']} 檔/{status['rocq']['lines']} 行 "
        f"(Admitted {status['rocq']['admitted']} / Axiom {status['rocq']['axiom']}, "
        f"對帳 {status['rocq']['reconcile_facts']} 樣本點) · "
        f"4×6 宇宙 {status['universes']['newman_4x6']['states']} 狀態"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
