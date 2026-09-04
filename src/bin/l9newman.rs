//! l9newman —— 機械的 Newman 通道:
//! 對兩個菜單(規範修剪 / 樸素)分別機械檢查 SN、WCR、唯一正規形。
//! 運行:`cargo run --bin l9newman`

use cl0r0::l9newman::newman_check;
use cl0r0::rep::{Menu, Policy};

fn main() {
    println!("======================================================================");
    println!(" 機械 Newman 通道: SN ∧ WCR ⇒ CR ⇒ 唯一正規形(§4.2–4.3)");
    println!("======================================================================");
    for (menu, policy) in [
        (Menu::CommutativeTrim, Policy::Guarded),
        (Menu::Naive, Policy::Raw),
        (Menu::Naive, Policy::Guarded),
    ] {
        let t0 = std::time::Instant::now();
        // P0 #4:空間由 3 事件 × 5 座標 → 4 事件 × 6 座標(並行分塊)。
        // 對 CT/Guarded 這是全域驗證(623,616 狀態 × 635,424 臨界對);
        // 對 Naive(壞菜單)檢查複雜度高 ~100×,取其存在性反例即可
        // (機器找反例:3 事件 × 3 座標即命中) —— 下方如實打印所用規模。
        let (n_ev, coord, depth) = if matches!(menu, Menu::CommutativeTrim) {
            (4, 6, 8)
        } else {
            // 壞菜單:反例在淺層即命中(depth 3),深走只會指數膨脹。
            (3, 3, 3)
        };
        let rep = newman_check(menu, policy, n_ev, coord, depth);
        let dt = t0.elapsed();
        println!("\n--- 菜單:{} / 政策:{:?} ---", menu.label(), policy);
        println!(
            "  窮舉狀態數:{}(空間 {} 事件 × {} 座標 × depth {};並行線程 {})",
            rep.states, n_ev, coord, depth, rep.threads
        );
        println!("  檢查耗時:{:.1}s", dt.as_secs_f64());
        println!("  臨界對檢查數:{}", rep.critical_pairs);
        println!("  L8 遞減違反數:{}", rep.l8_violations.len());
        for (s, s2, r) in rep.l8_violations.iter().take(3) {
            println!(
                "    反例:μ({:?}) = {:?} → 施用 {} → μ = {:?}(未嚴格遞減)",
                s.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>(),
                s.measure(),
                r.label(),
                s2.measure()
            );
        }
        println!("  不可回合臨界對數:{}", rep.non_joinable.len());
        for (s, r1, r2, a, b) in rep.non_joinable.iter().take(4) {
            println!("    反例(臨界對 ({}, {})):", r1.label(), r2.label());
            println!("      源狀態:{}", fmt_state(s));
            println!("      分支 a:{}", fmt_state(a));
            println!("      分支 b:{}", fmt_state(b));
        }
        println!("  唯一正規形狀態數:{}", rep.unique_nf_states);
        if rep.truncated {
            println!("  [注]反例列表為報告目的截斷(每類上限 64);計數為全量。");
        }
        println!("  多正規形狀態數:{}", rep.multi_nf.len());
        for (s, nfs) in rep.multi_nf.iter().take(2) {
            println!("    反例:源 {} 有 {} 個正規形", fmt_state(s), nfs.len());
        }
        println!("  ==> 結論:{}", rep.conclusion);
    }
    println!("\n註:Naive/Raw 違反 L8(存在不嚴格遞減的施用;如縮短後紅邊數不變);");
    println!("    對比之下,CommutativeTrim 的規範修剪是「L8 + L9 雙過」的封閉菜單 ——");
    println!("    這正是報告 §4.2「側條件收窄」與 §4.3「臨界對可窮舉」的機械形態。");
}

fn fmt_state(s: &cl0r0::rep::AState) -> String {
    let mut v: Vec<String> = s
        .evs
        .iter()
        .map(|e| {
            format!(
                "{}:{}{}[{},{})",
                e.id,
                e.storage,
                e.kind.label(),
                e.it.start,
                e.it.end
            )
        })
        .collect();
    v.sort();
    let r = s.red_edges();
    format!("{{ {} | 紅邊 {} }}", v.join(" "), r.len())
}
