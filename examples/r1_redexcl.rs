//! R1 —— 紅邊互斥檢查的演算法版(R3 具體 WCR 的 Guarded 側條件引理)
//!
//! 為什麼需要這支:Guarded 版 WCR 唯一的缺口是「鑽石的兩條補步是否仍為
//! Guarded」。集合相等(Guarded≡Raw)那條捷徑已被否證(runtime ≠ [] 時
//! 4,680 / 8,000 個狀態分歧),只能直接論證。論證的核心是互斥:
//!
//!   ra 只動 ia ⇒ 只可能移除涉及 ia 的紅邊;
//!   rb 只動 ib ⇒ 只可能移除涉及 ib 的紅邊;
//!   兩者的交集只可能是邊 (ia, ib);
//!   而「ra 移除 (ia,ib)」⇒ istart ia < istart ib,
//!     「rb 移除 (ia,ib)」⇒ istart ib < istart ia —— 不能同時成立
//!   ⇒ 交集為空 ⇒ 兩條規則各移除 ≥1 ⇒ |E_red| 嚴格降 ⇒ 補步仍 Guarded。
//!
//! 本程式把上述每一條都做成可執行的檢查,在**帶 runtime 標記**的宇宙上
//! 窮舉(這是 Guarded≠Raw 的宇宙,也是唯一有鑑別力的地方)。
//!
//! 用法:
//!   cargo run --release --example r1_redexcl -- 3 4        # 3 事件,座標 0..=4
//!   cargo run --release --example r1_redexcl -- 3 4 2      # 再多跑「兩條 runtime 邊」

use cl0r0::rep::{self, AState, Menu, Policy, Rule};
use std::collections::BTreeSet;

/// 紅邊集合(以 id 對為元素的集合;uniq_ids 下無重複)。
fn eset(s: &AState) -> BTreeSet<(u32, u32)> {
    s.red_edges().into_iter().collect()
}

/// 所有可能的事件對(以 id 排序)。
fn all_pairs(s: &AState) -> Vec<(u32, u32)> {
    let ids: Vec<u32> = s.evs.iter().map(|e| e.id).collect();
    let mut out = Vec::new();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            out.push((ids[i].min(ids[j]), ids[i].max(ids[j])));
        }
    }
    out
}

/// 由 CT 規則取出它作用的事件 id(CT 菜單只發 R1Shorten)。
fn target(r: Rule) -> Option<(u32, u32)> {
    match r {
        Rule::R1Shorten(i, c) => Some((i, c)),
        _ => None,
    }
}

struct Tally {
    peaks: u64,           // 異 id 的 Guarded 峰(真正需要互斥論證的那些)
    same_id: u64,         // 同 id 的峰(uniq 下參數必相等,平凡可回合)
    removed_a_empty: u64, // ra 宣稱 Guarded 卻一條紅邊都沒移除 ⇒ 不可能
    removed_b_empty: u64,
    intersect: u64,      // 兩條規則移除集合有交集 ⇒ 互斥論證破了
    not_incident_a: u64, // ra 移除了不涉及 ia 的邊 ⇒ 「只動 ia」的前提破了
    not_incident_b: u64,
    overlap_cases: u64,    // 邊 (ia,ib) 確實出現在 ra 的移除集合中的次數(有趣案例)
    join_not_guarded: u64, // 補步後 |E_red| 沒有嚴格下降 ⇒ 補步不是 Guarded 步
    join_mismatch: u64,    // 兩條補步沒到同一個狀態
    join_none: u64,        // 補步適用失敗
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(3);
    let maxc: u32 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(4);
    let nrt: usize = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(1);

    let base = rep::enumerate_states(n, maxc);
    let pairs = all_pairs(&base[0]);
    let mut t = Tally {
        peaks: 0,
        same_id: 0,
        removed_a_empty: 0,
        removed_b_empty: 0,
        intersect: 0,
        not_incident_a: 0,
        not_incident_b: 0,
        overlap_cases: 0,
        join_not_guarded: 0,
        join_mismatch: 0,
        join_none: 0,
    };
    let mut configs: u64 = 0;
    let mut by_rt: Vec<u64> = vec![0; nrt + 1]; // 依 |runtime| 分層的峰數

    for st in &base {
        for rt in runtime_sets(&pairs, nrt) {
            let mut u = st.clone();
            u.runtime = rt.clone();
            configs += 1;
            let before = t.peaks;
            check_peaks(&u, &mut t);
            by_rt[rt.len()] += t.peaks - before;
        }
    }

    println!("=== R1 紅邊互斥檢查(帶 runtime 標記的宇宙)===");
    println!(
        "宇宙:{} 事件 × 座標 0..={maxc} = {} 狀態;每個狀態標記 ≤{nrt} 條 runtime 邊 ⇒ {configs} 個配置",
        n,
        base.len()
    );
    println!();
    println!("異 id 的 Guarded 峰(需互斥論證)  : {}", t.peaks);
    println!("同 id 的 Guarded 峰(平凡)        : {}", t.same_id);
    println!(
        "  分層(依 runtime 邊數)          : {}",
        by_rt
            .iter()
            .enumerate()
            .map(|(k, v)| format!("|rt|={k} → {v}"))
            .collect::<Vec<_>>()
            .join("  ")
    );
    println!();
    println!("── 互斥論證的每一條前提 ──");
    println!("  ra 移除集合為空(不該發生)     : {}", t.removed_a_empty);
    println!("  rb 移除集合為空(不該發生)     : {}", t.removed_b_empty);
    println!("  ra 移除不涉及 ia 的邊          : {}", t.not_incident_a);
    println!("  rb 移除不涉及 ib 的邊          : {}", t.not_incident_b);
    println!("  兩者移除集合交集非空           : {}", t.intersect);
    println!("  其中 (ia,ib) 被 ra 移除的次數  : {}", t.overlap_cases);
    println!();
    println!("── 結論:補步是否仍為 Guarded ──");
    println!("  兩條補步未到同一狀態           : {}", t.join_mismatch);
    println!("  補步適用失敗                   : {}", t.join_none);
    println!("  補步後 |E_red| 未嚴格下降      : {}", t.join_not_guarded);
    println!();
    let ok = t.removed_a_empty == 0
        && t.removed_b_empty == 0
        && t.intersect == 0
        && t.not_incident_a == 0
        && t.not_incident_b == 0
        && t.join_not_guarded == 0
        && t.join_mismatch == 0
        && t.join_none == 0;
    println!(
        ">>> {}",
        if ok {
            "全部通過:互斥成立,且兩條補步皆為 Guarded 步"
        } else {
            "有違反 —— 互斥論證不成立,R3 路線需修正"
        }
    );
}

/// 枚舉所有大小 ≤ k 的 runtime 邊集合。
fn runtime_sets(pairs: &[(u32, u32)], k: usize) -> Vec<Vec<(u32, u32)>> {
    let mut out = vec![vec![]];
    if k == 0 {
        return out;
    }
    for p in pairs {
        let cur: Vec<Vec<(u32, u32)>> = out
            .iter()
            .filter(|s| s.len() < k)
            .map(|s| {
                let mut v = s.clone();
                v.push(*p);
                v
            })
            .collect();
        out.extend(cur);
    }
    out
}

/// 檢查單一狀態 u 上所有 Guarded 峰。
fn check_peaks(u: &AState, t: &mut Tally) {
    let g = Menu::CommutativeTrim.applicable(u, Policy::Guarded);
    for x in 0..g.len() {
        for y in (x + 1)..g.len() {
            let (Some((ia, _ca)), Some((ib, _cb))) = (target(g[x]), target(g[y])) else {
                continue;
            };
            if ia == ib {
                t.same_id += 1;
                continue;
            }
            t.peaks += 1;

            let (Some(sa), Some(sb)) = (rep::apply(u, g[x]), rep::apply(u, g[y])) else {
                continue;
            };

            let eu = eset(u);
            let esa = eset(&sa);
            let esb = eset(&sb);

            // Guarded ⇒ 各移除 ≥1 條紅邊(這是側條件的定義,必為真,但值得量出來)
            let ra_rm: BTreeSet<(u32, u32)> = eu.difference(&esa).copied().collect();
            let rb_rm: BTreeSet<(u32, u32)> = eu.difference(&esb).copied().collect();
            if ra_rm.is_empty() {
                t.removed_a_empty += 1;
            }
            if rb_rm.is_empty() {
                t.removed_b_empty += 1;
            }

            // 只動 ia / 只動 ib
            for e in &ra_rm {
                if e.0 != ia && e.1 != ia {
                    t.not_incident_a += 1;
                }
            }
            for e in &rb_rm {
                if e.0 != ib && e.1 != ib {
                    t.not_incident_b += 1;
                }
            }

            // 交集最多只能是 (ia, ib)
            let inter: Vec<&(u32, u32)> = ra_rm.intersection(&rb_rm).collect();
            let edge_ab = (ia.min(ib), ia.max(ib));
            for e in &inter {
                if **e != edge_ab {
                    t.intersect += 1;
                }
            }
            if ra_rm.contains(&edge_ab) {
                t.overlap_cases += 1;
            }

            // 補步:sa --rb--> c1,sb --ra--> c2
            let (Some(c1), Some(c2)) = (rep::apply(&sa, g[y]), rep::apply(&sb, g[x])) else {
                t.join_none += 1;
                continue;
            };
            if eset(&c1) != eset(&c2)
                || c1.evs.iter().map(|e| e.it).collect::<Vec<_>>()
                    != c2.evs.iter().map(|e| e.it).collect::<Vec<_>>()
            {
                t.join_mismatch += 1;
            }
            // 關鍵:補步是否仍為 Guarded 步(|E_red| 嚴格下降)
            let ec = c1.red_edges().len();
            if !(ec < sa.red_edges().len()) || !(ec < sb.red_edges().len()) {
                t.join_not_guarded += 1;
            }
        }
    }
}
