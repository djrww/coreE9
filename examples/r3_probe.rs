//! R3 可行性探針(Phase 3 開工前的「先找反例,再寫證明」紀律)。
//!
//! 檢驗三件事(全部在 CommutativeTrim 菜單上,狀態空間比 CI 的 4×6 更大):
//!   P1 start 不變性:每條 CT 規則的施用是否只改 end、不改任何事件的 start;
//!   P2 交換引理(diamond,精確版本):s→a、s→b(b≠a)⇒ ∃c, a→*c、b→*c(深度 D 內);
//!   P3 Guarded 與 Raw 在 CT 上是否重合(若重合,鏡像可省去 filter)。
//!
//! 用法:cargo run --release --example r3_probe [n_events] [max_coord] [depth]
//! 輸出:計數 + 首個反例(若有)。反例 = Phase 3 的真正難點所在。

use cl0r0::rep::{apply, enumerate_states, AState, Menu, Policy, Rule};
use std::collections::HashSet;

/// 狀態的規範鍵(與 l9newman 同法;log 不參與)。
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

/// 深度 d 內的到達集(不含自身之外剪枝:全域 visited)。
fn reachable(
    s: &AState,
    menu: Menu,
    policy: Policy,
    d: usize,
) -> HashSet<Vec<(u32, u32, u32, u32)>> {
    let mut seen = HashSet::new();
    seen.insert(key(s));
    let mut frontier = vec![s.clone()];
    for _ in 0..d {
        let mut next = Vec::new();
        for st in &frontier {
            for r in menu.applicable(st, policy) {
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

fn starts(s: &AState) -> Vec<(u32, u32)> {
    s.evs.iter().map(|e| (e.id, e.it.start)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(4);
    let m: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(7);
    let depth: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(4);
    let filtered = !args.iter().any(|a| a == "--nofilter");

    let all = enumerate_states(n, m);
    let states: Vec<AState> = all
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
    println!(
        "[r3_probe] n={n} m={m} depth={depth} 狀態宇宙 = {} (distinct-start 過濾:{})",
        states.len(),
        if filtered { "是" } else { "否" }
    );

    let mut steps = 0usize;
    let mut pairs = 0usize;
    let mut p1_violations = 0usize;
    let mut p2_violations = 0usize;
    let mut p3_diverge = 0usize;
    let mut nf_multi = 0usize;
    let mut first_p1 = String::new();
    let mut first_p2 = String::new();

    for s in &states {
        // ---- P1:start 不變性 + P3:Guarded ⊆ Raw / 重合 ----
        let raw = Menu::CommutativeTrim.applicable(s, Policy::Raw);
        let guarded = Menu::CommutativeTrim.applicable(s, Policy::Guarded);
        let raw_set: HashSet<String> = raw.iter().map(|r| format!("{r:?}")).collect();
        let g_set: HashSet<String> = guarded.iter().map(|r| format!("{r:?}")).collect();
        if raw_set != g_set {
            p3_diverge += 1;
        }
        for r in &raw {
            if let Some(s2) = apply(s, *r) {
                steps += 1;
                if starts(s) != starts(&s2) {
                    p1_violations += 1;
                    if first_p1.is_empty() {
                        first_p1 = format!(
                            "state={:?} rule={:?} → {:?}",
                            dbg_state(s),
                            r.label(),
                            dbg_state(&s2)
                        );
                    }
                }
                // R1 專屬檢查:只有被修事件之 end 變
                if let Rule::R1Shorten(_id, cut) = r {
                    let diff: Vec<_> = s
                        .evs
                        .iter()
                        .zip(s2.evs.iter())
                        .filter(|(a, b)| a.it != b.it || a.kind != b.kind || a.storage != b.storage)
                        .map(|(a, b)| (a.id, a.it.start, a.it.end, b.it.start, b.it.end, *cut))
                        .collect();
                    if diff.len() != 1 || diff[0].1 != diff[0].3 || diff[0].4 != diff[0].5 {
                        p1_violations += 1;
                        if first_p1.is_empty() {
                            first_p1 = format!("diff={diff:?}");
                        }
                    }
                }
            }
        }

        // ---- P2:diamond(精確版本;兩步皆取 Guarded 合法步)----
        let g = &guarded;
        for x in 0..g.len() {
            for y in (x + 1)..g.len() {
                let (ra, rb) = (g[x], g[y]);
                let (Some(a), Some(b)) = (apply(s, ra), apply(s, rb)) else {
                    continue;
                };
                if key(&a) == key(&b) {
                    continue; // 平行同步:trivially joinable
                }
                pairs += 1;
                // 快路徑(便宜):零步與一步交换。
                if let Some(b2) = apply(&a, rb) {
                    if key(&b2) == key(&b) {
                        continue; // a -rb-> b:精確交换,免搜索
                    }
                }
                if let Some(a2) = apply(&b, ra) {
                    if key(&a2) == key(&a) {
                        continue; // b -ra-> a
                    }
                }
                let ca = reachable(&a, Menu::CommutativeTrim, Policy::Guarded, depth);
                let cb = reachable(&b, Menu::CommutativeTrim, Policy::Guarded, depth);
                if ca.intersection(&cb).next().is_none() {
                    p2_violations += 1;
                    if first_p2.is_empty() {
                        first_p2 = format!(
                            "s={}\n      r1={:?} → a={}\n      r2={:?} → b={}",
                            one_line(s),
                            ra.label(),
                            one_line(&a),
                            rb.label(),
                            one_line(&b)
                        );
                    }
                }
            }
        }

        // ---- 多正規形(唯一正規形的窮舉面;DFS 至正規形)----
        let mut stack = vec![s.clone()];
        let mut seen: HashSet<_> = HashSet::new();
        let mut nfs: HashSet<_> = HashSet::new();
        while let Some(st) = stack.pop() {
            if !seen.insert(key(&st)) {
                continue;
            }
            let rules = Menu::CommutativeTrim.applicable(&st, Policy::Guarded);
            let mut pushed = false;
            for r in rules {
                if let Some(s2) = apply(&st, r) {
                    stack.push(s2);
                    pushed = true;
                }
            }
            if !pushed {
                nfs.insert(key(&st));
            }
        }
        if nfs.len() > 1 {
            nf_multi += 1;
        }
    }

    println!("[r3_probe] 單步數 = {steps},臨界對(不同後繼)= {pairs}");
    println!(
        "[r3_probe] P1 start-不變 違反 = {p1_violations} {}",
        if p1_violations == 0 { "✅" } else { "❌" }
    );
    if !first_p1.is_empty() {
        println!("   first: {first_p1}");
    }
    println!(
        "[r3_probe] P2 diamond 違反(深度 {depth} 內不可回合)= {p2_violations} {}",
        if p2_violations == 0 { "✅" } else { "❌" }
    );
    if !first_p2.is_empty() {
        println!("   first:\n   {first_p2}");
    }
    println!(
        "[r3_probe] P3 Guarded≠Raw 之狀態數 = {p3_diverge} {}",
        if p3_diverge == 0 {
            "(完全重合)"
        } else {
            "(不可省 filter!)"
        }
    );
    println!("[r3_probe] 多正規形狀態數 = {nf_multi}");
}

fn dbg_state(s: &AState) -> String {
    one_line(s)
}

fn one_line(s: &AState) -> String {
    let evs: Vec<String> = s
        .evs
        .iter()
        .map(|e| {
            format!(
                "{}#{}{}[{},{})",
                e.id,
                if e.kind == cl0r0::rep::K::Mut {
                    "mut"
                } else {
                    "sh"
                },
                if e.storage == 0 { "" } else { "?s" },
                e.it.start,
                e.it.end
            )
        })
        .collect();
    format!("evs[{}] red{:?}", evs.join(" "), s.red_edges())
}
