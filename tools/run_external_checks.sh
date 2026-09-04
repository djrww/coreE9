#!/usr/bin/env sh
# 外部工具交叉驗證一鍵腳本(不進 CI —— 對應 R3-RESEARCH §三「採納為測試對照」)。
# 用法:sh tools/run_external_checks.sh
# 依賴:maude(apt)、NaTT 2.3 自行編譯(NATT_EXE 指到 bin/NaTT.exe)、z3(NaTT 後端)。
set -eu

NATT_EXE="${NATT_EXE:-$(dirname "$0")/natt/NaTT.exe}"

fail() { echo "EXTERNAL-CHECK FAIL: $1" >&2; exit 1; }

echo "== [1/2] Maude: CT 菜單宇宙級性質(P1..P5 + 倒掛宇宙 P2)=="
maude -no-banner "$(dirname "$0")/ct_maude.maude" | tee /tmp/ct_maude.out >/dev/null
# 計數契約:1728 / 8000 / 32768
grep -q "result NzNat: 1728"  /tmp/ct_maude.out || fail "count(3,3)"
grep -q "result NzNat: 8000"  /tmp/ct_maude.out || fail "count(3,4)"
grep -q "result NzNat: 32768" /tmp/ct_maude.out || fail "count-wide(3,3)"
# 五性質全綠 ×3 個宇宙(3,3)(3,4)(3,5)+ 倒掛宇宙 P2
[ "$(grep -c 'result Bool: true' /tmp/ct_maude.out)" = 4 ] || fail "allOK/allDia"
echo "   Maude: 4 條 result Bool: true + 3 條計數契約 OK"

echo "== [2/2] NaTT:R1 過度近似超集 TRS 的終止證明 =="
"$NATT_EXE" "$(dirname "$0")/natt/r1_sup.xtc" | tee /tmp/natt.out >/dev/null
grep -q '^YES$' /tmp/natt.out || fail "NaTT termination"
echo "   NaTT: YES(多重集/多項式權重,0.02s 級)"

echo "== EXTERNAL CHECKS ALL GREEN =="
