#!/usr/bin/env python3
"""基準回歸門(P1 #7;2026-09-02 重寫)。

用法:
  python3 tools/bench_gate.py <bench.log> <baseline.json> [--tol 0.25]
                              [--noise-floor 0.05] [--tiny-us 0] [--strict]
                              [--no-null-correct]
                              [--update]      # 用本次執行結果重寫 baseline(同機刷新)
                              [--quiet]

★ 統計量的選擇(2026-09-02 用三趟獨立執行量出來的,別再改回 median):
    同一份代碼三趟的 median 趟間變異 cv = 10–24%,而 **min-of-min(best-of-n)**
    只有 0.3–3.3% —— median 把「搶核/降頻」的離群值吃進去了,在這台 2 核沙箱
    上會 flapping(實測:run2 vs run1/run3 基線曾被誤判紅)。故判定用 min,
    median 只用來**估計污染度**並作為次要條件。

判定(每個內核):
  ratio_raw  = cur_min / base_min   (best-of-n;環境標尺 drift 另算)
  corr       = cur_null / base_null                  # 環境漂移(null 參考內核)
  ratio_corr = ratio_raw / corr                      # 扣掉環境後的代碼比值
  band       = max(noise_floor, mad%_cur, mad%_base) # 實測雜訊(mad/median)
  eff_tol    = max(0.02, tol - band)                 # 雜訊**抵免**容差,不是疊加
  REGRESSED 需 ratio_raw > 1+eff_tol 且 ratio_corr > 1+eff_tol
  (只 raw 超標 = 環境變慢;只 corr 超標不會發生,但保留雙條件可防
   null 內核自身被污染的情況)。improved 為兩側皆 < 1-。
  median < tiny-us 的內核標 informational(不参与紅綠)。
  基線缺 null(舊基線)時 corr := 1.0,自動退化為純 ratio_raw。
  ⚠ 固有盲区:若「所有內核與 null 一起變慢」(例如某 commit 讓全鏈路變重),
    比值法无法區分「環境」與「代碼」。此時用 --assume-env-stable 關閉校正
    (等於只信 raw);同機重跑或 --update 刷新基線是正解。
  --strict:忽略 band(eff_tol = tol),發佈前擰緊用。

為什麼不再是「裸 ratio > 1.25 就紅」(舊版在 main HEAD 長期紅的原因):
  * 舊基線是**另一台機器單次採樣**;跨機漂移本身就常有 20–35%,把它當成代碼回歸
    是誤判。故:(a) 基線改由**同機** `--update` 刷新;(b) 容差加上雜訊帶,
    雜訊由**實測的 MAD** 推得,而非拍腦袋。
  * 舊 harness 只跑 5 輪、單樣本 µs 級 ⇒ 改用 n≥7 + 每輪整批語料(ms 級),
    並把 MAD 寫進基線,讓「這輪到底穩不穩」變成可檢查的數字。
  * `null`(環境參考內核)只作**診斷**:若 null 自己漂了 >5%,先懷疑機器,
    不直接參與紅綠(它的絕對值太接近計時底噪,拿来歸一會把噪聲放大)。
輸出:每項 base/cur/ratio/noise/verdict + 漂移診斷;有 REGRESSED → exit 1。
"""
import json
import re
import sys

TINY_INFO_DEFAULT_US = 0.0
MIN_EFF_TOL = 0.02
# 2026-09-02 實測:本機 best-of-9 的**趟間**變異 cv = 0.3–3.3%(median 為 10–24%)
# ⇒ floor 設 5% 即可,容差實際為 25% − 5% = 20%。换機器請重量。
NULL_DRIFT_WARN = 0.05
NULL_ANOMALY = 0.25


def load_payload(text):
    """吃新格式(BENCH_JSON:{"kernels":{...}})與舊格式(BENCH_JSON:{flat})。"""
    m = re.search(r"BENCH_JSON:(\{.*\})", text)
    if not m:
        return None
    doc = json.loads(m.group(1))
    if "kernels" in doc:
        return doc["kernels"], doc.get("harness", "?")
    # 舊格式:flat {name: ms}
    return {k: as_entry(v) for k, v in doc.items()}, "legacy"


def as_entry(v):
    """新格式是 dict;舊基線 `values` 是裸 float → 正規化成 dict(MAD 視為未知)。"""
    if isinstance(v, dict):
        return v
    return {"median_ms_per_sample": float(v), "mad_ms": 0.0, "reps": 1}


def stat(entry):
    """回傳 (best, med, mad, reps)。best = 該輪最小值 = 無污染下限。"""
    entry = as_entry(entry)
    med = float(entry.get("median_ms_per_sample", 0.0))
    mad = float(entry.get("mad_ms", 0.0))
    mn = float(entry.get("min_ms", med))
    reps = int(entry.get("reps", 1))
    return mn, med, mad, reps


def main(argv):
    args = list(argv)
    tol, floor, tiny = 0.25, 0.05, TINY_INFO_DEFAULT_US
    strict = quiet = update = no_null = False
    assume_env_stable = False
    for flag in ("--tol", "--noise-floor", "--tiny-us"):
        if flag in args:
            i = args.index(flag)
            v = float(args[i + 1])
            del args[i : i + 2]
            if flag == "--tol":
                tol = v
            elif flag == "--noise-floor":
                floor = v
            else:
                tiny = v
    if "--assume-env-stable" in args:
        assume_env_stable = no_null = True
        args.remove("--assume-env-stable")
    if "--no-null-correct" in args:
        no_null = True
        args.remove("--no-null-correct")
    if "--strict" in args:
        strict, _ = True, args.remove("--strict")
    if "--quiet" in args:
        quiet, _ = True, args.remove("--quiet")
    if "--update" in args:
        update, _ = True, args.remove("--update")
    if len(args) < 2:
        print(__doc__)
        return 2
    log_path, base_path = args[0], args[1]

    text = open(log_path, encoding="utf-8").read()
    loaded = load_payload(text)
    if not loaded:
        print("NO BENCH_JSON FOUND", file=sys.stderr)
        return 2
    cur, harness = loaded

    try:
        bdoc = json.load(open(base_path, encoding="utf-8"))
    except FileNotFoundError:
        bdoc = {}
    bkeys = bdoc.get("kernels") or bdoc.get("values") or {}
    bkeys = {k: as_entry(v) for k, v in bkeys.items()}

    if update:
        doc = {
            "note": (
                "hotpaths 基線(同機刷新;tools/bench_gate.py --update)。"
                "單位 ms/樣本 = n≥7 輪之中位數,mad_ms 為其 MAD。"
                "`null` 為環境參考內核(僅診斷漂移用)。"
                "換機器/換 rustc 後請重新 --update,勿用跨機基線硬比。"
            ),
            "harness": harness,
            "kernels": cur,
        }
        with open(base_path, "w", encoding="utf-8") as f:
            json.dump(doc, f, ensure_ascii=False, indent=2)
            f.write("\n")
        print(f"baseline updated: {base_path}({len(cur)} 內核,同機)")
        return 0

    print(f"{'kernel':<18}{'base':>10}{'cur':>10}{'best':>8}{'median':>8}{'band':>8}  verdict")
    rows, bad, info, med_warn = [], [], [], []
    # 環境漂移:null 參考內核(缺失時為 1.0 = 不校正)
    corr, drift_note = 1.0, ""
    if "null" in cur and "null" in bkeys and not no_null:
        cn, cmed_n, _, _ = stat(cur["null"])
        bn, bmed_n, _, _ = stat(bkeys["null"])
        cn, bn = cmed_n, bmed_n  # null 的 best 無意義(太小),用 median
        if bn > 0 and cn > 0:
            corr = cn / bn
            d = corr - 1.0
            drift_note = (
                f"null 參考內核 {d * 100:+.1f}% ⇒ 環境漂移校正係數 {corr:.3f}"
                + ("(環境不穩,已按此縮放判定)" if abs(d) > NULL_DRIFT_WARN else "(環境穩定)")
            )
    # 註:刻意**不**因環境漂移放寬 tol —— 放寬會讓「所有內核一起慢」的
    # 真回歸被一起吃掉(本輪用合成資料驗出)。環境漂移只透過 corr(比值除法)
    # 處理;若漂移過大,下面單獨提出警告,由人決定是否重跑/換機器。
    anomaly = ""
    if abs(corr - 1.0) > NULL_ANOMALY:
        anomaly = (
            f"⚠ null 漂移 {(corr - 1) * 100:+.1f}% 超過 {NULL_ANOMALY * 100:.0f}% 門檻"
            " ⇒ 環境異常(搶核/降頻),建議重跑;判定仍以 raw+corr 雙條件為準"
        )

    for k, centry in cur.items():
        if k == "null":
            continue
        cm, cmed, cmad, creps = stat(centry)
        b = bkeys.get(k)
        if b is None:
            rows.append((k, None, cm, None, None, None, "missing-in-baseline"))
            bad.append(k)
            continue
        bm, bmed, bmad, breps = stat(b)
        if bm <= 0:
            rows.append((k, bm, cm, None, None, None, "no-baseline-value"))
            continue
        ratio = cm / bm                       # best-of-n 比值 = 主判据
        # 污染度:median 比 min 慢多少(此機此輪被搶走多少時間)
        sp_c = (cmed / cm - 1.0) if cm else 0.0
        sp_b = (bmed / bm - 1.0) if bm else 0.0
        noise = max(floor, sp_c, sp_b)
        if tiny and cm * 1000.0 < tiny:
            rows.append((k, bm, cm, ratio, None, noise, f"info(tiny<{tiny:g}µs)"))
            info.append(k)
            continue
        rc = ratio / corr
        lim = tol if strict else max(MIN_EFF_TOL, tol - floor)
        med_ratio = cmed / bmed if bmed else 1.0
        if ratio > 1.0 + lim:
            if rc > 1.0 + lim:
                verdict = "REGRESSED"
                bad.append(k)
            else:
                verdict = f"ok(env-explained:best {ratio:.3f} 超標、扣漂移 {corr:.3f} 後在容差內)"
        elif ratio < 1.0 - lim:
            verdict = "improved(建議 --update 刷新基線)"
        else:
            verdict = "ok"
        if med_ratio > 1.0 + (tol if strict else max(MIN_EFF_TOL, tol - floor)):
            med_warn.append(f"{k}({med_ratio:.2f}×;best 僅 {ratio:.2f}×)")
        # best-of-n 只需少量重複即可收斂(實測 n=5 的 ms 級內核穩定);
        # n<3 才有「下限未收斂」的風險。
        warn = f"  ⚠ n={creps}<3:best 下限未收斂" if creps < 3 else ""
        rows.append((k, bm, cm, ratio, med_ratio, noise, verdict + warn))

    def fmt(v, w, prec, suffix=""):
        if v is None:
            return f"{'-':>{w}}"
        return f"{v:>{w}.{prec}f}{suffix}"

    for k, bm, cm, ratio, med_ratio, noise, verdict in rows:
        print(
            f"{k:<18}"
            + fmt(bm, 10, 4)
            + fmt(cm, 10, 4)
            + fmt(ratio, 8, 3)
            + fmt(med_ratio, 8, 3)
            + fmt(None if noise is None else noise * 100, 7, 1, "%")
            + f"  {verdict}"
        )
    if drift_note and not quiet:
        print(f"---- {drift_note}")
    if anomaly:
        print(f"---- {anomaly}")
    if info and not quiet:
        print(f"---- informational(未判紅):{', '.join(info)}")
    if med_warn:
        print("---- median 超標(環境污染線索,不判紅):" + ", ".join(med_warn))
    if bad:
        print(f"REGRESSION beyond +{tol * 100:.0f}% 扣雜訊帶: {', '.join(sorted(set(bad)))}")
        return 1
    if not quiet:
        band = "不抵免" if strict else f"{tol * 100:.0f}% 扣雜訊帶"
        print(
            f"bench gate: ok(tol {band};環境校正={'關' if no_null else '開'};harness={harness})"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
