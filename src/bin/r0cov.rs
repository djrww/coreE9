//! r0cov —— R₀ 覆蓋面實測(P4-6;docs/R0-COVERAGE-EMPIRICAL.md 的數據源)。
//!
//! 對給定的一批 `.rs` 檔案,用 `r0_parse` 實測 R₀ 的子集覆蓋面:
//!   * 是否整檔落入 R₀ + 模型範圍(in_scope:無 error 節點、無 unsupported 節點);
//!   * unsupported 節點數 × N 與其 note 直方圖(「哪些構造被排除」的實測分布);
//!   * 字節覆蓋率:全檔字節中,不被任何 unsupported / error 節點覆蓋的比例。
//!
//! 用法:`cargo run --bin r0cov -- <file.rs> [file.rs …]`
//! 輸出:每檔一行指標 + 尾部彙總(供寫入 R0-COVERAGE-EMPIRICAL.md)。
//!
//! 誠實紀律:這裡**只測量、如實申報**,不宣稱 R₀「覆蓋」未覆蓋之物;
//! `unsupported` note 就是 R₀ 對「我為何不處理這裡」的結構化自白。

use cl0r0::r0::{r0_parse, R0Kind, R0Tree};
use cl0r0::span::Span;

/// 全檔中「有待處理(unsupported / error)」的節點:(kind, note, span) 列表。
fn problem_nodes(t: &R0Tree) -> Vec<(R0Kind, Option<&'static str>, Span)> {
    let mut out = Vec::new();
    for id in 0..t.total_nodes() as u32 {
        let n = t.node(id);
        if matches!(n.kind, R0Kind::Unsupported | R0Kind::Error) {
            out.push((n.kind, n.note, n.span));
        }
    }
    out
}

/// 把若干半開區間合併,回傳被覆蓋的總字節數。
fn merged_bytes(spans: &mut [Span]) -> usize {
    spans.sort_by_key(|s| (s.start, s.end));
    let mut total = 0u64;
    let mut cur: Option<(u32, u32)> = None;
    for s in spans.iter() {
        match cur {
            Some((cs, ce)) if s.start <= ce => {
                cur = Some((cs, ce.max(s.end)));
            }
            Some((cs, ce)) => {
                total += (ce - cs) as u64;
                cur = Some((s.start, s.end));
            }
            None => cur = Some((s.start, s.end)),
        }
    }
    if let Some((cs, ce)) = cur {
        total += (ce - cs) as u64;
    }
    total as usize
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("用法:r0cov <file.rs> [file.rs …]");
        std::process::exit(2);
    }
    println!("檔名                          行數   unsup#  err#  字節覆蓋率    範圍內");
    println!("--------------------------------------------------------------------------");
    let mut tot_lines = 0usize;
    let mut tot_unsup = 0usize;
    let mut tot_err = 0usize;
    let mut tot_bytes = 0usize;
    let mut tot_covered = 0usize;
    let mut tot_in_scope = 0usize;
    for f in &files {
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("讀取失敗 {f}: {e}");
                continue;
            }
        };
        let lines = src.lines().count();
        let bytes = src.len();
        tot_lines += lines;
        tot_bytes += bytes;
        let t = r0_parse(&src).unwrap_or_else(|_| {
            // r0_parse 全化;Depth 邊界應不出現,仍防護之。
            eprintln!("[warn] {f}: r0_parse 返回 Err(引擎深度界)");
            std::process::exit(1);
        });
        let p = problem_nodes(&t);
        let unsup = t.unsupported_spans();
        let err = t.n_errors();
        let mut spans: Vec<Span> = p
            .iter()
            .map(|(_, _, sp)| *sp)
            .filter(|s| !s.is_empty())
            .collect();
        let covered = merged_bytes(&mut spans);
        let cov_frac = if bytes == 0 {
            1.0
        } else {
            1.0 - (covered as f64 / bytes as f64)
        };
        let in_scope = err == 0 && unsup.is_empty();
        tot_unsup += unsup.len();
        tot_err += err;
        tot_covered += covered;
        if in_scope {
            tot_in_scope += 1;
        }
        println!(
            "{:<30} {:>6}   {:>5}  {:>4}   {:>8.2}%      {}",
            file_stem(f),
            lines,
            unsup.len(),
            err,
            cov_frac * 100.0,
            if in_scope { "在範圍" } else { "範圍外" }
        );
    }
    println!("--------------------------------------------------------------------------");
    let overall = if tot_bytes == 0 {
        0.0
    } else {
        1.0 - (tot_covered as f64 / tot_bytes as f64)
    };
    println!(
        "彙總:{} 檔 / {} 行 / {} 字節;unsupported 節點總數 {};error 節點總數 {};",
        files.len(),
        tot_lines,
        tot_bytes,
        tot_unsup,
        tot_err
    );
    println!(
        "     整檔落入 R₀ 範圍的檔案:{} / {};整體字節覆蓋率 {:.2}%。",
        tot_in_scope,
        files.len(),
        overall * 100.0
    );
}

fn file_stem(f: &str) -> String {
    std::path::Path::new(f)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| f.to_string())
}
