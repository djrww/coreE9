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
        let rep = newman_check(menu, policy, 3, 5, 8);
        println!("\n--- 菜單:{} / 政策:{:?} ---", menu.label(), policy);
        println!("  窮舉狀態數:{}", rep.states);
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
