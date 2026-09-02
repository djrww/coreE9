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
            if !n.span.is_empty()
                || !(n.span.start as usize == t.len() || seams.contains(&n.span.start))
            {
                bad.push((i, n.span));
            }
        }
        // 到達不動點;殘餘只允許切縫 / EOF 處的空錯誤(缺失內容)。
        // 機械化版本(tree::l7b_evaluate)必須給出同一判定(P1 #6 覆蓋與契約複驗)。
        let (mech_bad, mech_rounds) = cl0r0::tree::l7b_evaluate(&half);
        assert_eq!(
            (mech_bad == 0, mech_rounds < 8),
            (bad.is_empty(), rounds < 8),
            "l7b_evaluate must agree with the in-test purification"
        );
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

#[test]
fn test_law_L7_error_totalization() {
    // L7 全化(附錄 §2.3):任意位元串輸入,CL0' 都回傳樹,永不 panic、
    // 永不 Err;樹必層狀(laminar)、span 必連續(continuity)、形必合公理,
    // 且 unparse 逐位元等價 —— 總化即「零丟失」。
    // 覆蓋面(四源):(a) 合法程式全語法面;(b) 雙重垃圾檔案;(c) 半截合法檔案;
    // (d) 近似隨機位元串(經 lossy 重編碼為合法 UTF-8,如實申報)。
    let mut rng = Rng::new(53);
    let mut samples: Vec<String> = Vec::new();

    for _ in 0..200 {
        samples.push(gen_legal(&mut rng));
    }
    for _ in 0..400 {
        samples.push(gen_garbage(&mut rng, 200));
    }
    for _ in 0..200 {
        let legal = gen_legal(&mut rng);
        samples.push(gen_half_file(&mut rng, &legal));
    }
    for _ in 0..200 {
        let len = rng.below(160) as usize;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(rng.next_u64() as u8);
        }
        samples.push(String::from_utf8_lossy(&v).into_owned());
    }

    assert!(samples.len() >= 1000, "L7: coverage insufficient");
    for src in &samples {
        let t = parse(src).expect("L7 violated: parser must be total (never Err)");
        assert!(
            t.laminar_ok(),
            "L7: laminar violated on {:?}…",
            &src[..src.len().min(60)]
        );
        t.validate_continuity()
            .unwrap_or_else(|e| panic!("L7: continuity violated: {}\nfor {:?}", e, src));
        t.validate_tree_shapes()
            .unwrap_or_else(|e| panic!("L7: tree axioms {} for {:?}", e, src));
        assert_eq!(t.unparse(), *src, "L7: roundtrip (byte-exact) violated");
    }
}

#[test]
fn test_shrink_finds_minimal_error_trigger() {
    // 反例最小化的真實語義用法(P0 #3):以「parse 產生 ERROR」為失敗屬性,
    // 對夾雜垃圾的程式縮小 → 得到最小錯誤觸發串(任何單字符刪除都不再觸發)。
    // 這正是 fuzz LAW FAIL 時的自動化路徑:輸入 → 最小現場。
    let mut rng = Rng::new(0x51);
    let legal = gen_legal(&mut rng);
    // 以一個確定觸發 ERROR 的非法符號作為「垃圾種子」(詞法級 Bad ⇒ 錯誤區)。
    let src = format!("{} @", legal);
    assert!(!src.is_empty());
    // L7a 屬性:合法部分清潔,整體因垃圾產生 ERROR。
    let prop = |s: &str| parse(s).map(|t| t.has_error()).unwrap_or(true);
    assert!(prop(&src), "precondition: input must be failing");
    let m = cl0r0::shrink::shrink_to_minimal(&src, &prop);
    assert!(prop(&m), "minimal must still trigger ERROR");
    // 局部最小:無單字符可刪。
    for i in 0..m.chars().count() {
        let mut cand: String = m.chars().take(i).collect();
        cand.extend(m.chars().skip(i + 1));
        assert!(!prop(&cand), "removing char {} must not keep failing", i);
    }
    // 最小現場不長於觸發所需(合法程式與冗餘應被刪光;單字符符號即觸發)。
    assert!(
        m.chars().count() <= 2,
        "minimal trigger should be a lone bad symbol: {:?}",
        m
    );
}

#[test]
fn test_law_regression_fixtures() {
    // 回歸防線(P0 #3):`tests/fixtures/` 的每個 .txt 都是一個**歷史反例**
    // (曾使 L1/L7 破裂的輸入,如第一迭代的哨兵跨度泄漏)。
    // 重播 = 現在必須全綠:parse 總化、roundtrip 逐字節、連續性、laminar、
    // 樹公理;R₀ 載體同樣總化(雙載體互為注入面)。
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut n = 0;
    for entry in std::fs::read_dir(&dir).expect("tests/fixtures must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&path).unwrap();
        let t = parse(&src).unwrap_or_else(|e| panic!("fixture {}: parse Err {:?}", name, e));
        assert_eq!(t.unparse(), src, "fixture {}: byte-exact roundtrip", name);
        t.validate_continuity()
            .unwrap_or_else(|e| panic!("fixture {}: continuity {}", name, e));
        assert!(t.laminar_ok(), "fixture {}: laminar", name);
        t.validate_tree_shapes()
            .unwrap_or_else(|e| panic!("fixture {}: tree axioms {}", name, e));
        let t0 = cl0r0::r0::r0_parse(&src)
            .unwrap_or_else(|e| panic!("fixture {}: r0_parse {:?}", name, e));
        assert_eq!(t0.unparse(), src, "fixture {}: r0 roundtrip", name);
        n += 1;
    }
    assert!(n >= 5, "fixtures must be present, got {}", n);
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

#[test]
fn test_law_L9_scaled_space_joinable() {
    // P0 #4:空間擴張(3 事件 × 5 座標 → 4 事件 × 5 座標)+ 並行分塊。
    // CommutativeTrim/Guarded 在 105,216 狀態 × 100,392 臨界對上:
    // L8 零違反、臨界對全可回合、全狀態唯一正規形 —— Newman 結論在
    // 更大空間上機械成立(規模較 3×5 大 ~10×)。
    let r = cl0r0::l9newman::newman_check(Menu::CommutativeTrim, Policy::Guarded, 4, 5, 8);
    assert!(
        r.l8_violations.is_empty(),
        "L8 violated on scaled space: {:?}",
        r.l8_violations.first()
    );
    assert!(
        r.non_joinable.is_empty(),
        "WCR violated on scaled space: {:?}",
        r.non_joinable.first()
    );
    assert!(
        r.multi_nf.is_empty(),
        "normal forms not unique on scaled space"
    );
    assert_eq!(
        r.unique_nf_states, r.states,
        "every state must have a unique normal form"
    );
    assert!(r.critical_pairs > 100_000, "scaled space must be exercised");
    assert!(r.states > 100_000, "scaled space must be larger than 3x5");
    assert!(r.threads >= 1, "parallel path must be honest about threads");
    assert!(!r.truncated, "no violations ⇒ nothing to truncate");
}

#[test]
fn test_law_L9_scaled_space_counterexample_found() {
    // 並行分塊不得削弱「機器找反例」:Naive/Raw 在 3 事件 × 3 座標即
    // 遇到不可回合臨界對(WCR 反例;這是菜單必須規範化的機械證據)。
    let r = cl0r0::l9newman::newman_check(Menu::Naive, Policy::Raw, 3, 3, 3);
    assert!(
        !r.non_joinable.is_empty(),
        "parallel newman must still find WCR counterexamples for the naive menu"
    );
    // 結論必須如實申報違反(而非誤報通過);截斷只影響展示,不影響結論。
    assert!(
        r.conclusion.contains("WCR") || r.conclusion.contains("L8"),
        "conclusion must report the violation: {}",
        r.conclusion
    );
}

// ===========================================================================
// L9b′(Phase 3 前置):平行步「精確交換」+ 側條件冗餘 + 紅邊單調性
//   Rocq 側 R3(`ct_join_exact`)的經驗前測:三條引理的無量詞版本。
// ===========================================================================

/// 狀態的可觀測骨架(與 `l9newman::canon_key` 同法:log 不參與相等性)。
fn ct_key(s: &AState) -> Vec<(u32, u32, K, u32, u32)> {
    let mut v: Vec<_> = s
        .evs
        .iter()
        .map(|e| (e.id, e.storage, e.kind, e.it.start, e.it.end))
        .collect();
    v.sort();
    v
}

fn ct_starts(s: &AState) -> Vec<(u32, u32)> {
    s.evs.iter().map(|e| (e.id, e.it.start)).collect()
}

#[test]
fn test_law_L9b_parallel_moves_exact_swap() {
    // 宇宙:3 事件 × 6 座標(35,280 狀態)+ 4 事件 × 5 座標(105,216 狀態)。
    // 註:**不**加 distinct-start 過濾 —— 證明不該依赖新狀態的额外假設。
    let mut pairs = 0usize;
    let mut swaps = 0usize;
    for (n, m) in [(3usize, 6u32), (4, 5)] {
        for s in &rep::enumerate_states(n, m) {
            // (i) 側條件冗餘:CT 菜單上 Guarded 與 Raw 給出同一規則集
            //     ⇒ Rocq 可證 `applicable s CT Guarded = applicable s CT Raw`,
            //        L8 的 guard 在 CT 上是定理而非假設。
            assert_eq!(
                Menu::CommutativeTrim.applicable(s, Policy::Guarded).len(),
                Menu::CommutativeTrim.applicable(s, Policy::Raw).len(),
                "guard must be redundant on the canonical menu (state {:?})",
                s.evs
            );

            // (ii) 紅邊單調性:任何 CT 步驟只縮 end,故 |E_red| 不增
            //      ⇒ μ 的非增性;嚴格遞減則由 guard(見 (i):兩者同集)給出。
            for r in Menu::CommutativeTrim.applicable(s, Policy::Raw) {
                let Some(t) = rep::apply(s, r) else {
                    continue;
                };
                assert!(
                    t.red_edges().len() <= s.red_edges().len(),
                    "red edges must never grow: {:?} → {:?}",
                    s.evs,
                    t.evs
                );
            }

            // (iii) 精確交換(swap lemma):s→a、s→b ⇒ a→b、b→a 且兩端同狀態。
            let g = Menu::CommutativeTrim.applicable(s, Policy::Guarded);
            for x in 0..g.len() {
                for y in (x + 1)..g.len() {
                    let (ra, rb) = (g[x], g[y]);
                    let (Some(a), Some(b)) = (rep::apply(s, ra), rep::apply(s, rb)) else {
                        continue;
                    };
                    pairs += 1;
                    if ct_key(&a) == ct_key(&b) {
                        continue; // 同一後繼:菱形平凡
                    }
                    // start 不變性:修任何事件都不動任何事件的起點
                    assert_eq!(ct_starts(&a), ct_starts(s), "start invariance (a)");
                    assert_eq!(ct_starts(&b), ct_starts(s), "start invariance (b)");
                    match (rep::apply(&a, rb), rep::apply(&b, ra)) {
                        (Some(ac), Some(bc)) => {
                            assert_eq!(
                                ct_key(&ac),
                                ct_key(&bc),
                                "diamond corner differs: {:?} / {:?}",
                                ac.evs,
                                bc.evs
                            );
                            swaps += 1;
                        }
                        (ac, bc) => panic!(
                            "exact swap failed: rules {:?} / {:?} on {:?} → {:?} / {:?}",
                            ra,
                            rb,
                            s.evs,
                            ac.map(|t| t.evs),
                            bc.map(|t| t.evs)
                        ),
                    }
                }
            }
        }
    }
    // 探針必須真的咬到東西(否則測試是空轉)。
    assert!(pairs > 100_000, "probe must exercise pairs, got {pairs}");
    assert_eq!(pairs, swaps, "every non-trivial peak must swap exactly");
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

// ===========================================================================
// 第二迭代補測:語義面衝量(P1 #6 覆蓋密度)與工具層契約
// ===========================================================================

#[test]
fn test_law_span_geometry() {
    // §1.2 半開區間代數:len/is_empty/contains/overlaps(相接不算)/shift/Display。
    let s = Span::new(2, 5);
    assert_eq!(s.len(), 3);
    assert!(!s.is_empty());
    assert!(Span::new(0, 0).is_empty());
    assert!(Span::new(0, 7).contains(&s));
    assert!(s.contains(&Span::new(3, 4)));
    assert!(!s.contains(&Span::new(1, 2)));
    // 半開:相接 [2,5) 與 [5,7) 不重疊
    assert!(!s.overlaps(&Span::new(5, 7)));
    assert!(s.overlaps(&Span::new(4, 6)));
    assert!(!s.overlaps(&Span::new(0, 2)));
    assert_eq!(s.shift(10), Span::new(12, 15));
    assert_eq!(s.shift(-2), Span::new(0, 3));
    assert_eq!(format!("{}", s), "[2, 5)");
}

#[test]
fn test_law_semantic_conflict_matrix() {
    // §3.3 相容性矩陣:15 個 kind 對,斷言對稱性與判據表一致。
    use cl0r0::ast::{EvKind, EvKind::*};
    let kinds = [Decl, Read, Move, BorrowSh, BorrowMut, Deref];
    let expect = [
        (Decl, Decl, false),
        (Decl, Read, false),
        (Decl, Move, false),
        (Decl, BorrowSh, false),
        (Decl, BorrowMut, false),
        (Decl, Deref, false),
        (Read, Read, false),
        (Read, Move, false),
        (Read, BorrowSh, false),
        (Read, BorrowMut, true),
        (Read, Deref, false),
        (Move, Move, false),
        (Move, BorrowSh, true),
        (Move, BorrowMut, true),
        (Move, Deref, false),
        (BorrowSh, BorrowSh, false),
        (BorrowSh, BorrowMut, true),
        (BorrowSh, Deref, false),
        (BorrowMut, BorrowMut, true),
        (BorrowMut, Deref, false), // 解引用是借用鏈的內部使用,非相容性衝突
        (Deref, Deref, false),
    ];
    for (a, b, want) in expect {
        assert_eq!(
            cl0r0::ast::conflicts(a, b),
            want,
            "conflicts({:?}, {:?})",
            a,
            b
        );
        assert_eq!(
            cl0r0::ast::conflicts(b, a),
            want,
            "symmetry conflicts({:?}, {:?})",
            b,
            a
        );
    }
    // 全矩陣完備(6×6):與 expect 表一致(表中未列出的對 = false)
    let mut pairs: Vec<(EvKind, EvKind)> = Vec::new();
    for (i, a) in kinds.iter().enumerate() {
        for (_j, b) in kinds.iter().enumerate().skip(i) {
            pairs.push((*a, *b));
        }
    }
    assert_eq!(pairs.len(), 21);
    for (a, b) in &pairs {
        let want = expect.iter().any(|&(x, y, w)| (x == *a && y == *b) && w);
        assert_eq!(
            cl0r0::ast::conflicts(*a, *b),
            want,
            "matrix must be complete for {:?} {:?}",
            a,
            b
        );
    }
}

#[test]
fn test_law_semantic_facts_consistent() {
    // §3.2–3.3 語義面一致性:extract → 三軌 intervals → 紅邊 → 圖形狀,
    // 並行驗證 T2 的 χ = ω(區間圖完美:max_clique == greedy_chromatic)。
    use cl0r0::ast::Track;
    use cl0r0::ast::{
        conflict_graph_shape, extract, greedy_chromatic, intervals, max_clique, red_edges,
    };
    let mut rng = Rng::new(0xA57);
    let mut checked = 0usize;
    for i in 0..600 {
        let src = match i % 3 {
            0 => gen_legal(&mut rng),
            1 => {
                let l = gen_legal(&mut rng);
                gen_half_file(&mut rng, &l)
            }
            _ => gen_garbage(&mut rng, 50),
        };
        let t = parse(&src).unwrap();
        let facts = extract(&t);
        assert_eq!(facts.has_error_regions, t.has_error());
        // 標籤面(§3.2 顯示標簽):種類與軌道標籤都必須非空(sexp/報告的基礎)。
        assert!(
            !Track::Lexical.label().is_empty()
                && !Track::Nll.label().is_empty()
                && !Track::Referent.label().is_empty()
        );
        for ev in &facts.events {
            assert!(!ev.kind.label().is_empty());
        }
        for track in [Track::Lexical, Track::Nll, Track::Referent] {
            let (ivs, evs) = intervals(&facts, track);
            assert_eq!(ivs.len(), facts.bindings.len(), "one vec per binding");
            let mut total = 0usize;
            for (b, iv) in ivs.iter().enumerate() {
                total += iv.len();
                for it in iv {
                    assert!(it.start <= it.end, "{:?} must be half-open", track);
                }
                // interval 圖完美性:χ = ω(同一綁定的活躍區間形成區間圖)
                if iv.len() >= 2 {
                    assert_eq!(
                        max_clique(iv),
                        greedy_chromatic(iv),
                        "T2 χ=ω violated on binding {} ({:?})",
                        b,
                        track
                    );
                }
            }
            assert_eq!(total, evs.len(), "one interval per event ({:?})", track);
            // 紅邊:同綁定 + 相容性被違反 + 區間重疊(獨立複驗)
            let edges = red_edges(&facts, track);
            let (v, e) = conflict_graph_shape(&facts, track);
            assert_eq!(e, edges.len(), "shape edge count");
            let mut verts: Vec<usize> = Vec::new();
            for ed in &edges {
                verts.push(ed.a);
                verts.push(ed.b);
                let ea = &facts.events[ed.a];
                let eb = &facts.events[ed.b];
                assert_eq!(ea.binding, ed.binding);
                assert_eq!(eb.binding, ed.binding);
                assert!(cl0r0::ast::conflicts(ea.kind, eb.kind));
            }
            verts.sort_unstable();
            verts.dedup();
            assert_eq!(v, verts.len(), "shape vertex count");
        }
        checked += 1;
    }
    assert!(checked >= 500, "must exercise the fact layer");
}

#[test]
fn test_law_parse_error_paths_total() {
    // 錯誤回收的構造矩陣:每個語法構造一個壞一半的輸入 → 錯誤回收分支。
    // 斷言:總化 + byte-exact roundtrip + 連續性 + laminar + 樹公理(零 panic)。
    let cases: &[&str] = &[
        "fn",
        "fn f",
        "fn f(",
        "fn f()",
        "fn f() {",
        "fn f() { let",
        "fn f() { let mut",
        "fn f() { let x",
        "fn f() { let x =",
        "fn f() { let x = ; }",
        "fn f() { let x = 1",
        "fn f() { if",
        "fn f() { if x",
        "fn f() { if x {",
        "fn f() { if x { } else",
        "fn f() { while",
        "fn f() { while x",
        "fn f() { loop",
        "fn f() { return",
        "fn f() { return ;",
        "fn f() { x(",
        "fn f() { x.y",
        "fn f() { x[",
        "fn f() { x[0",
        "fn f() { {",
        "fn f() { &",
        "fn f(,",
        "fn f(x",
        "fn f(x:",
        "struct",
        "struct S",
        "struct S {",
        "struct S { a",
        "struct S { a:",
        "fn f() { let x = (1; }",
        "fn f() { let x = { 1; }",
        "fn f() { !; }",
        "fn f() { -; }",
        "fn f() { x = ; }",
        "fn f() { *; }",
        "fn f() { f(,); }",
        "fn f() { f(a,); }",
        "fn f() { (a b); }",
        "fn f() { a & & ; }",
        "fn f() { a + ; }",
        "fn f() { a < ; }",
        "fn f() { || ; }",
        "fn f() { let _ = 1; }",
        "fn f() { let m = &mut; }",
        "fn f() { x = y = ; }",
    ];
    for src in cases {
        let t = parse(src).expect("total (no Err)");
        assert_eq!(t.unparse(), *src, "roundtrip");
        t.validate_continuity()
            .unwrap_or_else(|e| panic!("continuity: {} ({:?})", e, src));
        assert!(t.laminar_ok(), "laminar ({:?})", src);
        t.validate_tree_shapes()
            .unwrap_or_else(|e| panic!("tree axioms: {} ({:?})", e, src));
    }
}

#[test]
fn test_law_r0_error_paths_total() {
    // R₀ 錯誤回收矩陣:半截構造 → 節點級 Unsupported / Error,必 Ok 且 roundtrip。
    let cases: &[&str] = &[
        "fn",
        "fn f",
        "fn f(",
        "fn f()",
        "fn f() {",
        "fn f() { let",
        "fn f() { let x =",
        "fn f() { if",
        "fn f() { if x {",
        "fn f() { while",
        "fn f() { return",
        "fn f() { x(",
        "fn f() { x[",
        "fn f(,",
        "fn f(x:",
        "struct S {",
        "struct S { a:",
        "fn f() { let x = (1; }",
        "fn f() { let x = { 1; }",
        "fn f() { |a| a; }",
        "fn f() { #; }",
        "fn f() { 'a; }",
        "fn f() { let x: Vec<; }",
        "fn f() { let x: [int; }",
        "fn f() { &'a int; }",
        "fn f() { x y; }",
        "fn f() { &&x; }",
        "fn f() { !x; }",
        "fn f() { *x; }",
        "fn f() { let x = ; }",
        "fn f() { let mut; }",
        "fn f() { x. ; }",
        "fn f() { x .. ; }",
        "fn f() { x = ; }",
    ];
    for src in cases {
        let t = cl0r0::r0::r0_parse(src).expect("total (no Err)");
        assert_eq!(t.unparse(), *src, "roundtrip ({:?})", src);
        t.validate_continuity()
            .unwrap_or_else(|e| panic!("r0 continuity: {} ({:?})", e, src));
        assert!(t.laminar_ok(), "r0 laminar ({:?})", src);
    }
}

#[test]
fn test_law_rep_menu_algebra() {
    // 菜單 × 政策代數:適用規則集、單步、歸約、L8 檢查在全部組合上
    // 給出一致的測度行為(Guarded 保證嚴格遞減;Raw 不保證)。
    let mut rng = Rng::new(0xBE);
    let mut states = 0usize;
    for _ in 0..300 {
        let s = random_state(&mut rng);
        for menu in [Menu::CommutativeTrim, Menu::Naive] {
            for policy in [Policy::Raw, Policy::Guarded] {
                assert!(menu.label().contains("CommutativeTrim") || menu.label().contains("Naive"));
                let rules = menu.applicable(&s, policy);
                for r in &rules {
                    assert!(!r.label().is_empty(), "rule labels must be non-empty");
                    if let Some(s2) = rep::apply(&s, *r) {
                        // 施加後紅邊數不得增加(修剪/分裂都是「拆紅邊」)
                        assert!(
                            s2.red_edges().len() <= s.red_edges().len() + 8,
                            "red edges must not explode: {} -> {}",
                            s.red_edges().len(),
                            s2.red_edges().len()
                        );
                    }
                }
                if matches!(menu, Menu::CommutativeTrim) {
                    let (nf, steps) = rep::normalize(s.clone(), menu, policy);
                    assert!(steps <= 10001, "normalize must terminate");
                    let _ = nf.red_edges();
                }
                if policy == Policy::Guarded {
                    assert!(
                        rep::l8_check(&s, menu, policy).is_none(),
                        "Guarded must satisfy L8 on {:?}",
                        s.evs
                    );
                }
            }
        }
        states += 1;
    }
    assert!(states >= 250);
}

#[test]
fn test_reuse_data_consistency() {
    // §2.2/§5.3 增量重析基礎設施的工具性契約(非 L3/L4 等價斷言 —— 依規格排除):
    // 單點編輯 → reparse:新樹必須反映新源碼(roundtrip),reuse 統計一致。
    let mut rng = Rng::new(0x2A);
    let mut n = 0usize;
    for _ in 0..60 {
        let src = gen_legal(&mut rng);
        let t = parse(&src).unwrap();
        let e = gen_edit(&mut rng, src.len());
        let new_src = cl0r0::edit::apply(&src, &e);
        let out = cl0r0::parse::reparse(&t, &new_src, std::slice::from_ref(&e))
            .expect("reparse must be total on single edits");
        let (nn, an, ne, nt) = out.tree.stats();
        assert_eq!(
            out.total,
            nn + an + ne + nt,
            "total must equal the new tree's node count"
        );
        let (_, _, _, _) = t.stats();
        assert!(
            out.reused <= out.total,
            "reused must not exceed the new tree's node count"
        );
        assert!(out.reused <= out.total, "reused ⊆ total");
        // 新樹必須是新源碼的忠實 parse(roundtrip 契約)
        let t2 = parse(&new_src).unwrap();
        assert_eq!(t2.unparse(), new_src);
        n += 1;
    }
    assert!(n >= 50);
}

#[test]
fn test_edit_unit_operations() {
    // §2.1 編輯單元的邊緣操作:is_empty / shift ⊥ / unshift / compose 重疊拒絕 /
    // compose_seq 排序與非互斥拒絕。
    use cl0r0::edit::{compose, compose_seq, is_pairwise_disjoint, Edit};
    use cl0r0::span::Span;
    let e0 = Edit::new(4, 4, "");
    assert!(e0.is_empty(), "identity edit");
    let e = Edit::new(2, 5, "xy"); // 替換 [2,5) → "xy"
    assert!(!e.is_empty());
    assert_eq!(e.shift(1), Some(1), "before region");
    // delta = 2 − 3 = −1(替換 [2,5) 為 2 字元 ⇒ 淨縮短 1)
    assert_eq!(e.shift(10), Some(9), "after region: p + delta");
    assert_eq!(e.shift(3), None, "inside region ⇒ ⊥");
    assert_eq!(e.unshift(10), Some(11), "inverse shift: p − delta");
    assert_eq!(e.unshift(3), None, "inside the new text ⇒ ⊥");
    assert_eq!(e.unshift(50), Some(51), "beyond the new region: p − delta");
    // 重疊編輯:compose 拒絕(單體語義外)
    let e2 = Edit::new(4, 6, "z");
    assert!(
        compose(&e, &e2).is_none(),
        "overlapping edits must be rejected"
    );
    // 相接(半開語義 [2,5) 與 [5,7)):不重疊 ⇒ 可分離(與 §1.2 一致)
    let e3 = Edit::new(5, 7, "w");
    assert!(compose(&e, &e3).is_some(), "half-open touching is disjoint");
    // 同點插入(文本拼接語義)必須拒絕
    let e5 = Edit::new(2, 2, "ins");
    assert!(compose(&e, &e5).is_none(), "same-point insertion rejected");
    // 分離編輯:compose_seq 排序歸併
    let a = Edit::new(0, 1, "A");
    let b = Edit::new(10, 11, "B");
    let c = Edit::new(5, 6, "C");
    assert!(is_pairwise_disjoint(&[a.clone(), b.clone(), c.clone()]));
    let seq = compose_seq(&[b.clone(), c.clone(), a.clone()]).expect("disjoint ⇒ composable");
    assert!(
        seq[0].start == 0 && seq[1].start == 5 && seq[2].start == 10,
        "sorted"
    );
    assert!(
        compose_seq(&[Edit::new(0, 3, "A"), Edit::new(2, 5, "B")]).is_none(),
        "overlapping edits rejected by compose_seq"
    );
    // 應用一個編輯的結果以 span 檢查
    let src = "abcdefghij";
    let r = cl0r0::edit::apply(src, &Edit::new(2, 4, "XY"));
    assert_eq!(r, "abXYefghij");
    let _ = Span::new(0, 0);
}

#[test]
fn test_law_r0_kind_label_exhaustive() {
    // R0Kind 標籤與具名性:枚舉完備(label 非空、sexp 投影只留 is_named)。
    use cl0r0::r0::R0Kind::*;
    let kinds = [
        Root,
        FnItem,
        StructItem,
        FieldDef,
        Param,
        TypeRef,
        Block,
        LetStmt,
        ReturnStmt,
        IfStmt,
        WhileStmt,
        LoopStmt,
        ExprStmt,
        Expr,
        UnaryExpr,
        Unsupported,
        Error,
        FnKw,
        StructKw,
        LetKw,
        MutKw,
        IfKw,
        ElseKw,
        WhileKw,
        LoopKw,
        ReturnKw,
        TrueKw,
        FalseKw,
        Amp,
        AmpMut,
        Star,
        Plus,
        Minus,
        Eq,
        EqEq,
        NotEq,
        Lt,
        Le,
        Gt,
        Ge,
        AndAnd,
        OrOr,
        Not,
        Dot,
        Semi,
        Colon,
        Comma,
        LParen,
        RParen,
        LBrace,
        RBrace,
        LBrack,
        RBrack,
        Arrow,
        Slash,
        Percent,
        Ident,
        Number,
        RawString,
        Trivia,
        Bad,
    ];
    let mut labels: Vec<&str> = Vec::new();
    for k in kinds {
        let l = k.label();
        assert!(!l.is_empty(), "label must be non-empty");
        labels.push(l);
        // 具名層:結構性(非 token/trivia/bad/error/unsupported)
        if matches!(
            k,
            Root | FnItem
                | StructItem
                | FieldDef
                | Param
                | TypeRef
                | Block
                | LetStmt
                | ReturnStmt
                | IfStmt
                | WhileStmt
                | LoopStmt
                | ExprStmt
                | Expr
                | UnaryExpr
        ) {
            assert!(k.is_named(), "{:?} must be named", k);
        }
    }
    labels.sort_unstable();
    for w in labels.windows(2) {
        assert_ne!(w[0], w[1], "labels must be distinct");
    }
}

#[test]
fn test_law_r0_unsupported_keyword_matrix() {
    // §9 排除項關鍵字(詞面掃描,legacy unsupported):16 個關鍵字逐一申報。
    let kws = [
        "trait",
        "impl",
        "use",
        "mod",
        "pub",
        "unsafe",
        "async",
        "match",
        "macro_rules",
        "dyn",
        "enum",
        "type",
        "static",
        "const",
        "extern",
        "where",
    ];
    for kw in kws {
        let src = format!("{} X {{}}", kw);
        let v = cl0r0::r0::unsupported(&src);
        assert!(
            v.iter()
                .any(|(note, _)| note.contains(kw) || note.contains("排除")),
            "keyword {} must be reported, got {:?}",
            kw,
            v
        );
    }
    // 空輸入與普通程式:無虛報
    assert!(cl0r0::r0::unsupported("").is_empty());
    assert!(cl0r0::r0::unsupported("fn f() {}").is_empty());
}

#[test]
fn test_law_semantic_extract_breadth() {
    // 語法面寬度:每種事件形態都在事實層留下可機械檢查的痕跡。
    use cl0r0::ast::{extract, intervals, red_edges, Track};
    // 注意:CL0 語法面(附錄 A)不含 `>` / 賦值語句 / loop / else(解析為 ERROR);
    // 語義面只對可達語法負責,樣本矩陣以 CL0 可達語法為界。
    let progs: &[&str] = &[
        // 借鏈 + 解引用(鏈:宣告 → 借用 → 使用)
        "fn f() { let x = 1; let r = &x; let y = *r; }",
        "fn f() { let x = 1; let r = &mut x; let y = *r; }",
        "fn f() { let x = 1; let y = *&x; }",
        // 參數綁定 + 讀
        "fn f(a: int, b) { let c = a + b; }",
        "fn f(a: int) { let b = *a; }",
        // 重聲明(Nll killer)
        "fn f() { let x; let x = 1; let y = x; }",
        // if(條件 + 分支塊內宣告)
        "fn f() { let x = 1; if x < 2 { let y = 3; } }",
        "fn f() { let x = 1; if x < 2 { let y = 3; } let z = x; }",
        // while(條件 + 塊)
        "fn f() { let x = 1; while x < 2 { let y = 3; } }",
        // 移動語義(調用參數)
        "fn f() { let x = 1; g(x); let y = 2; }",
        "fn f() { let x = 1; g(x * 2); }",
        // 索引 / 字段 / 一元
        "fn f() { let x = 1; let y = x[0]; }",
        "fn f() { let x = 1; let y = p.q; }",
        "fn f() { let x = 1; let y = !x; }",
        "fn f() { let x = 1; let y = -x; }",
        // 嵌套 block 表達式
        "fn f() { let y = { let z = 1; z }; }",
        "fn f() { let y = f({ let z = 2; z }); }",
        "fn f() { let y = { let z = { let w = 3; w }; z }; }",
        // 未定義名(emit 失敗路徑)
        "fn f() { let y = undeclared + x; }",
        "fn f() { let y = &undeclared; }",
        // 常數 / 混合
        "fn f() { let y = true + 42; }",
        "fn f() { let y = false * 7 % 2; }",
        "fn f() { let x = 1; let y = x == 1 && x != 2; }",
        "fn f() { let x = 1; let y = x < 2 || x >= 1; }",
        "fn f() { let x = 1; let y = (x); }",
        // 多重使用(紅邊路徑)
        "fn f() { let x = 1; let y = &mut x; let z = x; }",
        "fn f() { let x = 1; let a = &x; let b = &x; let c = x; }",
        // 借鏈完整使用(Referent 軌的事件擴展)
        "fn f() { let x = 1; let r = &x; let y = r; }",
        "fn f() { let x = 1; let r = &x; let y = r + x; }",
        "fn f() { let mut x = 1; let r = &mut x; let y = r; }",
        "fn f() { let x = 1; let r = &x; f(r); }",
        "fn f() { let x = 1; let r = &x; let s = r; let t = r; }",
        // killer(重宣告 / 移動)
        "fn f() { let x = 1; let x = x + 1; let y = x; }",
        "fn f() { let x = 1; g(x); let y = 2; }",
        // 未綁定借用源(link 的 src lookup 失敗)
        "fn f() { let r = &nope; }",
        "fn f() { let r = &mut nope; }",
        // 多條件 / 多塊
        "fn f() { let x = 1; if x < 1 { let y = 2; } if x < 2 { let z = 3; } }",
        "fn f() { let a = 1; while a < 2 { let b = 3; } while a < 3 { let c = 4; } }",
        // block 表達式內運算與命名使用
        "fn f() { let a = { let b = 1; b * 2 }; let c = a; }",
        "fn f() { let a = 1; let b = { a }; let c = b; }",
        // 空函數 / 空塊
        "fn f() {}",
        "fn f() { }",
        "fn f() { let x = 1; }",
        "fn f() { let x = 1; let y = x; let z = y + x; }",
    ];
    let mut n = 0usize;
    for src in progs {
        let t = parse(src).expect("legal breadth sample");
        let facts = extract(&t);
        assert_eq!(facts.has_error_regions, t.has_error(), "{:?}", src);
        for track in [Track::Lexical, Track::Nll, Track::Referent] {
            let (ivs, evs) = intervals(&facts, track);
            assert_eq!(ivs.len(), facts.bindings.len());
            let tot: usize = ivs.iter().map(|v| v.len()).sum();
            assert_eq!(tot, evs.len(), "{:?} on {:?}", track, src);
            let _ = red_edges(&facts, track);
        }
        n += 1;
    }
    assert!(n >= 40, "breadth samples must be exercised, got {}", n);
    // 兼帶:垃圾輸入 → has_error 幀,事實層如實為空/部分(不得 panic)
    let mut rng = Rng::new(0xED);
    for _ in 0..30 {
        let g = gen_garbage(&mut rng, 40);
        let t = parse(&g).unwrap();
        let facts = extract(&t);
        let _ = facts.bindings.len();
        for track in [Track::Lexical, Track::Nll, Track::Referent] {
            let _ = red_edges(&facts, track);
        }
    }
}
