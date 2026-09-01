//! fuzz —— 屬性測試主程序(確定性種子,可重現)。
//! 運行:`cargo run --bin fuzz`(失敗時以非零退出碼結束)。
//!
//! 覆蓋(全部為「律」級檢查;L3/L4 增量等價依約定排除,見 README):
//!   L1 無損回環、L2 決定論、L5 嵌套、L6 投影一致、L7a 不假報、L7b 良構極大、
//!   L8 遞減、L9 合流/唯一正規形(抽查)+ 編輯單體定律 + 連續性/樹公理 + T2。

use cl0r0::ast;
use cl0r0::gen::{gen_edit, gen_garbage, gen_half_file, gen_legal, Rng};
use cl0r0::parse::{parse, reparse};
use cl0r0::rep::{self, AState, Ev, Menu, Policy, K};
use cl0r0::span::Span;

fn main() {
    let mut rng = Rng::new(0xC10_2024_0001);
    let mut stats = Vec::<(String, usize, usize)>::new(); // (law, passed, failed)
    let fail = |stats: &mut Vec<(String, usize, usize)>,
                law: &str,
                extra: &str,
                ok: bool,
                detail: std::fmt::Arguments| {
        if let Some(e) = stats.iter_mut().find(|(l, _, _)| *l == law) {
            e.1 += 1;
            if !ok {
                e.2 += 1;
            }
        } else {
            stats.push((law.to_string(), 1, if ok { 0 } else { 1 }));
        }
        if !ok {
            eprintln!("LAW FAIL [{}] {}: {}", law, extra, detail);
        }
    };

    // ---------- L1/L2/L5/L6 + 連續性 + 樹公理:任意輸入(含髒) ----------
    let n1 = 4000usize;
    for i in 0..n1 {
        let src = if i % 3 == 0 {
            gen_legal(&mut rng)
        } else {
            gen_garbage(&mut rng, 30)
        };
        let t = match parse(&src) {
            Ok(t) => t,
            Err(e) => {
                fail(
                    &mut stats,
                    "L1",
                    "deep",
                    false,
                    format_args!("parse err {:?}", e),
                );
                continue;
            }
        };
        // L1
        let un = t.unparse();
        if un != src {
            fail(
                &mut stats,
                "L1",
                "roundtrip",
                false,
                format_args!("{:?} != {:?}", un, src),
            );
        }
        // L2
        let t2 = parse(&src).unwrap();
        if t2.sexp() != t.sexp() {
            fail(&mut stats, "L2", "determinism", false, format_args!(""));
        }
        // L5
        if !t.laminar_ok() {
            fail(&mut stats, "L5", "laminar", false, format_args!("{}", src));
        }
        // 連續性 + 樹公理
        let c = t.validate_continuity().is_ok();
        if !c {
            fail(
                &mut stats,
                "L1",
                "continuity",
                false,
                format_args!("{}", t.validate_continuity().unwrap_err()),
            );
        }
        let sh = t.validate_tree_shapes().is_ok();
        if !sh {
            fail(
                &mut stats,
                "L1",
                "treeaxiom",
                false,
                format_args!("{}", t.validate_tree_shapes().unwrap_err()),
            );
        }
        // L7a:合法生成 ⇒ 無 ERROR
        if i % 3 == 0 && t.has_error() {
            fail(
                &mut stats,
                "L7a",
                "no-false-error",
                false,
                format_args!("legal src has errors: {}", src),
            );
        }
    }
    // 合法程式的密集樣本
    let n_legal = 800usize;
    for _ in 0..n_legal {
        let src = gen_legal(&mut rng);
        let t = parse(&src).unwrap();
        if t.has_error() {
            fail(
                &mut stats,
                "L7a",
                "no-false-error",
                false,
                format_args!("{}", src),
            );
        }
        if t.unparse() != src {
            fail(
                &mut stats,
                "L1",
                "legal-roundtrip",
                false,
                format_args!("{}", src),
            );
        }
        // L6:具名投影一致性(與重析無關的靜態面:投影由表面層決定)
        let named1 = t.named_sexp();
        let t2 = parse(&src).unwrap();
        if t2.named_sexp() != named1 {
            fail(
                &mut stats,
                "L6",
                "projection-consistency",
                false,
                format_args!(""),
            );
        }
    }

    // ---------- L7b:寫一半的檔案(注入式) ----------
    let n_half = 1500usize;
    for _ in 0..n_half {
        let legal = gen_legal(&mut rng);
        let half = gen_half_file(&mut rng, &legal);
        // (a) 極大錯誤跨度互不嵌套。
        let t = parse(&half).unwrap();
        let spans = t.maximal_error_spans();
        for a in 0..spans.len() {
            for b in (a + 1)..spans.len() {
                let strict = (spans[a].start < spans[b].start && spans[b].end < spans[a].end)
                    || (spans[b].start < spans[a].start && spans[a].end < spans[b].end);
                if strict {
                    fail(
                        &mut stats,
                        "L7b",
                        "non-nested",
                        false,
                        format_args!("{:?} vs {:?}", spans[a], spans[b]),
                    );
                }
            }
        }
        // (b) 迭代淨化:反覆移除極大錯誤跨度,直到不動點;殘餘只允許
        //     切縫 / EOF 處的空錯誤(缺失內容)。
        let mut cur = half.clone();
        let mut seams: Vec<u32> = Vec::new();
        let mut rounds = 0;
        while rounds < 8 {
            let rec = parse(&cur).unwrap();
            let spans = rec.maximal_error_spans();
            if spans.is_empty() {
                break;
            }
            let mut cut = String::new();
            let mut last = 0u32;
            for sp in &spans {
                cut.push_str(&cur[last as usize..sp.start as usize]);
                seams.push(sp.start);
                seams.push(sp.end);
                last = sp.end;
            }
            cut.push_str(&cur[last as usize..]);
            if cut == cur {
                break;
            }
            cur = cut;
            rounds += 1;
        }
        let fin = parse(&cur).unwrap();
        let mut bad = 0usize;
        for n in &fin.nodes {
            if n.kind != cl0r0::parse::Kind::Error {
                continue;
            }
            if !n.span.is_empty()
                || !(n.span.start as usize == cur.len() || seams.contains(&n.span.start))
            {
                bad += 1;
            }
        }
        if bad > 0 || rounds >= 8 {
            fail(
                &mut stats,
                "L7b",
                "iterative-purify",
                false,
                format_args!(
                    "half={:?} cur={:?} bad={} rounds={}",
                    half, cur, bad, rounds
                ),
            );
        }
    }

    // ---------- L7a 窮舉小樣(另見 tests/laws.rs)----------
    let alphabet = [
        "x", "1", ";", "{", "}", "(", ")", "&", "=", "let", "fn", "if", " ",
    ];
    exhaust(&alphabet, 0, 4, &mut String::new(), &mut |s| {
        let t = parse(s).unwrap();
        let ok1 = t.unparse() == s;
        if !ok1 {
            fail(
                &mut stats,
                "L1",
                "exhaustive",
                false,
                format_args!("{:?}", s),
            );
        }
        if !t.laminar_ok() {
            fail(
                &mut stats,
                "L5",
                "exhaustive",
                false,
                format_args!("{:?}", s),
            );
        }
    });

    // ---------- 編輯單體 ----------
    let n_edits = 500usize;
    for _ in 0..n_edits {
        let src = gen_legal(&mut rng);
        let e1 = gen_edit(&mut rng, src.len());
        let s1 = cl0r0::edit::apply(&src, &e1);
        let e2 = gen_edit(&mut rng, s1.len());
        let s2 = cl0r0::edit::apply(&s1, &e2);
        // M4:複合編輯 = 先 e1 後 e2(e2 坐標先經 e1 逆位移)
        if let Some(combo) = cl0r0::edit::compose(&e1, &e2) {
            let s3 = cl0r0::edit::apply_all(&src, &combo);
            if s2 != s3 {
                fail(
                    &mut stats,
                    "M4",
                    "apply-compose",
                    false,
                    format_args!("{:?} / {:?}", src, s2),
                );
            }
            // M5:兩個互不重疊的原空間編輯,任意順序歸併結果相同
            let f2 = gen_edit(&mut rng, src.len());
            if cl0r0::edit::is_pairwise_disjoint(&[e1.clone(), f2.clone()]) {
                let a = cl0r0::edit::apply_all(&src, &[e1.clone(), f2.clone()]);
                let b = cl0r0::edit::apply_all(&src, &[f2.clone(), e1.clone()]);
                if a != b {
                    fail(
                        &mut stats,
                        "M5",
                        "order-independence",
                        false,
                        format_args!(""),
                    );
                }
            }
        }
        // M2:位移複合 = 平移量之和(僅在兩個位移都生效時成立)
        let p = (e1.old_end + e2.old_end + 2).min(src.len() as u32);
        if let Some(x) = e1.shift(p) {
            if let Some(y) = e2.shift(x) {
                let total = e1.delta() + e2.delta();
                if p > e1.old_end && x > e2.old_end && y as i64 != p as i64 + total {
                    fail(
                        &mut stats,
                        "M2",
                        "shift-sum",
                        false,
                        format_args!("p={} y={} total={}", p, y, total),
                    );
                }
            }
        }
    }

    // ---------- 增量層工具(注:非律級斷言;L3/L4 依約定排除)----------
    let n_rep = 200usize;
    let mut reuse_ratio = 0.0f64;
    for _ in 0..n_rep {
        let src = gen_legal(&mut rng);
        let t = parse(&src).unwrap();
        let e = gen_edit(&mut rng, src.len());
        let new_src = cl0r0::edit::apply(&src, &e);
        if let Ok(out) = reparse(&t, &new_src, std::slice::from_ref(&e)) {
            // 工具性檢查(不作律級斷言):重用率統計
            reuse_ratio += out.reused as f64 / out.total.max(1) as f64;
        }
    }
    stats.push((
        "REUSE".to_string(),
        1,
        if reuse_ratio > 0.0 { 0 } else { 1 },
    ));

    // ---------- L8/L9 抽查(詳盡版在 l9newman bin)----------
    let n_l8 = 300usize;
    for _ in 0..n_l8 {
        let n = 2 + rng.below(3) as usize;
        let mut evs = Vec::new();
        let mut p = 0u32;
        for i in 0..n {
            let len = 1 + rng.below(4) as u32;
            let kind = if rng.chance(1, 2) { K::Mut } else { K::Sh };
            evs.push(Ev {
                id: i as u32,
                storage: 0,
                kind,
                it: ast::Interval {
                    start: p,
                    end: p + len,
                },
            });
            p += len;
        }
        let s = AState::new(evs);
        if let Some((_, s2, r)) = rep::l8_check(&s, Menu::CommutativeTrim, Policy::Guarded) {
            fail(
                &mut stats,
                "L8",
                "decrease",
                false,
                format_args!("{}: {:?}→{:?}", r.label(), s.measure(), s2.measure()),
            );
        }
        // 一次正規化必須清零
        let (nf, steps) = rep::normalize(s, Menu::CommutativeTrim, Policy::Guarded);
        if !nf.red_edges().is_empty() || steps > 100 {
            fail(
                &mut stats,
                "L9",
                "normalize",
                false,
                format_args!("steps={} red={}", steps, nf.red_edges().len()),
            );
        }
    }

    // ---------- 輸出 ----------
    let mut total_fail = 0usize;
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║ 機械自証報告(種子 0xC1020240001,可重現)                 ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    for (law, pass, f) in &stats {
        total_fail += f;
        println!(
            "║ {:<6} 檢查 {} 次,失敗 {} 次                                    ║",
            law, pass, f
        );
    }
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║ 總失敗數:{}", total_fail);
    println!("╚══════════════════════════════════════════════════════════╝");
    if total_fail > 0 {
        std::process::exit(1);
    }
}

fn exhaust(alphabet: &[&str], depth: usize, max: usize, cur: &mut String, f: &mut dyn FnMut(&str)) {
    if depth >= max {
        f(cur);
        return;
    }
    for a in alphabet {
        cur.push_str(a);
        exhaust(alphabet, depth + 1, max, cur, f);
        cur.truncate(cur.len() - a.len());
    }
}

#[allow(dead_code)]
fn sp(x: u32, y: u32) -> Span {
    Span::new(x, y)
}
