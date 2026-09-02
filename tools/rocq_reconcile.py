#!/usr/bin/env python3
"""P0-c 對帳框架:Rust 實例(標準事實)↔ Rocq 鏡像(vm_compute)差分校驗。

流程:
  1. 執行 `cargo run --example reconcile`(Rust 側,lib 的 rep.rs 語義);
  2. 解析 `key=value` 行;
  3. 生成 `rocq/theories/reconcile_gen.v`:把 Rust 輸出寫成常數,並用
     `Example ... : <Rocq 計算> = <Rust 值> := eq_refl` 斷言 ——
     等號兩邊由 Rocq kernel 以 vm_compute 計算還原,相等才算通過;
  4. `coqc` 編譯該檔(含既有定理庫)。

通過 = 兩個獨立實作在樣本點上語義一致(計數 / 紅邊 / 測度 / 正規形 /
適用規則數 / 四條規則的應用結果)。
"""
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROCQ = ROOT / "rocq"
THEO = ROCQ / "theories"
GEN = THEO / "reconcile_gen.v"

COQFLAGS = [
    "-Q", str(THEO), "Cl0r0",
    "-w", "-deprecated-native-compiler-option",
    "-w", "-native-compiler-disabled",
]


def _ensure_cargo():
    """cargo 可能在 ~/.cargo/bin(沙箱重置後路徑不定),補進 PATH。"""
    if shutil.which("cargo") is None:
        home_bin = str(Path.home() / ".cargo" / "bin")
        if Path(home_bin, "cargo").exists():
            os.environ["PATH"] = home_bin + os.pathsep + os.environ.get("PATH", "")


def run(cmd, cwd=ROOT, check=True, **kw):
    p = subprocess.run(cmd, cwd=cwd, text=True, capture_output=True, **kw)
    if check and p.returncode != 0:
        print(f"[reconcile] command failed: {cmd!r}", file=sys.stderr)
        print(p.stdout[-2000:], file=sys.stderr)
        print(p.stderr[-4000:], file=sys.stderr)
        sys.exit(1)
    return p


def rust_facts():
    _ensure_cargo()
    p = run(["cargo", "run", "--quiet", "--example", "reconcile"])
    facts = {}
    for line in p.stdout.splitlines():
        line = line.strip()
        if "=" in line and not line.startswith("["):
            k, _, v = line.partition("=")
            facts[k.strip()] = v.strip()
    return facts


def coq_list(rust_value):
    """[(0, 1), (0, 2)] -> [(0, 1); (0, 2)] ; [0, 3] -> [0; 3]
    只替換「頂層」分隔逗號(pair 內的逗號保留):以括號深度計。"""
    v = rust_value.strip()
    inner = v[1:-1].strip()
    if not inner:
        return "[]"
    out, depth = [], 0
    for ch in inner:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(";")
        else:
            out.append(ch)
    return "[" + "".join(out) + "]"


def gen(facts):
    checks = []

    def eq(name, rocq_expr, rust_value):
        checks.append(f"Definition v_{name} := Eval vm_compute in {rocq_expr}.")
        checks.append(f"Example chk_{name} : v_{name} = {rust_value} := eq_refl.\n")

    # 枚舉計數
    for n, m in [(1, 3), (2, 3), (2, 4), (3, 3), (3, 4)]:
        expected = facts[f"count({n},{m})"]
        if expected != str((m * (m + 1)) ** n):
            print(f"[reconcile] WARN: Rust count({n},{m})={expected} ≠ formula {m* (m+1)}**{n}")
        eq(f"count_{n}_{m}", f"count_states {n} {m}", expected)

    # 標準樣本(與 examples/reconcile.rs 字面一致)。
    # 注意:record 內不能用 `[;]` 記法(與 {| 解析衝突),用 :: nil 鏈。
    def ev(i, st, kind, a, b):
        return (f"{{| ev_id := {i}; ev_storage := {st}; ev_kind := {kind}; "
                f"ev_it := {{| istart := {a}; iend := {b} |}} |}}")
    canon = (
        "{| st_evs := (" + " :: ".join([
            ev(0, 0, "Mut", 0, 4),
            ev(1, 0, "Sh", 1, 3),
            ev(2, 0, "Sh", 2, 5),
            ev(3, 1, "Mut", 0, 2),
        ]) + " :: nil); st_runtime := ((1, 2) :: nil) |}"
    )
    head = [
        "(* 自動生成:由 tools/rocq_reconcile.py 從 Rust 實例輸出生成。勿手改。 *)",
        "From Coq Require Import List Arith.  (* 記法不跨檔案傳遞,需自行引入 *)",
        "Import ListNotations.",
        "Require Import Cl0r0.Mirror.",
        "Set Implicit Arguments.",
        f"Definition canon : AState := {canon}.",
        "",
    ]

    eq("red_edges", "red_edges canon", coq_list(facts["red_edges"]))
    eq("red_count", "length (red_edges canon)", facts["red_count"])
    eq("measure", "measure canon", facts["measure"])
    eq("nf", "is_nf canon", facts["nf"])
    eq("appli_ct_g", "length (applicable canon CommutativeTrim Guarded)", facts["appli(ct,g)"])
    eq("appli_ct_r", "length (applicable canon CommutativeTrim Raw)", facts["appli(ct,r)"])
    eq("appli_naive_r", "length (applicable canon Naive Raw)", facts["appli(naive,r)"])

    eq("r1_red", "match apply_rule canon (R1Shorten 0 2) with Some s2 => length (red_edges s2) | None => 999 end", facts["r1_red"])
    m_iv = "match apply_rule canon (R1Shorten 0 2) with Some s2 => match st_evs s2 with e :: _ => (istart (ev_it e), iend (ev_it e)) | [] => (0,0) end | None => (0,0) end"
    eq("r1_iv", m_iv, facts["r1_iv"])
    eq("r2_storage", "match apply_rule canon (R2Split 1 2) with Some s2 => map ev_storage (st_evs s2) | None => [] end", coq_list(facts["r2_storage"]))
    m_r3 = "match apply_rule canon (R3Swap 0 2) with Some s2 => match st_evs s2 with e0 :: _ :: e2 :: _ => (istart (ev_it e0), iend (ev_it e0), istart (ev_it e2), iend (ev_it e2)) | _ => (0,0,0,0) end | None => (0,0,0,0) end"
    eq("r3_iv", m_r3, facts["r3_iv"])
    m_step = ("match applicable canon CommutativeTrim Guarded with "
              "r :: _ => (match apply_rule canon r with "
              "Some s2 => length (red_edges s2) | None => 999 end) "
              "| [] => 999 end")
    eq("ct_step_measure", m_step, facts["ct_step_measure"])

    eq("r4_red", "match apply_rule canon (R4Runtime 0 1) with Some s2 => length (red_edges s2) | None => 999 end", facts["r4_red"])
    eq("r4_runtime", "match apply_rule canon (R4Runtime 0 1) with Some s2 => st_runtime s2 | None => [] end", coq_list(facts["r4_runtime"]))

    GEN.write_text("\n".join(head + checks), encoding="utf-8")
    print(f"[reconcile] wrote {GEN.relative_to(ROOT)}")


def main():
    facts = rust_facts()
    keys = list(facts)
    print(f"[reconcile] Rust 事實 {len(keys)} 項: {', '.join(sorted(keys))}")
    gen(facts)
    p = subprocess.run(["coqc", *COQFLAGS, "reconcile_gen.v"], cwd=THEO, text=True,
                       capture_output=True)
    if p.returncode != 0:
        print("[reconcile] FAIL — 兩端語義分歧:", file=sys.stderr)
        print(p.stdout[-3000:], file=sys.stderr)
        print(p.stderr[-3000:], file=sys.stderr)
        return 1
    print("[reconcile] OK — Rust 語義與 Rocq 鏡像在全部樣本點一致(kernel 複驗)。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
