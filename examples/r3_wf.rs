//! R3 交換引理的「良構性敏感」探針(第二輪,2026-09-02)。
//!
//! 要回答的問題:Rocq 側想證「把 a 剪到 `cut_for a` ⇒ a 從他人的候選集消失」。
//! 手工分析指出該命題需要區間良構性 `istart ≤ iend` —— 若容許倒掛區間
//! (例如 a = [5,2)),剪完之後 `istart b <? iend a` 可能仍為真,被剪者就
//! **還在**別人的候選集裡,交換引理的這條捷徑斷掉。
//!
//! 本探針 therefore 分兩節:
//!   1. `enumerate_states` 的 CI 宇宙 —— 但它的生成式是 `for end in (start+1)..=max`
//!      (Rocq 側 `ends_for m s := iend := s + S d` 同形),**由構造排除**倒掛。
//!      實測 n=3 m=5:27,000 個狀態、含倒掛 **0** 個 ⇒ 這一節對「要不要 wf 前提」
//!      是**套套邏輯**,不能拿來下結論(我第一版就差點被它騙)。
//!   2. 自行枚舉「允許倒掛」的小宇宙(end 取 0..=m,不要求 > start),
//!      這才是決定性的樣本:若其中有非精確交換 ⇒ R3 必須带 `wf_state` 前提。
//!
//! 用法:`cargo run --release --example r3_wf [n] [m]`

use cl0r0::ast::Interval;
use cl0r0::rep::{apply, enumerate_states, AState, Ev, Menu, Policy, K};

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

fn inverted(s: &AState) -> usize {
    s.evs.iter().filter(|e| e.it.start > e.it.end).count()
}

/// 自行枚舉「允許倒掛」的小宇宙:每個事件 (kind, start, end) 三者獨立。
fn enumerate_inverted(n: usize, m: u32) -> Vec<AState> {
    let mut per: Vec<Ev> = Vec::new();
    for kind in [K::Mut, K::Sh] {
        for start in 0..=m {
            for end in 0..=m {
                per.push(Ev {
                    id: u32::MAX,
                    storage: 0,
                    kind,
                    it: Interval { start, end },
                });
            }
        }
    }
    let mut out = vec![Vec::new()];
    for _ in 0..n {
        let mut next = Vec::with_capacity(out.len() * per.len());
        for pref in &out {
            for e in &per {
                let mut v = pref.clone();
                v.push(*e);
                next.push(v);
            }
        }
        out = next;
    }
    out.into_iter()
        .map(|evs| {
            let mut evs = evs;
            for (i, e) in evs.iter_mut().enumerate() {
                e.id = i as u32;
            }
            AState::new(evs)
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let only_inv = args.iter().any(|a| a == "--inverted-only");
    let n: usize = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(3);
    let m: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(6);

    let states: Vec<AState> = enumerate_states(n, m);
    let with_inv: usize = states.iter().filter(|s| inverted(s) > 0).count();

    // 分桶:peers 計數、違反計數,依「s/a/b 三者最大倒掛數」
    let mut peers = [0u64; 4];
    let mut viol = [0u64; 4];
    let mut broken_total = 0u64;
    let mut first: Vec<String> = Vec::new();
    let mut exact = 0u64;
    let mut lazy = 0u64;

    for s in &states {
        let g = Menu::CommutativeTrim.applicable(s, Policy::Guarded);
        for x in 0..g.len() {
            for y in (x + 1)..g.len() {
                let (ra, rb) = (g[x], g[y]);
                let (Some(a), Some(b)) = (apply(s, ra), apply(s, rb)) else {
                    continue;
                };
                let bucket = inverted(s).max(inverted(&a)).max(inverted(&b)).min(3);
                peers[bucket] += 1;
                if key(&a) == key(&b) {
                    exact += 1;
                    continue;
                }
                let c = apply(&a, rb);
                let c2 = apply(&b, ra);
                if matches!((&c, &c2), (Some(x), Some(y)) if key(x) == key(y)) {
                    exact += 1;
                } else if joinable(&a, &b) {
                    lazy += 1;
                    viol[bucket] += 1;
                } else {
                    broken_total += 1;
                    viol[bucket] += 1;
                    if first.len() < 4 {
                        first.push(format!(
                            "inv={} s={} a={} b={} a→rb={} b→ra={}",
                            bucket,
                            one_line(s),
                            one_line(&a),
                            one_line(&b),
                            c.as_ref().map(one_line).unwrap_or_else(|| "⊥".into()),
                            c2.as_ref().map(one_line).unwrap_or_else(|| "⊥".into()),
                        ));
                    }
                }
            }
        }
    }

    println!(
        "[r3_wf] n={n} m={m} 宇宙={} 含倒掛狀態={with_inv}",
        states.len()
    );
    println!("      精確交換/平凡={exact} 需多步={lazy} 不可回合={broken_total}");
    for b in 0..4 {
        if peers[b] > 0 {
            println!(
                "      倒掛{}: peers={:<9} 非精確交換={}",
                if b == 3 {
                    "≥3".to_string()
                } else {
                    b.to_string()
                },
                peers[b],
                viol[b]
            );
        }
    }
    for f in &first {
        println!("   {f}");
    }
    if with_inv == 0 && !only_inv {
        println!(
            "   → ⚠ 本宇宙**不含任何倒掛區間**(生成式已排除),故無法判斷 R3 是否需要
      wf 前提。請接跑 `--inverted-only` 段(下面第二節)再下結論。"
        );
    } else if viol[0] == 0 && viol.iter().skip(1).all(|v| *v == 0) {
        println!("   → 全部 peers 精確交換(含倒掛樣本)⇒ 無條件 R3 可證");
    } else if viol[0] == 0 {
        println!("   → 僅良構樣本全綠 ⇒ R3 必須帶 wf_state 前提(範圍縮小要寫進 ROCQ-TRACE)");
    } else {
        println!("   → 良構樣本也有違反 ⇒ 模型有漏洞,先修模型再談定理");
    }

    // ---- 第二節:允許倒掛的宇宙(這才是決定性樣本) ----
    let inv_states = enumerate_inverted(n.max(3), m.max(3));
    let mut ip = 0u64;
    let mut iv = 0u64;
    let mut iinv = 0u64;
    for s in &inv_states {
        if inverted(s) > 0 {
            iinv += 1;
        }
        let g = Menu::CommutativeTrim.applicable(s, Policy::Guarded);
        for x in 0..g.len() {
            for y in (x + 1)..g.len() {
                let (Some(a), Some(b)) = (apply(s, g[x]), apply(s, g[y])) else {
                    continue;
                };
                ip += 1;
                if key(&a) == key(&b) {
                    continue;
                }
                let c = apply(&a, g[y]);
                let c2 = apply(&b, g[x]);
                if !matches!((&c, &c2), (Some(x), Some(y)) if key(x) == key(y)) {
                    iv += 1;
                    if iv <= 3 {
                        println!(
                            "   [倒掛反例候選] s={} a={} b={} a→rb={} b→ra={}",
                            one_line(s),
                            one_line(&a),
                            one_line(&b),
                            c.as_ref().map(one_line).unwrap_or_else(|| "⊥".into()),
                            c2.as_ref().map(one_line).unwrap_or_else(|| "⊥".into())
                        );
                    }
                }
            }
        }
    }
    println!(
        "[r3_wf/inverted] n={} m={} 宇宙={} 含倒掛={} peers={} 非精確交換={}",
        n.max(3),
        m.max(3),
        inv_states.len(),
        iinv,
        ip,
        iv
    );
    println!(
        "   → {}",
        if iv == 0 {
            "即使允許倒掛也全數精確交換 ⇒ wf 前提可能不必要(仍需 Rocq 論證)"
        } else {
            "倒掛樣本出現非精確交換 ⇒ Rocq 側必須显式帶 wf_state 不變量"
        }
    );
}

fn joinable(a: &AState, b: &AState) -> bool {
    use std::collections::HashSet;
    let ra = reachable(a, 3, HashSet::new());
    let rb = reachable(b, 3, HashSet::new());
    !ra.is_disjoint(&rb)
}

fn reachable(
    s: &AState,
    d: usize,
    mut seen: std::collections::HashSet<Vec<(u32, u32, u32, u32)>>,
) -> std::collections::HashSet<Vec<(u32, u32, u32, u32)>> {
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
    format!("{{{}}}", evs.join(" "))
}
