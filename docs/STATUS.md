# STATUS.json —— 數字單一來源(N1 / N2)

> 健檢 §P3-14 #5「數字單一來源」的落地。2026-09-03。

## 為什麼需要它

本專案的文件裡散落著**只能靠人手重跑才會更新**的數字:測試總數、宇宙規模、
覆蓋率表、對帳樣本點。沒有單一來源時,文件會靜默說謊 —— 健檢抓到的兩例:

| 案例 | 文件寫的 | 實況 |
|---|---|---|
| §P3-8 | 「4×6 + 過濾(**CI 現有規模**) | 623,616」(`R3-RESEARCH.md` / `ROCQ-TRACE.md` / `SPEC-TRACE.md` 三處) | CI 實際只跑 4×5 = **105,216**;623,616 是人工跑 `r3_probe` 抄來的,**沒有回歸保護**。<br>**處置(2026-09-03 M5)**:不降文件、升測試 —— `test_law_L9_scaled_space_joinable` 改跑 4×6 並以 `assert_eq!` 釘住 623,616 / 635,424,文件從此**由 CI 背書** |
| §P3-9 | 「3 事件 × 6 座標(**35,280** 狀態)+ 4 事件 × 5 座標(**105,216** 狀態)」(`tests/laws.rs:631`) | 該測試用**未過濾**的 `enumerate_states`,實為 **74,088 / 810,000** —— 測試比註解強 2.1× / 7.7×,但註解誤導 |

兩例都不是「寫錯字」,而是**數字沒有機械來源**。解法不是再校對一次,而是讓
文件裡那些數字**每次 CI 都被量測值對一遍**。

## 資料鏈(單向,不可倒流)

```
   docs/*.md、ROADMAP.md
            ▲
            │ 稽核:tools/docs_check.py     ← N2,CI job `docs-consistency`
            │
   docs/STATUS.json                        ← N1,CI 每次重新產生
            ▲
            │ 量測:tools/gen_status.py
            │
   ┌────────┼──────────────┬─────────────────┬──────────────────────┐
cargo test  cargo llvm-cov  make -C rocq      cargo run --release
-- --list    --lcov         reconcile         --example status_probe
(測試數)     (覆蓋率)        (Rocq/對帳)       (宇宙規模)
```

* **`gen_status.py` 不含任何手抄常數** —— 每個欄位都來自一次實際執行。
* 任何一個來源量不到 ⇒ **exit 1,絕不寫出半份 STATUS.json**
  (失敗即紅;半份真相比沒有真相更危險,見 `docs/COVERAGE.md` §三之二)。
* 庫裡的 `docs/STATUS.json` 只是**給人看的快照**;CI 每次重新產生,不信任快照。

## 欄位

| 區塊 | 來源 | 用途 |
|---|---|---|
| `tests.{unit,integration,total}` | `cargo test --lib -- --list` + `cargo test --test laws -- --list` | 文件裡的「N 具名測試」 |
| `coverage.<mod>.{lf,lh,ratio}` | lcov(**複用 `tools/cov_gate.py` 的解析器**,避免第二份解析器漂移) | `docs/COVERAGE.md` 的表格 |
| `rocq.{files,lines,admitted,axiom,reconcile_facts}` | 掃 `rocq/theories/*.v` + `make -C rocq reconcile` | 「0 Admitted / 0 Axiom」「19 樣本點」 |
| `universes.*` | `examples/status_probe`(如實量測,不做性質判斷) | 623,616 / 105,216 / 35,280 等 |

## 用法

```bash
python3 tools/gen_status.py            # 重新量測並寫出 docs/STATUS.json
python3 tools/docs_check.py            # 拿 STATUS.json 稽核文件
python3 tools/docs_check_selftest.py   # 稽核器自身的判別力自測(9 情境)
```

## 兩條紀律

1. **規則命中 0 次 = 失敗**(不是跳過)。文件被重整、句子被改寫後,規則若靜默
   失效,「文件一致性」就只剩一個 job 名字 —— 那正是 §P3-8 那類漂移的溫床。
   每條規則都設了 `min_hits`,CI 會逼人更新規則或文件。
2. **不對歷史敘事判紅,但也不因此放寬**。`release-v0.1.1/RELEASE.md` 整體排除
   (它描述的是**當時**的狀態);`docs/ROCQ-TRACE.md` 裡被引用的「「46 具名
   測試」」是歷史口徑,規則刻意只錨定同一句的「實跑 = 50」。
   「無 `Admitted`/`Axiom`」是**條件式**規則:只在程式碼真的出現 `Admitted`
   時才判紅 —— 程式碼乾淨時,文件那句話是真話。

## CI

`.github/workflows/ci.yml` → job `docs-consistency`:
`docs_check_selftest.py`(判別力)→ `gen_status.py`(重新量測)→
`docs_check.py`(稽核)→ 上傳 `STATUS.json` artifact。
