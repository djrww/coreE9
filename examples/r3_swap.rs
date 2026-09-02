//! R3 交換引理的「精確版」探針:不用深度搜索,直接檢查
//!   s --ra--> a --rb--> c  與  s --rb--> b --ra--> c'   是否 c == c'
//! (即 parallel moves 的严格交换,而非 merely joinable)。
//!
//! 用法:cargo run --release --example r3_swap [n] [m] [--nofilter]
//! 這是 Rocq 側 `ct_join_exact` 引理的經驗前測:若全綠,Phase 3 可用
//! 一条「交换引理 + 紅邊單調性」的乾淨證明;若有違反,證明必須改寫成
//! 深度 ≤ 2 的菱形(需在 Rocq 裡多跑一步)或退到反射式證書。

use cl0r0::rep::{apply, enumerate_states, AState, Menu, Policy};
use std::collections::HashSet;

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
    let n: usize = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(3);
    let m: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(6);
    let filtered = !args.iter().any(|a| a == "--nofilter");

    let states: Vec<AState> = enumerate_states(n, m)
        .into_iter()
        .filter(|s| {
            if !filtered {
                return true;
            }
            let mut st: Vec<u32> = s.evs.iter().map(|e| e.it.start).collect();
            st.sort();
            st.windows(2).all(|w| w[0] != w[1])
        })
        .collect();

    let mut exact = 0usize;
    let mut lazy_join = 0usize;
    let mut broken = 0usize;
    let mut red_nondecreasing = 0usize;
    let mut first = String::new();

    for s in &states {
        // 紅邊單調性:任何 CT 步驟不增紅邊數
        for r in Menu::CommutativeTrim.applicable(s, Policy::Raw) {
            if let Some(s2) = apply(s, r) {
                if s2.red_edges().len() > s.red_edges().len() {
                    red_nondecreasing += 1;
                }
            }
        }
        let g = Menu::CommutativeTrim.applicable(s, Policy::Guarded);
        for x in 0..g.len() {
            for y in (x + 1)..g.len() {
                let (ra, rb) = (g[x], g[y]);
                let (Some(a), Some(b)) = (apply(s, ra), apply(s, rb)) else {
                    continue;
                };
                if key(&a) == key(&b) {
                    exact += 1;
                    continue;
                }
                let c = apply(&a, rb);
                let c2 = apply(&b, ra);
                let exact_swap = matches!((&c, &c2), (Some(x), Some(y)) if key(x) == key(y));
                if exact_swap {
                    exact += 1;
                } else {
                    // 仍可回合?用深度 2 到達集查
                    let ra2 = reachable(&a, 2);
                    let rb2 = reachable(&b, 3);
                    if ra2.intersection(&rb2).next().is_some() {
                        lazy_join += 1;
                    } else {
                        broken += 1;
                        if first.is_empty() {
                            first = format!(
                                "s={} ra={:?} rb={:?} a→rb={} b→ra={}",
                                one_line(s),
                                ra,
                                rb,
                                c.as_ref().map(one_line).unwrap_or_else(|| "⊥".to_string()),
                                c2.as_ref().map(one_line).unwrap_or_else(|| "⊥".to_string())
                            );
                        }
                    }
                }
            }
        }
    }
    println!(
        "[r3_swap] n={n} m={m} 宇宙={} (filter:{}) | 精確交換/平凡={exact} 需多步={lazy_join} 不可回合={broken} 紅邊增加={red_nondecreasing}",
        states.len(),
        if filtered { "是" } else { "否" }
    );
    if !first.is_empty() {
        println!("   first: {first}");
    }
    if broken == 0 && red_nondecreasing == 0 {
        println!("   → 精確交換引理在該宇宙成立(可直接 Rocq 化:ct_join_exact)✅");
    }
}

fn reachable(s: &AState, d: usize) -> HashSet<Vec<(u32, u32, u32, u32)>> {
    let mut seen = HashSet::new();
    seen.insert(key(s));
    let mut frontier = vec![s.clone()];
    for _ in 0..d {
        let mut next = Vec::new();
        for st in &frontier {
            for r in Menu::CommutativeTrim.applicable(st, Policy::Guarded) {
                if let Some(s2) = apply(st, r) {
                    if seen.insert(key(&s2)) {
                        next.push(s2);
                    }
                }
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    seen
}

fn one_line(s: &AState) -> String {
    let evs: Vec<String> = s
        .evs
        .iter()
        .map(|e| {
            format!(
                "{}{}[{},{}]",
                e.id,
                if e.kind == cl0r0::rep::K::Mut {
                    "mut"
                } else {
                    "sh"
                },
                e.it.start,
                e.it.end
            )
        })
        .collect();
    format!("{{{}}}#{}", evs.join(" "), s.red_edges().len())
}
