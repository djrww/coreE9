//! cl0r0 —— 演示程序:CL0 載體上的九律檢查 + 幾何/重寫演示 + R₀ 接線預備。
//!
//! 運行:`cargo run --bin cl0r0`(全綠輸出 0 個失敗即為機械自証通過)。

use cl0r0::ast::{self, Track};
use cl0r0::parse::parse;
use cl0r0::rep::{self, AState, Ev, Menu, Policy, K};

fn main() {
    println!("======================================================================");
    println!(" CL0 / R₀ 雙載體 · 機械自証演示(九條定律 + 幾何 + 重寫系統)");
    println!("======================================================================");

    // ---------- 樣本:一個含借用衝突的 CL0 程式 ----------
    let src = "fn main() {\n\
               \x20 let mut x = 1;\n\
               \x20 let r = &mut x;\n\
               \x20 let y = x + 1;\n\
               \x20 let z = *r;\n\
               \x20 while z < 10 { f(y); }\n\
               \x20 if z == 3 { f(x); } else { g(); }\n\
               }";
    println!("\n[演示輸入]\n{}", src);

    let t = parse(src).expect("全化解析器:任何輸入都產出樹");
    let (named, anon, errs, trivia) = t.stats();
    println!(
        "\n[L1/L2/L5 樹性質] 節點 {} (具名 {} 匿名 {} trivia {} error {})",
        t.total_nodes(),
        named,
        anon,
        trivia,
        errs
    );
    let rr = t.unparse() == src;
    println!("  L1 無損回環(unparse∘parse ≡ 源碼逐字節):{}", ok(rr));
    println!(
        "  L2 決定論(兩次解析序列化相同):{}",
        ok(parse(src).unwrap().sexp() == t.sexp())
    );
    println!("  L5 區間嵌套(laminar):{}", ok(t.laminar_ok()));
    println!(
        "  連續性公理(父跨度 = 子跨度並):{}",
        ok(t.validate_continuity().is_ok())
    );
    println!(
        "  樹公理(連通/單父):{}",
        ok(t.validate_tree_shapes().is_ok())
    );
    println!(
        "  χ = |V| − |E| = 1(CW 複形,§3.4):{}",
        ok(cl0r0::parse::euler_characteristic(&t) == 1)
    );

    // ---------- 具名投影 ----------
    println!("\n[具名投影 §1.3](匿名 token 與 trivia 投影掉;結構同態)");
    println!("  {}", t.named_sexp());

    // ---------- 三軌 liveness + 衝突圖 ----------
    let facts = ast::extract(&t);
    println!(
        "\n[事實層 §3.2] 綁定 {} 事件 {} 借用鏈 {}",
        facts.bindings.len(),
        facts.events.len(),
        facts.links.len()
    );
    for (i, b) in facts.bindings.iter().enumerate() {
        println!(
            "  binding[{}] {} @{} (mut={} param={})",
            i, b.name, b.span, b.mutable, b.is_param
        );
    }
    for e in &facts.events {
        println!(
            "  event: {} {} @{}",
            e.kind.label(),
            facts.bindings[e.binding].name,
            e.span
        );
    }
    for l in &facts.links {
        println!(
            "  borrow-link: {} = {} {} @{}",
            facts.bindings[l.ref_binding].name,
            l.kind.label(),
            facts.bindings[l.src_binding].name,
            l.span
        );
    }

    for track in [Track::Lexical, Track::Nll, Track::Referent] {
        let edges = ast::red_edges(&facts, track);
        println!("\n[{} 軌] 紅邊 {} 條:", track.label(), edges.len());
        for e in &edges {
            println!(
                "  ({}, {}) on {} @{} [{},{})",
                facts.events[e.a].kind.label(),
                facts.events[e.b].kind.label(),
                facts.bindings[e.binding].name,
                e.span,
                facts.events[e.a].span.start,
                facts.events[e.b].span.start
            );
        }
        // T2:區間圖 = 弦圖 = 完美圖(χ = ω,§3.3)
        let (iv, _) = ast::intervals(&facts, track);
        let all: Vec<ast::Interval> = iv.iter().flatten().copied().collect();
        let omega = ast::max_clique(&all);
        let chi = ast::greedy_chromatic(&all);
        println!(
            "  [定理 T2 實例] 區間圖 ω = {} χ = {} → χ = ω(完美圖,弦圖):{}",
            omega,
            chi,
            ok(omega == chi)
        );
    }

    // ---------- 幾何演示:把紅邊清零的「修剪」計劃 ----------
    println!("\n[修法菜單 §4] 以 referent 軌的紅邊為對象,運行規範修剪菜單");
    // 把事實層投影為 AState:每個事件一區間(取 referent 軌;無 referent 用 nll)
    let (ivs, events) = ast::intervals(&facts, Track::Referent);
    let mut evs: Vec<Ev> = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        let kind = match ev.kind {
            ast::EvKind::BorrowMut => K::Mut,
            ast::EvKind::Move | ast::EvKind::Deref => K::Mut,
            _ => K::Sh,
        };
        evs.push(Ev {
            id: i as u32,
            storage: 0,
            kind,
            it: ivs[ev.binding].get(i).copied().unwrap_or(ast::Interval {
                start: ev.span.start,
                end: ev.span.end,
            }),
        });
    }
    let s0 = AState::new(evs);
    let (nf, steps) = rep::normalize(s0.clone(), Menu::CommutativeTrim, Policy::Guarded);
    println!(
        "  初始:紅邊 {} 條;正規化 {} 步;最終紅邊 {} 條",
        s0.red_edges().len(),
        steps,
        nf.red_edges().len()
    );
    let mut st = s0.clone();
    let mut plan = Vec::new();
    while let Some((s2, r)) = rep::step(&st, Menu::CommutativeTrim, Policy::Guarded) {
        plan.push((r, st.red_edges().len(), s2.red_edges().len()));
        st = s2;
    }
    for (r, before, after) in plan {
        println!("    {} : 紅邊 {} → {}", r.label(), before, after);
    }
    println!(
        "  結果(紅邊清零 = 幾何收斂,§3.5):{}",
        ok(nf.red_edges().is_empty())
    );

    // ---------- ERROR 全化(L7)演示 ----------
    println!("\n[L7 ERROR 全化 §2.3]「寫到一半的檔案」");
    for half in [
        "fn main() {\n  let mut x = 1;\n  let r = &mut",
        "fn main() {\n  let y = x +",
        "fn main() { if x { } else {",
        "@@@",
    ] {
        let h = parse(half).expect("total");
        let spans = h.maximal_error_spans();
        let mut cut = String::new();
        let mut last = 0u32;
        for sp in &spans {
            cut.push_str(&half[last as usize..sp.start as usize]);
            last = sp.end;
        }
        cut.push_str(&half[last as usize..]);
        let rec = parse(&cut).expect("total");
        println!(
            "  {:?}\n    → ERROR 節點 {} 個(跨度 {:?});挖除後重析:{} 錯誤 → L7b 良構極大:{}",
            half,
            spans.len(),
            spans,
            rec.n_errors(),
            ok(rec.n_errors() == 0)
        );
    }

    // ---------- R₀ 接線預備 ----------
    println!("\n[R₀ 附錄 B 接線] LALR(1)-乾淨片段 + unsupported 如實申報");
    println!("  EBNF:\n{}", cl0r0::r0::R0_EBNF);
    let r0src = "fn main() {\n  let mut x = 1;\n  let r = &mut x;\n  *r = *r + 1;\n  loop { if x < 42 { return; } }\n}";
    println!("  片段:\n{}", r0src);
    println!(
        "  r0 詞法平鋪:{};  LALR(1)-乾淨:{}",
        ok(cl0r0::r0::r0_lexical_invariants(r0src).is_ok()),
        ok(cl0r0::r0::lalr1_clean(r0src).is_ok())
    );
    let outof =
        "fn main() { let c = |a| a; match c { 0 => {} } let v: Vec<int> = vs; println!(x); }";
    println!("  越界樣本:{:?}", outof);
    let u = cl0r0::r0::unsupported(outof);
    for (k, sp) in &u {
        println!("    unsupported: {} @{}", k, sp);
    }
    println!(
        "  lalr1_clean 越界樣本(應為 Err):{}",
        ok(cl0r0::r0::lalr1_clean(outof).is_err())
    );

    println!("\n======================================================================");
    println!(" 總計:九律矩陣的機械檢查結果見 `tests/laws.rs` 與 `cargo run --bin l9newman`");
    println!("======================================================================");
}

fn ok(b: bool) -> &'static str {
    if b {
        "✓"
    } else {
        "✗ FAIL"
    }
}
