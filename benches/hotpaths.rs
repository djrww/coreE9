//! 熱點內核基準(P1 #7;零依賴自研計時器,harness = false)。
//!
//! 運行:`cargo bench --bench hotpaths`。
//! 輸出:人讀表格 + 一行 `BENCH_JSON:{...}`(中位數,ms/樣本)。
//! 比較:見 `tools/bench_gate.py`(與 `bench/BASELINE.json` 做 ±tol 回歸門)。
//!
//! 語料:固定種子生成 —— 500 合法程式 + 500 垃圾輸入(混裝,與 fuzz 同宇宙),
//! 與 200 個半截檔案(L7b 淨化內核)。

use cl0r0::gen::{gen_garbage, gen_half_file, gen_legal, Rng};
use std::time::Instant;

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[xs.len() / 2]
}

/// 對語料整體跑 `f`,重複直到至少 5 次完整測量;回傳每次每樣本的毫秒數中位數。
fn measure(name: &str, samples: &[String], f: impl Fn(&str)) -> f64 {
    for s in samples {
        f(s); // 預熱
    }
    let mut per_sample = Vec::new();
    for _ in 0..5 {
        let t0 = Instant::now();
        for s in samples {
            f(s);
        }
        per_sample.push(t0.elapsed().as_secs_f64() * 1000.0 / samples.len().max(1) as f64);
    }
    let ms = median(per_sample);
    println!("{:<12} {:>10.4} ms/樣本", name, ms);
    ms
}

fn main() {
    println!("======================================================================");
    println!("  cl0r0 熱點內核基準(零依賴計時;樣本 = 種子固定生成)");
    println!("======================================================================");

    let mut rng = Rng::new(0xB0B);
    let mut legal: Vec<String> = Vec::new();
    for _ in 0..500 {
        legal.push(gen_legal(&mut rng));
    }
    let mut garbage: Vec<String> = Vec::new();
    for _ in 0..500 {
        garbage.push(gen_garbage(&mut rng, 60));
    }
    let mut half: Vec<String> = Vec::new();
    for _ in 0..200 {
        let l = gen_legal(&mut rng);
        half.push(gen_half_file(&mut rng, &l));
    }
    let mut mixed: Vec<String> = Vec::with_capacity(legal.len() + garbage.len());
    mixed.extend(legal.iter().cloned());
    mixed.extend(garbage.iter().cloned());

    let mut out: Vec<(String, f64)> = Vec::new();
    let mut push = |name: &str, ms: f64| out.push((name.to_string(), ms));

    push(
        "lex",
        measure("lex", &mixed, |s| {
            cl0r0::lex::lex(s);
        }),
    );
    push(
        "parse",
        measure("parse", &mixed, |s| {
            let _ = cl0r0::parse::parse(s).unwrap();
        }),
    );
    push(
        "laminar",
        measure("laminar(含 parse)", &legal, |s| {
            let t = cl0r0::parse::parse(s).unwrap();
            let _ = t.laminar_ok();
        }),
    );
    push(
        "named_sexp",
        measure("named_sexp(含 parse)", &legal, |s| {
            let t = cl0r0::parse::parse(s).unwrap();
            let _ = t.named_sexp();
        }),
    );
    push(
        "l7b_evaluate",
        measure("l7b_evaluate", &half, |s| {
            let _ = cl0r0::tree::l7b_evaluate(s);
        }),
    );
    push(
        "r0_lex",
        measure("r0_lex", &mixed, |s| {
            let _ = cl0r0::r0::r0_lex(s);
        }),
    );
    push(
        "r0_parse",
        measure("r0_parse", &mixed, |s| {
            let _ = cl0r0::r0::r0_parse(s).unwrap();
        }),
    );

    // Newman 通道(小空間絕對基準;並行分塊)
    let t0 = Instant::now();
    let rep = cl0r0::l9newman::newman_check(
        cl0r0::rep::Menu::CommutativeTrim,
        cl0r0::rep::Policy::Guarded,
        3,
        4,
        6,
    );
    let newman_ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{:<12} {:>10.1} ms/次(3×4×6,{} 狀態)",
        "newman", newman_ms, rep.states
    );
    push("newman_3x4x6", newman_ms);

    println!("======================================================================");
    println!("  (中位數;語料:500 合法 + 500 垃圾 + 200 半截)");
    let json = out
        .iter()
        .map(|(k, v)| format!("\"{}\":{:.4}", k, v))
        .collect::<Vec<_>>()
        .join(",");
    println!("BENCH_JSON:{{{}}}", json);
}
