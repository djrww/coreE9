//! R3 紅邊遞減探針(Phase 3 主橋樑的實測前測)。
//!
//! 定理級的跳板是「剪短一個事件的右端點 ⇒ 紅邊計數**嚴格**遞減」——
//! 這是把 Raw 版 WCR 升格為 Guarded 版的關鍵(guard = sd(µ) true ⇔ 紅邊遞減)。
//! 本探針掃描枚舉宇宙,並在**可達閉包**上再掃一次(不只初始宇宙):
//!   (a) 有沒有一條 Raw CT 規則施作後紅邊計數**不**嚴格遞減(即 Guarded 剔除它);
//!   (b) Guarded 菜單 ≡ Raw 菜單 是否成立。
//!
//! 用法:cargo run --release --example r3_red [n_events] [max_coord] [--reach]

use cl0r0::rep::{apply, enumerate_states, AState, Menu, Policy};
use std::collections::{HashMap, HashSet};

fn report(s: &AState, universe: &str, out: &mut (usize, usize)) {
    let raw = Menu::CommutativeTrim.applicable(s, Policy::Raw);
    let guarded = Menu::CommutativeTrim.applicable(s, Policy::Guarded);
    let rset: HashSet<String> = raw.iter().map(|r| format!("{:?}", r)).collect();
    let gset: HashSet<String> = guarded.iter().map(|r| format!("{:?}", r)).collect();
    let mut not_dec = 0usize;
    for r in &raw {
        if let Some(s2) = apply(s, *r) {
            let m = s.measure();
            let m2 = s2.measure();
            if !AState::strictly_decreases(m2, m) {
                not_dec += 1;
                if out.0 + out.1 == 0 || (not_dec <= 2) {
                    println!(
                        "  [{universe}] !decrease  rule={:?}  µ=({},{})->({},{})",
                        r, m.0, m.1, m2.0, m2.1
                    );
                }
            }
        }
    }
    out.0 += not_dec;
    out.1 += if rset != gset { 1 } else { 0 };
}

fn key(s: &AState) -> Vec<(u32, u32, u32, u32)> {
    let mut v: Vec<_> = s
        .evs
        .iter()
        .map(|e| {
            (
                e.id,
                e.it.start,
                e.it.end,
                if e.kind == cl0r0::rep::K::Mut { 1 } else { 0 },
            )
        })
        .collect();
    v.sort();
    v
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(4);
    let m: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(6);
    let do_reach = args.iter().any(|a| a == "--reach");

    let enum_states = enumerate_states(n, m);
    println!("[r3_red] n={n} m={m} 枚舉宇宙={} 個狀態", enum_states.len());

    let mut enum_out = (0usize, 0usize);
    for s in &enum_states {
        report(s, "enum", &mut enum_out);
    }
    println!(
        "  enumerate: raw CT 規則施作後「紅邊未嚴格遞減」總次數 = {}, Guarded≠Raw 狀態數 = {}",
        enum_out.0, enum_out.1
    );

    if do_reach {
        // CT 可達閉包(Guarded 政策 —— 這才是被真正執行的系統)。   *
        let mut seen: HashMap<Vec<(u32, u32, u32, u32)>, AState> = HashMap::new();
        let mut frontier: Vec<AState> = enum_states.clone();
        for s in &frontier {
            seen.insert(key(s), s.clone());
        }
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for st in &frontier {
                for r in Menu::CommutativeTrim.applicable(st, Policy::Guarded) {
                    if let Some(s2) = apply(st, r) {
                        if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key(&s2)) {
                            e.insert(s2.clone());
                            next.push(s2);
                        }
                    }
                }
            }
            frontier = next;
        }
        println!("  reachable(Guarded) 狀態數 = {}", seen.len());
        let mut reach_out = (0usize, 0usize);
        for s in seen.values() {
            report(s, "reach", &mut reach_out);
        }
        println!(
            "  reach-closure: raw CT 規則施作後「紅邊未嚴格遞減」總次數 = {}, Guarded≠Raw 狀態數 = {}",
            reach_out.0, reach_out.1
        );
    }
}
