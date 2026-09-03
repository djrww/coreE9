#!/usr/bin/env python3
"""基準回歸門(P1 #7;2026-09-02 重寫,2026-09-03 健檢 P2-4/P2-5 修判定式)。

用法:
  python3 tools/bench_gate.py <bench.log> <baseline.json> [--tol 0.25]
                              [--noise-floor 0.05] [--max-slack 0.10]
                              [--null-k 2.0] [--tiny-us 0] [--strict]
                              [--no-null-correct] [--assume-env-stable]
                              [--update]      # 用本次執行結果重寫 baseline(同機刷新)
                              [--quiet]

★ 統計量的選擇(2026-09-02 用三趟獨立執行量出來的,別再改回 median):
    同一份代碼三趟的 median 趟間變異 cv = 10–24%,而 **min-of-min(best-of-n)**
    只有 0.3–3.3% —— median 把「搶核/降頻」的離群值吃進去了,在這台 2 核沙箱
    上會 flapping(實測:run2 vs run1/run3 基線曾被誤判紅)。故判定用 min,
    median 只用來**估計污染度**並作為次要條件。

──────────────────────────────────────────────────────────────────────────
判定(每個內核):

  ratio     = cur_min / base_min        # ★ 主判据(best-of-n)
  contam    = max(noise_floor, cur 的 median/min − 1, base 的 median/min − 1)
  base_tol  = max(2%, tol − noise_floor)          # 乾淨趟的基準有效容差(=20%)
  slack     = min(max(0, contam − noise_floor), max_slack)   # 單向放寬,≤10%
  lim       = tol (--strict)  否則  base_tol + slack
  corr      = null 漂移校正係數(見下;須通過顯著性門檻才啟用)
  ratio_corr= ratio / corr
  REGRESSED ⇔ ratio > 1+lim 且 ratio_corr > 1+lim
  improved  ⇔ ratio < 1−base_tol(改善判定不受本趟污染度影響)
  median < tiny-us 的內核標 informational(不参与紅綠)。

──────────────────────────────────────────────────────────────────────────
★ `contam`(污染度)是什麼,以及 2026-09-03 改了什麼

`contam = median/min − 1` 量的是「這一趟的**典型**輪比**最快**輪慢了多少」,
也就是機器在本趟被干擾的程度 —— 它是**同趟內的污染指標**,不是趟間變異,
也不等於 MAD%(舊 docstring 三種定義互相打架,本版統一為此定義)。

舊版的判定式寫 `eff_tol = max(2%, tol − band)`,但程式碼裡真正用的是
`tol − floor`(常數),`band` 只是印出來好看 —— 文件、註解、實作三者不一致。
更糟的是「抵免」這個方向本身是反的:`tol − band` 讓**越吵的內核容差越小**
(實測 newman 污染 14% ⇒ 只給 11%,而乾淨的 l7b 拿到 20%),
但最容易被誤紅的正是最吵的那個。

本版改為**單向放寬**:污染只會讓門檻變鬆或持平,永不變嚴;且設上限
(`--max-slack`,預設 10pp),避免「疊加成 ±36% 而放過真回歸」的舊顧慮。
判別力由 `tools/bench_gate_selftest.py` 的「污染度低→紅 / 同樣數字但污染度高→綠」
情境對釘住。

──────────────────────────────────────────────────────────────────────────
★ null 環境標尺的顯著性門檻(2026-09-03 健檢 P2-5)

`corr = cur_null_median / base_null_median` 用來把「整台機器變慢」從比值裡除掉。
但 null 是全表最小、最接近計時底噪的內核(實測 median 16 ns,min 15 / max 20 ns),
純雜訊就能讓它漂移兩位數百分比。實測:同機、同碼、連續兩趟,corr = 0.941 / 0.882,
而 `corr > 1` 會**單向放寬**門檻 ⇒ 有效紅線在 1.20× ~ 1.34× 之間游移(14 個百分點)。

本版加顯著性門檻:只有當
      |corr − 1| > k × max(cur/base 的 null 相對 MAD, 2%)
(k = `--null-k`,預設 2.0)才認定為「真的環境漂移」並啟用校正;否則 corr := 1.0。
實測效果:上面那兩趟(−5.9% / −11.8%)都落在噪聲帶內 ⇒ 不校正 ⇒ 紅線穩定在 1.20×。
合成資料中的「整機慢 30%(null 同步)」仍在門檻外 ⇒ 照常校正(不誤判)。

⚠ 固有盲区:若「所有內核與 null 一起變慢」(例如某 commit 讓全鏈路變重),
  比值法无法區分「環境」與「代碼」。此時用 `--assume-env-stable` 關閉校正
  (等於只信 raw),或同機 `--update` 重刷基線 —— 這點與重寫前相同,未改變。
──────────────────────────────────────────────────────────────────────────

輸出:每項 base/cur/ratio/contam/有效紅線/verdict + 漂移診斷;有 REGRESSED → exit 1。
"""
import json
import re
import sys

TINY_INFO_DEFAULT_US = 0.0
MIN_EFF_TOL = 0.02
# 2026-09-02 實測:本機 best-of-9 的**趟間**變異 cv = 0.3–3.3%(median 為 10–24%)
# ⇒ floor 設 5% 即可,乾淨趟的有效容差 = 25% − 5% = 20%。换機器請重量。
NULL_DRIFT_WARN = 0.05
NULL_ANOMALY = 0.25
# 2026-09-03 新增
MAX_CONTAM_SLACK = 0.10   # 污染度可放寬的上限(單向,不疊加到失控)
NULL_NOISE_K = 2.0        # null 漂移須超過 k × 其自身相對噪聲才算「真漂移」
NULL_REL_FLOOR = 0.02     # null 相對噪聲的下限(別把過小的 MAD 當成「量得很準」)


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
    max_slack, null_k = MAX_CONTAM_SLACK, NULL_NOISE_K
    strict = quiet = update = no_null = False
    assume_env_stable = False
    for flag in ("--tol", "--noise-floor", "--tiny-us", "--max-slack", "--null-k"):
        if flag in args:
            i = args.index(flag)
            v = float(args[i + 1])
            del args[i : i + 2]
            if flag == "--tol":
                tol = v
            elif flag == "--noise-floor":
                floor = v
            elif flag == "--max-slack":
                max_slack = v
            elif flag == "--null-k":
                null_k = v
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

    # 乾淨趟的基準有效容差(污染度 ≤ floor 時就是它)
    base_tol = tol if strict else max(MIN_EFF_TOL, tol - floor)

    print(
        f"{'kernel':<18}{'base':>10}{'cur':>10}{'best':>8}{'median':>8}"
        f"{'contam':>8}{'red@':>7}  verdict"
    )
    rows, bad, info, med_warn = [], [], [], []
    # 環境漂移:null 參考內核(缺失時為 1.0 = 不校正)
    corr, drift_note = 1.0, ""
    if "null" in cur and "null" in bkeys and not no_null:
        cn, cmed_n, cmad_n, _ = stat(cur["null"])
        bn, bmed_n, bmad_n, _ = stat(bkeys["null"])
        cn, bn = cmed_n, bmed_n  # null 的 best 無意義(太小),用 median
        if bn > 0 and cn > 0:
            raw = cn / bn
            # null 自身的相對噪聲(兩邊取大者,並設下限)
            rel_c = (cmad_n / cmed_n) if cmed_n else 0.0
            rel_b = (bmad_n / bmed_n) if bmed_n else 0.0
            null_rel = max(rel_c, rel_b, NULL_REL_FLOOR)
            sig = null_k * null_rel
            d = raw - 1.0
            if abs(d) > sig:
                corr = raw
                drift_note = (
                    f"null 參考內核 {d * 100:+.1f}% ⇒ 環境漂移校正係數 {corr:.3f}"
                    f"(顯著:>{null_k:g}× 噪聲 {sig * 100:.1f}%,已按此縮放判定)"
                )
            else:
                corr = 1.0
                drift_note = (
                    f"null 參考內核 {d * 100:+.1f}% —— 未過顯著性門檻"
                    f"(|漂移| ≤ {null_k:g}× 噪聲 = {sig * 100:.1f}%)⇒ **不校正**"
                    f"(讓 16 ns 的底噪挪動紅線比不校正更危險)"
                )
    # 註:刻意**不**因環境漂移無條件放寬 tol —— 放寬會讓「所有內核一起慢」的
    # 真回歸被一起吃掉(本輪用合成資料驗出)。環境漂移只透過 corr(比值除法)
    # 處理,且須先通過上面的顯著性門檻;若漂移過大,下面單獨提出警告。
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
            rows.append((k, None, cm, None, None, None, None, "missing-in-baseline"))
            bad.append(k)
            continue
        bm, bmed, bmad, breps = stat(b)
        if bm <= 0:
            rows.append((k, bm, cm, None, None, None, None, "no-baseline-value"))
            continue
        ratio = cm / bm                       # best-of-n 比值 = 主判据
        # 污染度:median 比 min 慢多少(此機此輪被搶走多少時間)
        sp_c = (cmed / cm - 1.0) if cm else 0.0
        sp_b = (bmed / bm - 1.0) if bm else 0.0
        contam = max(floor, sp_c, sp_b)
        if tiny and cm * 1000.0 < tiny:
            rows.append((k, bm, cm, ratio, None, contam, None, f"info(tiny<{tiny:g}µs)"))
            info.append(k)
            continue
        rc = ratio / corr
        # ★ 單向放寬:污染只讓門檻變鬆或持平,永不變嚴(健檢 P2-4)
        slack = min(max(0.0, contam - floor), max_slack)
        lim = tol if strict else base_tol + slack
        med_ratio = cmed / bmed if bmed else 1.0
        if ratio > 1.0 + lim:
            if rc > 1.0 + lim:
                verdict = "REGRESSED"
                bad.append(k)
            else:
                verdict = f"ok(env-explained:best {ratio:.3f} 超標、扣漂移 {corr:.3f} 後在容差內)"
        elif ratio < 1.0 - base_tol:
            # 改善判定不受本趟污染度影響(污染只放寬「紅」的那一側)
            verdict = "improved(建議 --update 刷新基線)"
        else:
            verdict = "ok"
        if med_ratio > 1.0 + base_tol:
            med_warn.append(f"{k}({med_ratio:.2f}×;best 僅 {ratio:.2f}×)")
        # best-of-n 只需少量重複即可收斂(實測 n=5 的 ms 級內核穩定);
        # n<3 才有「下限未收斂」的風險。
        warn = f"  ⚠ n={creps}<3:best 下限未收斂" if creps < 3 else ""
        rows.append((k, bm, cm, ratio, med_ratio, contam, lim, verdict + warn))

    def fmt(v, w, prec, suffix=""):
        if v is None:
            return f"{'-':>{w}}"
        return f"{v:>{w}.{prec}f}{suffix}"

    for k, bm, cm, ratio, med_ratio, contam, lim, verdict in rows:
        print(
            f"{k:<18}"
            + fmt(bm, 10, 4)
            + fmt(cm, 10, 4)
            + fmt(ratio, 8, 3)
            + fmt(med_ratio, 8, 3)
            + fmt(None if contam is None else contam * 100, 7, 1, "%")
            + fmt(None if lim is None else 1.0 + lim, 7, 2, "×")
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
        lims = [r[6] for r in rows if r[6] is not None]
        if lims:
            rng = f"有效紅線 {1.0 + min(lims):.2f}× ~ {1.0 + max(lims):.2f}×"
        else:
            rng = "有效紅線 (全部 informational)"
        env = "關" if no_null else ("開" if corr != 1.0 else "開(本趟未達顯著,未校正)")
        print(
            f"bench gate: ok(tol {tol * 100:.0f}% − 名目 {floor * 100:.0f}% "
            f"+ 污染單向放寬 ≤{max_slack * 100:.0f}%;{rng};"
            f"環境校正={env};harness={harness})"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
