//! 九律 L1–L9 + 編輯單體 M1/M2/M4/M5 + 定理 T2 的機械自証矩陣(驗收合同)。
#![allow(non_snake_case)]
//! 九條定律的具名測試矩陣 = worker 的驗收合同(報告 §2.4)。
//!
//! 紀律:律先於碼 —— 每條定律一個具名測試;律不過,碼不合。
//! 依約定,L3/L4(增量等價)不在驗收矩陣內(見 README「範圍與排除」)。

use cl0r0::ast::{self, Interval, Track};
use cl0r0::edit::{apply, compose, Edit};
use cl0r0::gen::{gen_edit, gen_garbage, gen_half_file, gen_legal, Rng};
use cl0r0::parse::{parse, Kind};
use cl0r0::rep::{self, AState, Ev, Menu, Policy, Rule, K};
use cl0r0::span::Span;

// ===========================================================================
// L1 無損回環:unparse(parse(s)) ≡ s(逐字節)
// ===========================================================================

#[test]
fn test_law_L1_roundtrip_byte_exact() {
    let mut rng = Rng::new(11);
    for i in 0..1500 {
        let src = if i % 2 == 0 {
            gen_legal(&mut rng)
        } else {
            gen_garbage(&mut rng, 24)
        };
        let t = parse(&src).expect("total parser");
        assert_eq!(t.unparse(), src, "L1 violated for {:?}", src);
        t.validate_continuity()
            .unwrap_or_else(|e| panic!("continuity: {}\nfor {:?}", e, src));
        t.validate_tree_shapes()
            .unwrap_or_else(|e| panic!("tree axioms: {} for {:?}", e, src));
    }
}

#[test]
fn test_law_L1_lexical_tiling() {
    // 詞法平鋪是 L1 的字節級前提(§5.1)。
    let mut rng = Rng::new(12);
    for _ in 0..500 {
        let src = gen_garbage(&mut rng, 30);
        cl0r0::lex::lexical_invariants(&src)
            .unwrap_or_else(|e| panic!("tiling: {} for {:?}", e, src));
    }
}

#[test]
fn test_law_L1_exhaustive_small() {
    // 窮舉小樣(字母表含關鍵字碎片、trivia、非法字元)。
    let alphabet = [
        "x", "1", ";", "{", "}", "(", ")", "&", "=", "let", "fn", "if", " ",
    ];
    fn go(a: &[&str], d: usize, m: usize, cur: &mut String, f: &mut dyn FnMut(&str)) {
        if d >= m {
            f(cur);
            return;
        }
        for x in a {
            cur.push_str(x);
            go(a, d + 1, m, cur, f);
            cur.truncate(cur.len() - x.len());
        }
    }
    let mut count = 0;
    go(&alphabet, 0, 4, &mut String::new(), &mut |s| {
        let t = parse(s).expect("total");
        assert_eq!(t.unparse(), s, "L1 exhaustive {:?}", s);
        assert!(t.laminar_ok(), "L5 exhaustive {:?}", s);
        count += 1;
    });
    assert!(count > 2000);
}

// ===========================================================================
// L2 決定論:s = s′ ⇒ parse(s) = parse(s′)(序列化相等)
// ===========================================================================

#[test]
fn test_law_L2_determinism() {
    let mut rng = Rng::new(21);
    for i in 0..400 {
        let src = if i % 3 == 0 {
            gen_legal(&mut rng)
        } else {
            gen_garbage(&mut rng, 40)
        };
        let a = parse(&src).unwrap();
        let b = parse(&src).unwrap();
        assert_eq!(a.sexp(), b.sexp(), "L2 violated for {:?}", src);
        assert_eq!(a.named_sexp(), b.named_sexp());
    }
}

// ===========================================================================
// L5 區間嵌套:任意兩節點 span 要嘛嵌套、要嘛不交(§3.1 定理 + 反證檢驗)
// ===========================================================================

#[test]
fn test_law_L5_laminar_nesting() {
    let mut rng = Rng::new(31);
    for i in 0..600 {
        let src = if i % 2 == 0 {
            gen_legal(&mut rng)
        } else {
            gen_garbage(&mut rng, 30)
        };
        let t = parse(&src).unwrap();
        assert!(
            t.laminar_ok(),
            "L5 violated (partial overlap) for {:?}",
            src
        );
    }
}

// ===========================================================================
// L6 投影一致:named(reparse(…)) ≡ named(parse(edit(…)))(具名層繼承)
// 依約定:投影一致性的靜態面(投影是表面層的函數)在此驗證;
// 增量面(L3 相關)不在矩陣內。
// ===========================================================================

#[test]
fn test_law_L6_projection_is_function_of_surface() {
    // 具名投影是樹同態:同一輸入 ⇒ 同一投影;且投影不依賴匿名細節。
    let mut rng = Rng::new(41);
    for _ in 0..300 {
        let src = gen_legal(&mut rng);
        let t = parse(&src).unwrap();
        let named = t.named_sexp();
        let t2 = parse(&src).unwrap();
        assert_eq!(named, t2.named_sexp(), "L6 violated");
        // 具名層不含匿名 token 與 trivia
        assert!(
            !named.contains("\" "),
            "named projection must drop trivia text"
        );
        assert!(
            !named.contains("(;)") && !named.contains("(&)"),
            "anonymous dropped"
        );
        // 具名層保留錨位址(spans)
        for id in t.named_node_ids() {
            let n = t.node(id);
            assert!(n.kind.is_named());
            assert!(n.span.start <= n.span.end);
        }
    }
}

// ===========================================================================
// L7 ERROR 包攝
#[test]
fn test_law_L7a_no_false_errors() {
    let mut rng = Rng::new(51);
    for _ in 0..1200 {
        let src = gen_legal(&mut rng);
        let t = parse(&src).unwrap();
        assert!(
            !t.has_error(),
            "L7a violated: legal program got ERROR node:\n{}",
            src
        );
        assert_eq!(t.unparse(), src);
    }
}

#[test]
fn test_law_L7b_structural_maximality() {
    let mut rng = Rng::new(52);
    for _ in 0..800 {
        let legal = gen_legal(&mut rng);
        let half = gen_half_file(&mut rng, &legal);
        let t = parse(&half).unwrap();
        // (a) 極大錯誤跨度互不嵌套(§2.3 L7b)。
        let spans = t.maximal_error_spans();
        for a in 0..spans.len() {
            for b in (a + 1)..spans.len() {
                let strict_contained = (spans[a].start < spans[b].start
                    && spans[b].end < spans[a].end)
                    || (spans[b].start < spans[a].start && spans[a].end < spans[b].end);
                assert!(
                    !strict_contained,
                    "L7b: maximal error spans must be non-nested: {:?} vs {:?}",
                    spans[a], spans[b]
                );
            }
        }
        // (b) 迭代淨化:反覆移除全部「極大」錯誤跨度(長度嚴格下降 ⇒ 終止),
        //     正規形不得含**非空** ERROR 節點;空 ERROR(缺失內容)只允許
        //     位於切縫或文件末端。
        let mut t = half.clone();
        let mut seams: Vec<u32> = Vec::new();
        let mut rounds = 0;
        let mut clean = false;
        while rounds < 8 {
            let rec = parse(&t).unwrap();
            let spans = rec.maximal_error_spans();
            if spans.is_empty() {
                clean = true;
                break;
            }
            let mut cut = String::new();
            let mut last = 0u32;
            for sp in &spans {
                cut.push_str(&t[last as usize..sp.start as usize]);
                seams.push(sp.start);
                seams.push(sp.end);
                last = sp.end;
            }
            cut.push_str(&t[last as usize..]);
            if cut == t {
                break; // 只剩空跨度:無進展,交給下面的判定
            }
            t = cut;
            rounds += 1;
        }
        let fin = parse(&t).unwrap();
        let mut bad = Vec::new();
        for (i, n) in fin.nodes.iter().enumerate() {
            if n.kind != Kind::Error {
                continue;
            }
            if n.span.len() > 0 {
                bad.push((i, n.span));
            } else if !(n.span.start as usize == t.len() || seams.contains(&n.span.start)) {
                bad.push((i, n.span));
            }
        }
        // 到達不動點;殘餘只允許切縫 / EOF 處的空錯誤(缺失內容)。
        assert!(
            bad.is_empty() && rounds < 8,
            "L7b violated: iterative purification of {:?} ended at {:?} rounds={} bad={:?}",
            half,
            t,
            rounds,
            bad
        );
        let _ = clean;
    }
}

// ===========================================================================
// L8 紅邊遞減:菜單每條規則嚴格遞減 μ(§4.2)
// ===========================================================================

#[test]
fn test_law_L8_red_edge_decreasing() {
    let mut rng = Rng::new(61);
    for _ in 0..400 {
        let s = random_state(&mut rng);
        if let Some((s0, s1, r)) = rep::l8_check(&s, Menu::CommutativeTrim, Policy::Guarded) {
            panic!(
                "L8 violated: rule {} applied to {:?} increases/stalls μ {:?}→{:?}",
                r.label(),
                s0.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>(),
                s0.measure(),
                s1.measure()
            );
        }
    }
}

#[test]
fn test_law_L8_measure_is_guaranteeing_termination() {
    // 良基測度:μ 取值於 ℕ×ℕ 字典序 ⇒ 重寫序列長度 ≤ μ(s),不可能是無窮。
    let mut rng = Rng::new(62);
    for _ in 0..200 {
        let s = random_state(&mut rng);
        let (_, steps) = rep::normalize(s.clone(), Menu::CommutativeTrim, Policy::Guarded);
        let bound = s.measure().0 + 1;
        assert!(
            steps <= bound,
            "termination bound violated: {} > {}",
            steps,
            bound
        );
    }
}

// ===========================================================================
// L9 合流唯一:終止 + 局部合流 ⇒ 唯一正規形(Newman)
// 詳細的臨界對窮舉見 `cargo run --bin l9newman`;這裡做同構的流內檢查。
// ===========================================================================

#[test]
fn test_law_L9_unique_normal_form() {
    let mut rng = Rng::new(71);
    for _ in 0..250 {
        let s = random_state(&mut rng);
        // 窮舉所有極大歸約序列 → 所有正規形;必須唯一(Newman 結論)。
        let nfs = cl0r0::l9newman::normal_forms(&s, Menu::CommutativeTrim, Policy::Guarded, 12);
        assert_eq!(
            nfs.len(),
            1,
            "L9 violated: multiple normal forms from {:?}: {:?}",
            s.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>(),
            nfs
        );
        // 正規形必須紅邊清零
        let nf_key = &nfs[0];
        let nf_state = key_to_state(nf_key);
        assert!(nf_state.red_edges().is_empty(), "normal form not red-free");
    }
}

#[test]
fn test_law_L9_critical_pairs_joinable() {
    // 臨界對:兩條規則重疊施用。對(窮舉 + 隨機)3 事件狀態的雙步分歧,驗證可回合。
    // (2 事件狀態只有一條適用規則,沒有臨界對 —— 所以需要 ≥3 事件。)
    let mut checked = 0;
    let mut rng = Rng::new(72);
    let exhaustive = cl0r0::rep::enumerate_states(3, 4);
    let mut states = exhaustive;
    for _ in 0..400 {
        states.push(random_state(&mut rng));
    }
    for s in &states {
        let mut starts: Vec<u32> = s.evs.iter().map(|e| e.it.start).collect();
        starts.sort();
        if starts.windows(2).any(|w| w[0] == w[1]) {
            continue;
        }
        let rules = Menu::CommutativeTrim.applicable(s, Policy::Guarded);
        for i in 0..rules.len() {
            for j in (i + 1)..rules.len() {
                if let (Some(a), Some(b)) = (rep::apply(s, rules[i]), rep::apply(s, rules[j])) {
                    checked += 1;
                    assert!(
                        cl0r0::l9newman::joinable(
                            &a,
                            &b,
                            Menu::CommutativeTrim,
                            Policy::Guarded,
                            8
                        ),
                        "critical pair ({}, {}) from {:?} not joinable: a={:?} b={:?}",
                        rules[i].label(),
                        rules[j].label(),
                        s.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>(),
                        a.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>(),
                        b.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>()
                    );
                }
            }
        }
    }
    assert!(
        checked > 100,
        "critical pairs must be checked (got {})",
        checked
    );
}

#[test]
fn test_law_L9_naive_menu_finds_counterexample() {
    // 誠實申報:樸素菜單(任意 cut 縮短 + split + swap + runtime)
    // 存在不可回合臨界對 ⇒ Newman 前提不滿足 ⇒ 正規形不唯一。
    let states = cl0r0::rep::enumerate_states(2, 5);
    let mut found = None;
    'outer: for s in &states {
        let rules = Menu::Naive.applicable(s, Policy::Raw);
        for i in 0..rules.len() {
            for j in (i + 1)..rules.len() {
                if let (Some(a), Some(b)) = (rep::apply(s, rules[i]), rep::apply(s, rules[j])) {
                    if !cl0r0::l9newman::joinable(&a, &b, Menu::Naive, Policy::Raw, 4) {
                        found = Some((rules[i], rules[j], s.clone(), a, b));
                        break 'outer;
                    }
                }
            }
        }
    }
    assert!(
        found.is_some(),
        "expected a non-joinable critical pair in the naive menu (it is the documented reason why the menu must be canonicalized)"
    );
    if let Some((r1, r2, s, a, b)) = found {
        eprintln!(
            "L9 counterexample (naive menu): from {:?}, rules {} and {} diverge: {:?} vs {:?}",
            s.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>(),
            r1.label(),
            r2.label(),
            a.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>(),
            b.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>()
        );
    }
}

// ===========================================================================
// 編輯單體(§2.1):M1 單位元、M2 位移複合 = 平移量之和、M3 結合律、M4 應用一致
// ===========================================================================

#[test]
fn test_edit_monoid_laws() {
    let mut rng = Rng::new(81);
    for _ in 0..300 {
        let src = gen_legal(&mut rng);
        // M5:兩個互不重疊的原空間編輯,任意順序歸併結果相同(去抖歸併依據)
        let f1 = gen_edit(&mut rng, src.len());
        let f2 = gen_edit(&mut rng, src.len());
        if cl0r0::edit::is_pairwise_disjoint(&[f1.clone(), f2.clone()]) {
            let a = apply_all_(&src, &[f1.clone(), f2.clone()]);
            let b = apply_all_(&src, &[f2.clone(), f1.clone()]);
            assert_eq!(a, b, "M5 order-independence");
        }
        // M1 + M4:複合 = 先施後施的並行歸一(可複合對)
        let e1 = gen_edit(&mut rng, src.len());
        let s1 = apply(&src, &e1);
        let e2 = gen_edit(&mut rng, s1.len());
        let s2 = apply(&s1, &e2);
        if let Some(combo) = compose(&e1, &e2) {
            assert_eq!(apply_all_(&src, &combo), s2, "M4 apply-compose");
        }
        let empty = Edit::new(0, 0, "");
        if let Some(c) = compose(&empty, &e1) {
            assert_eq!(apply_all_(&src, &c), s1, "M1 left unit");
        }
        if let Some(c) = compose(&e1, &empty) {
            assert_eq!(apply_all_(&src, &c), s1, "M1 right unit");
        }
        // M2:位移複合 = 平移量之和(整數加法結合律,報告 §2.1 證明要點)。
        // 只在「兩個位移都生效」的 p 上檢查(p 位於兩個編輯區之後)。
        let p = (e1.old_end + e2.old_end + 2).min(src.len() as u32);
        if let Some(x) = e1.shift(p) {
            if let Some(y) = e2.shift(x) {
                if p > e1.old_end && x > e2.old_end {
                    assert_eq!(y as i64, p as i64 + e1.delta() + e2.delta(), "M2");
                }
            }
        }
    }
}

fn apply_all_(src: &str, edits: &[Edit]) -> String {
    cl0r0::edit::apply_all(src, edits)
}

// ===========================================================================
// 定理 T2(§3.3):區間圖 = 完美圖 —— χ(G) = ω(G)(實例機械驗證)
// ===========================================================================

#[test]
fn test_theorem_T2_interval_graphs_are_perfect() {
    let mut rng = Rng::new(91);
    for _ in 0..400 {
        let n = 2 + rng.below(40) as usize;
        let mut ivs = Vec::new();
        for _ in 0..n {
            let a = rng.below(100) as u32;
            let b = a + 1 + rng.below(30) as u32;
            ivs.push(Interval { start: a, end: b });
        }
        let omega = ast::max_clique(&ivs);
        let chi = ast::greedy_chromatic(&ivs);
        assert_eq!(
            omega, chi,
            "T2 violated: ω={} χ={} on {:?}",
            omega, chi, ivs
        );
    }
}

// ===========================================================================
// 輔助
// ===========================================================================

fn random_state(rng: &mut Rng) -> AState {
    let n = 2 + rng.below(3) as usize;
    let mut evs = Vec::new();
    let mut p = 0u32;
    for i in 0..n {
        let len = 1 + rng.below(4) as u32;
        let kind = if rng.chance(1, 2) { K::Mut } else { K::Sh };
        evs.push(Ev {
            id: i as u32,
            storage: if rng.chance(1, 4) { 1 } else { 0 },
            kind,
            it: Interval {
                start: p,
                end: p + len,
            },
        });
        p += len;
    }
    AState::new(evs)
}

fn key_to_state(key: &[(u32, u32, K, u32, u32)]) -> AState {
    AState::new(
        key.iter()
            .map(|&(id, storage, kind, st, en)| Ev {
                id,
                storage,
                kind,
                it: Interval { start: st, end: en },
            })
            .collect(),
    )
}

#[allow(dead_code)]
fn sp(x: u32, y: u32) -> Span {
    Span::new(x, y)
}

#[allow(dead_code)]
fn _kind_probe(k: Kind) {
    let _ = k;
}

#[allow(dead_code)]
fn _track_probe(t: Track) {
    let _ = t;
}

#[allow(dead_code)]
fn _rule_probe(r: Rule) {
    let _ = r;
}
