//! 狀態探針(N1):供 `tools/gen_status.py` 產生 `docs/STATUS.json`。
//!
//! 為什麼要有它:文件裡的宇宙規模(623,616 / 810,000 / 35,280 …)過去只能靠人手
//! 跑 `r3_probe` 再抄進 markdown —— 抄錯沒人會發現(健檢 §P3-8 / §P3-9 就是這樣
//! 產生的)。這裡把它們變成**可機械讀取**的一行 JSON,由 `gen_status.py` 消費、
//! 再由 `tools/docs_check.py` 回頭稽核文件,形成
//!   **文件 ← STATUS.json ← 本探針(實測) ← 生成器** 的單向鏈。
//!
//! 與 `r3_probe` 的分工:`r3_probe` 做完整的 P1/P2/P3 性質檢查(慢、人讀);
//! 本探針只回報**計數**(快、機讀),不做性質判斷 —— 性質由 `tests/laws.rs`
//! 以 `assert_eq!` 釘住,兩者互為對照:一個量規模,一個斷性質。
//!
//! 運行:`cargo run --release --example status_probe`(實測 ~6 s)
//! 輸出:一行 `STATUS_PROBE_JSON:{...}`(與 hotpaths 的 `BENCH_JSON:` 同風格)。
use cl0r0::l9newman::newman_check;
use cl0r0::rep::{enumerate_states, Menu, Policy};

/// `newman_check` 的狀態數:座標上限 `m`、depth `d`(depth 只影響回合搜尋深度,
/// 不影響狀態數與臨界對數 —— 實測 (4,5,4) 與 (4,5,8) 同為 105,216/100,392)。
fn newman(n: usize, m: u32, d: usize) -> (usize, usize) {
    let r = newman_check(Menu::CommutativeTrim, Policy::Guarded, n, m, d);
    (r.states, r.critical_pairs)
}

fn main() {
    let (s36, p36) = newman(3, 6, 4);
    let (s45, p45) = newman(4, 5, 8);
    // ← `test_law_L9_scaled_space_joinable` 的 CI 強制規模(M5:4×5 → 4×6)
    let (s46, p46) = newman(4, 6, 8);
    // L9b′ 精確交換測試所用的**未過濾**宇宙(`enumerate_states` 原樣,不加
    // distinct-start 過濾)—— 該測試註解曾誤用有過濾的數字,故在此如實量測。
    let e36 = enumerate_states(3, 6).len();
    let e45 = enumerate_states(4, 5).len();

    println!(
        "STATUS_PROBE_JSON:{{\"newman_3x6\":{{\"states\":{s36},\"critical_pairs\":{p36}}},\
\"newman_4x5\":{{\"states\":{s45},\"critical_pairs\":{p45}}},\
\"newman_4x6\":{{\"states\":{s46},\"critical_pairs\":{p46}}},\
\"enumerate_3x6_unfiltered\":{e36},\"enumerate_4x5_unfiltered\":{e45}}}"
    );
}
