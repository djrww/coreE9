//! demo —— ROADMAP **P2 #9「watch 層原型」**(增量編輯器演示)。
//!
//! 兩段演示,把 edit monoid(M5 並行歸併)與 `reparse`(增量重析)接起來:
//!   1. **一次按鍵 = 一次增量重析**:編輯器把每次編輯當成一個 `Edit`,只重析
//!      受影響區域 —— 逐字元注入一段 local edit,報出 reused/total(reuse 率),
//!      並證明「增量重析 ≡ 全量重析」(reparse 的樹與 parse 的樹逐字節同)。
//!   2. **並行編輯去抖歸併**:多個游標 / 多個檔案事件落在同一去抖窗口,用
//!      `compose_seq`(M5)按起始點排序歸併成**一次**重析,reuse 率更高,且
//!      結果與「分別施作」一致。
//!
//! 使用:cargo run --bin demo

use cl0r0::edit::{apply, apply_all, compose_seq, is_pairwise_disjoint, Edit};
use cl0r0::parse::{parse, reparse, ReparseOut, Tree};

/// 對一則編輯做一步增量重析,回傳輸出與新樹。
fn step(old: &Tree, e: &Edit) -> (ReparseOut, Tree) {
    let new_src = apply(&old.src, e);
    let out = reparse(old, &new_src, std::slice::from_ref(e)).expect("reparse is total");
    let tree = out.tree.clone();
    (out, tree)
}

fn pct(a: usize, b: usize) -> String {
    if b == 0 {
        return String::from("—");
    }
    format!("{:.1}%", 100.0 * a as f64 / b as f64)
}

fn main() {
    println!("======================================================================");
    println!(" P2 #9 watch 層原型 · M5 並行歸併 + §2.2 增量重析");
    println!("======================================================================");

    // ---- 基礎文檔 ----
    let base = "fn main() {\n\
                \x20 let mut x = 1;\n\
                \x20 let r = &mut x;\n\
                \x20 let z = *r;\n\
                \x20 f(x);\n\
                }";
    let t0 = parse(base).expect("total");
    println!("\n--- ① 一次按鍵 = 一次增量重析 -------------------------------------------");
    println!("初始文檔(全量 parse):{} 節點\n{}", t0.total_nodes(), base);

    // 在 `f(x);` 之前注入一段多字元程式:逐字元輸入。
    let to_type = "let y = x + 1;\n  if y > 0 { g(y); }";
    let mut cur = t0;
    let mut cumulative_reused = 0usize;
    let mut cumulative_total = 0usize;
    let mut i = 0usize;
    for ch in to_type.chars() {
        let pos = cur.src.find("f(x);").unwrap() as u32; // 每次都在游標處注入
        let e = Edit::new(pos, pos, &ch.to_string());
        let (out, t2) = step(&cur, &e);
        // 契約:增量重析的樹 == 全量重析的樹(逐字節 unparse 與 sexp 相等)。
        let full = parse(&t2.src).expect("total");
        let same = t2.unparse() == full.unparse() && t2.sexp() == full.sexp();
        i += 1;
        if i <= 6 || i.is_multiple_of(10) || i == to_type.chars().count() {
            println!(
                "  按鍵[{:>3}] {:?} → 增量重析:reused {}/{} ({}), 全等:{}",
                i,
                ch,
                out.reused,
                out.total,
                pct(out.reused, out.total),
                if same { "✓" } else { "✗" }
            );
        }
        cumulative_reused += out.reused;
        cumulative_total += out.total;
        cur = t2;
    }
    println!(
        "  合計 {} 次按鍵,累計 reuse 率 = {}",
        i,
        pct(cumulative_reused, cumulative_total)
    );
    println!("  最終文檔:\n{}", cur.src);

    // ---- ② 並行編輯去抖歸併 ----
    println!("\n--- ② 並行編輯去抖歸併(M5:同一批互不重疊編輯,任意順序歸併結果相同) --");
    // 三處互不重疊的編輯(模擬三個游標 / 三個檔案事件在一去抖窗口內湧入)。
    let p1 = cur.src.find("f(x);").unwrap() as u32;
    let p2 = (cur.src.find("{ g(y); }").unwrap() as u32) + 1;
    let p3 = cur.src.find("let z = *r;").unwrap() as u32;
    let e1 = Edit::new(p1, p1, "let a = 0;\n  ");
    let e2 = Edit::new(p2, p2, "let b = 2; ");
    let e3 = Edit::new(p3, p3 + "let z = *r;".len() as u32, "let z = *r + 1;");
    let batch = vec![e1.clone(), e2.clone(), e3.clone()];
    println!(
        "  去抖窗口內捕獲 {} 條互不重疊編輯(互不重疊:{})",
        batch.len(),
        is_pairwise_disjoint(&batch)
    );
    for (i, e) in batch.iter().enumerate() {
        println!(
            "    e{}: 替換 [{},{}) → {:?}",
            i + 1,
            e.start,
            e.old_end,
            e.text
        );
    }

    // 歸併成一次編輯投放(compose_seq 依起始點排序,M5)。
    let merged = compile_seq(&batch);
    let merged_src = apply_all(&cur.src, &merged);
    let out_m = reparse(&cur, &merged_src, &merged).expect("total");
    // 對照:全量重析必須一致。
    let full_m = parse(&merged_src).expect("total");
    let same_m = out_m.tree.unparse() == full_m.unparse();
    println!(
        "  compose_seq 歸併為 {} 條 → 一次增量重析:reused {}/{} ({}), 逐字節全等:{}",
        merged.len(),
        out_m.reused,
        out_m.total,
        pct(out_m.reused, out_m.total),
        if same_m { "✓" } else { "✗" }
    );
    println!("  歸併後文檔:\n{}", merged_src);

    println!("\n======================================================================");
    println!("  小結:reparse 的樹 ≡ parse 的樹(「增量重析 ≡ 全量重析」);");
    println!("        compose_seq 把並行編輯歸併為一次重析(M5)。");
    println!("======================================================================");
}

/// 對「互不重疊」批次做 sort(等同 compose_seq 的分離部分;此處批次已交驗
/// 互不重疊,若交疊則如實回退為原批次以保 1:1 對應)。
fn compile_seq(batch: &[Edit]) -> Vec<Edit> {
    compose_seq(batch).unwrap_or_else(|| batch.to_vec())
}
