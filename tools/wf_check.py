#!/usr/bin/env python3
""".github/workflows/*.yml 的結構健檢。

為什麼需要它(2026-09-03 的教訓):

  把 `env:` 區塊的內容清空、只留註解之後,`yaml.safe_load` **照樣通過**
  (它會解析成 `env: None`),但 GitHub **拒絕整個 workflow** —— 表現是
  workflow 的名稱退化成原始檔名 `.github/workflows/ci.yml`,而且**一次 run
  都不產生**(`jobs: total_count = 0`)。

  那是這整份稽核在追的那類失效的最壞形式:CI 不是「紅」,是**不存在**。
  而「CI 全綠」的宣稱仍可繼續成立 —— 因為根本沒有人跑過。

本腳本是針對這個失效模式的便宜防線:不驗證 GitHub 的完整 schema,只驗證
幾個「寫壞了會讓 workflow 靜默消失」的不變量。

用法:python3 tools/wf_check.py [.github/workflows]
"""
import glob
import os
import sys

try:
    import yaml
except ImportError:
    print("wf_check: 需要 PyYAML(pip install pyyaml)", file=sys.stderr)
    sys.exit(2)


def check(path):
    """回傳問題清單(空串列代表正常)。"""
    errs = []
    try:
        d = yaml.safe_load(open(path, encoding="utf-8"))
    except Exception as e:  # noqa: BLE001
        return [f"YAML 解析失敗: {e}"]

    if not isinstance(d, dict):
        return ["最上層不是 mapping"]

    # (1) name:缺失時 GitHub 會用檔名顯示,是「workflow 被拒」的第一個徵兆
    if not d.get("name"):
        errs.append("缺少 name: —— GitHub 會退化成顯示檔名")

    # (2) 觸發條件必須存在。
    #     注意:PyYAML 依 YAML 1.1 把裸的 `on` 解析成布林 True,
    #     所以鍵是 Python 的 True 而不是字串 "on"。
    if "on" not in d and True not in d and "True" not in d:
        errs.append("缺少 on:(觸發條件)")

    # (3) top-level env 若出現必須是 mapping(不能是 null)
    if "env" in d and not isinstance(d["env"], dict):
        errs.append(f"top-level env 是 {type(d['env']).__name__},必須是 mapping")

    jobs = d.get("jobs")
    if not isinstance(jobs, dict) or not jobs:
        errs.append("jobs 缺失或為空")
        return errs

    for jname, job in jobs.items():
        if not isinstance(job, dict):
            errs.append(f"job `{jname}` 不是 mapping")
            continue
        # (4) job 層 env/strategy 若出現必須是 mapping
        for key in ("env", "strategy", "with"):
            if key in job and not isinstance(job[key], dict):
                errs.append(
                    f"job `{jname}` 的 {key} 是 {type(job[key]).__name__},必須是 mapping"
                    " —— 只留註解的區塊會被解析成 None,GitHub 會拒絕整個 workflow"
                )
        # (5) runs-on
        if not job.get("runs-on"):
            errs.append(f"job `{jname}` 缺少 runs-on")
        # (6) steps:每個 step 要有 uses 或 run
        steps = job.get("steps")
        if not isinstance(steps, list) or not steps:
            errs.append(f"job `{jname}` 的 steps 缺失或為空")
            continue
        for i, st in enumerate(steps):
            if not isinstance(st, dict):
                errs.append(f"job `{jname}` step[{i}] 不是 mapping")
                continue
            if "uses" not in st and "run" not in st:
                errs.append(f"job `{jname}` step[{i}] 既無 uses 也無 run")
            # (7) step 層 env/with 同理
            for key in ("env", "with"):
                if key in st and not isinstance(st[key], dict):
                    errs.append(
                        f"job `{jname}` step[{i}] 的 {key} 是 "
                        f"{type(st[key]).__name__},必須是 mapping"
                    )
    return errs


def main(argv):
    root = argv[0] if argv else ".github/workflows"
    files = sorted(glob.glob(os.path.join(root, "*.yml")) +
                   glob.glob(os.path.join(root, "*.yaml")))
    if not files:
        print(f"wf_check: 在 {root} 找不到任何 workflow 檔 —— 這本身就是問題",
              file=sys.stderr)
        return 1
    bad = 0
    for f in files:
        errs = check(f)
        if errs:
            bad += 1
            print(f"❌ {f}")
            for e in errs:
                print(f"     {e}")
        else:
            print(f"✅ {f}")
    if bad:
        print(f"wf_check: {bad} 個 workflow 檔有問題", file=sys.stderr)
        return 1
    print(f"wf_check: ok({len(files)} 個 workflow 檔,結構不變量全數成立)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
