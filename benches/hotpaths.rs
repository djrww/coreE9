//! 熱點內核基準(P1 #7;零依賴自研計時器,harness = false)。
//!
//! 運行:`cargo bench --bench hotpaths [-- --reps 11 --newman-reps 5]`。
//! 輸出:人讀表格 + 一行 `BENCH_JSON:{...}`(median / MAD / min / max,ms/樣本)。
//! 比較:`tools/bench_gate.py`(對 `bench/BASELINE.json`,容差 ±25%,經 null 內核
//!       漂移校正 + MAD 雜訊帶)。基線重生成:`--emit-baseline bench/BASELINE.json`。
//!
//! ## 為什麼要這樣設計(2026-09-02 修 gate 時寫死,勿再退化)
//! * 舊版只跑 **5 輪**取中位數,且把「整批語料」壓成一個計時區間 ⇒ 單樣本 ~µs
//!   級的量被 ns 級計時器與 CPU 頻率漂移淹沒 ⇒ `laminar`/`named_sexp` 在不同
//!   機器上自然出現 1.26–1.35 倍的「假回歸」。
//! * 修法三件:(1) 重複數 n≥7(預設 9)並回報 MAD;(2) 加 **`null` 參考內核**
//!   (只做 `black_box` 迴圈)當環境標尺,gate 用 `cur/base ÷ null_cur/null_base`
//!   抵消頻率/搶核漂移;(3) 每輪計時涵蓋整個語料(1000 樣本 ≈ ms 級),把
//!   計時粒度從 µs 推到 ms。
//!
//! 語料:固定種子生成 —— 500 合法程式 + 500 垃圾輸入(混裝,與 fuzz 同宇宙),
//! 與 200 個半截檔案(L7b 淨化內核)。

use cl0r0::gen::{gen_garbage, gen_half_file, gen_legal, Rng};
use std::time::Instant;

/// 環境參考內核:只走一遍字串(取長度 + 掃一個位元組),**不碰任何被測代碼**。
/// 它是「記憶體 + 時脈」的標尺:同一台機器若因搶核/降頻變慢 x%,null 也會變慢
/// 約 x%,gate 據此把漂移從各內核比值中除掉。刻意做得比空迴圈重,否則它自己
/// 就淹死在計時噪聲裡(第一版用 `black_box(s.len())`,中位數 2 ns —— 無效)。
#[inline(never)]
fn null_kernel(s: &str) -> usize {
    let mut acc = std::hint::black_box(s.len());
    for b in s.as_bytes().iter().step_by(7).take(24) {
        acc = acc.wrapping_add(std::hint::black_box(*b) as usize);
    }
    acc
}

struct Stat {
    med: f64,
    mad: f64,
    min: f64,
    max: f64,
    reps: usize,
    samples: usize,
}

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        xs[n / 2]
    } else {
        (xs[n / 2 - 1] + xs[n / 2]) / 2.0
    }
}

/// 重複 `reps` 輪;每輪計時涵蓋整批語料 ⇒ 回傳「ms/樣本」的分佈統計。
fn measure(name: &str, samples: &[String], reps: usize, iters: usize, f: impl Fn(&str)) -> Stat {
    for _ in 0..iters {
        for s in samples {
            f(s); // 預熱
        }
    }
    let mut xs = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t0 = Instant::now();
        for _ in 0..iters {
            for s in samples {
                f(s);
            }
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / (samples.len().max(1) * iters.max(1)) as f64;
        xs.push(ms);
    }
    let med = median(xs.clone());
    let mad = median(xs.iter().map(|v| (v - med).abs()).collect());
    let stat = Stat {
        med,
        mad,
        min: xs.iter().cloned().fold(f64::INFINITY, f64::min),
        max: xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        reps,
        samples: samples.len(),
    };
    println!(
        "{:<18} {:>9.4} ms/樣本  (MAD {:.3} · min {:>8.4} · max {:>8.4} · n={} × {} 樣本)",
        name, stat.med, stat.mad, stat.min, stat.max, stat.reps, stat.samples
    );
    stat
}

struct Args {
    reps: usize,
    newman_reps: usize,
    null_reps: usize,
    skip: Vec<String>,
    emit: Option<String>,
}

const NEEDS_VALUE: [&str; 5] = [
    "--reps",
    "--newman-reps",
    "--null-reps",
    "--skip",
    "--emit-baseline",
];

fn parse_args() -> Args {
    let mut a = Args {
        reps: 9,
        newman_reps: 5,
        null_reps: 9,
        skip: Vec::new(),
        emit: None,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            f if NEEDS_VALUE.contains(&f) && i + 1 >= argv.len() => {
                eprintln!("missing value for {f}");
                std::process::exit(2);
            }
            "--reps" => {
                a.reps = argv[i + 1].parse().unwrap();
                i += 2;
            }
            "--newman-reps" => {
                a.newman_reps = argv[i + 1].parse().unwrap();
                i += 2;
            }
            "--null-reps" => {
                a.null_reps = argv[i + 1].parse().unwrap();
                i += 2;
            }
            "--skip" => {
                a.skip.push(argv[i + 1].clone());
                i += 2;
            }
            "--emit-baseline" => {
                a.emit = Some(argv[i + 1].clone());
                i += 2;
            }
            // cargo 會把 --bench / -- 之类的旗標轉遞進來:未知的「單詞旗標」
            // 一律忽略(不帶值),其他才報錯。
            other if other.starts_with("--") && !other.contains('=') => {
                i += 1;
            }
            other => {
                eprintln!("unknown flag: {other}");
                std::process::exit(2);
            }
        }
    }
    a
}

fn main() {
    let args = parse_args();
    let skipped = |k: &str| args.skip.iter().any(|s| s == k);

    println!("========================================================================");
    println!("  cl0r0 熱點內核基準(零依賴計時;n≥7 中位數 + MAD;含 null 參考內核)");
    println!("========================================================================");

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

    let mut out: Vec<(String, Stat)> = Vec::new();

    // ★ null 參考內核:與被測內核走同一份語料、同一套計時結構,但不含被测代碼。
    // iters=20:單遍 null 只有 ~10 ns/樣本,會淹死在計時噪聲;重複 20 遍把它
    // 推到 ~0.2 µs/樣本(每輪 ~0.2 ms),仍是「同一份語料的纯記憶體掃描」。
    let null_stat = measure("null(參考)", &mixed, args.null_reps, 20, |s| {
        std::hint::black_box(null_kernel(s));
    });
    out.push(("null".to_string(), null_stat));

    if !skipped("lex") {
        out.push((
            "lex".to_string(),
            measure("lex", &mixed, args.reps, 1, |s| {
                cl0r0::lex::lex(s);
            }),
        ));
    }
    if !skipped("parse") {
        out.push((
            "parse".to_string(),
            measure("parse", &mixed, args.reps, 1, |s| {
                let _ = cl0r0::parse::parse(s).unwrap();
            }),
        ));
    }
    if !skipped("laminar") {
        out.push((
            "laminar".to_string(),
            measure("laminar(含 parse)", &legal, args.reps, 1, |s| {
                let t = cl0r0::parse::parse(s).unwrap();
                let _ = t.laminar_ok();
            }),
        ));
    }
    if !skipped("named_sexp") {
        out.push((
            "named_sexp".to_string(),
            measure("named_sexp(含 parse)", &legal, args.reps, 1, |s| {
                let t = cl0r0::parse::parse(s).unwrap();
                let _ = t.named_sexp();
            }),
        ));
    }
    if !skipped("l7b_evaluate") {
        out.push((
            "l7b_evaluate".to_string(),
            measure("l7b_evaluate", &half, args.reps, 1, |s| {
                let _ = cl0r0::tree::l7b_evaluate(s);
            }),
        ));
    }
    if !skipped("r0_lex") {
        out.push((
            "r0_lex".to_string(),
            measure("r0_lex", &mixed, args.reps, 1, |s| {
                let _ = cl0r0::r0::r0_lex(s);
            }),
        ));
    }
    if !skipped("r0_parse") {
        out.push((
            "r0_parse".to_string(),
            measure("r0_parse", &mixed, args.reps, 1, |s| {
                let _ = cl0r0::r0::r0_parse(s).unwrap();
            }),
        ));
    }
    if !skipped("newman_3x4x6") {
        // Newman 通道:每次即 2,400 狀態的全量檢查 ⇒ 每次為一個樣本。
        // 與其他內核**同格式**走 measure():先預熱、再取 n 輪的最小值(best-of-n)。
        // 它是多執行緒通道,搶核時 median 會被拉高 30%+(實測 v2 趟),故一律用 min。
        let newman_corpus: Vec<String> = vec!["3x4x6".to_string(); 1];
        let nrep = args.newman_reps.max(5);
        let stat = measure("newman_3x4x6", &newman_corpus, nrep, 1, |_seed| {
            let rep = cl0r0::l9newman::newman_check(
                cl0r0::rep::Menu::CommutativeTrim,
                cl0r0::rep::Policy::Guarded,
                3,
                4,
                6,
            );
            std::hint::black_box(rep.states);
        });
        out.push(("newman_3x4x6".to_string(), stat));
    }

    println!("----------------------------------------------------------------------");
    println!("  (中位數 over n 輪;語料:500 合法 + 500 垃圾 + 200 半截;null 為環境參考)");

    let json = out
        .iter()
        .map(|(k, v)| {
            format!(
                "\"{}\":{{\"median_ms_per_sample\":{:.6},\"mad_ms\":{:.6},\"min_ms\":{:.6},\"max_ms\":{:.6},\"reps\":{},\"samples_per_rep\":{}}}",
                k, v.med, v.mad, v.min, v.max, v.reps, v.samples
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    println!("BENCH_JSON:{{\"kernels\":{{{json}}},\"harness\":\"hotpaths-v2\"}}");

    if let Some(path) = &args.emit {
        let body = out
            .iter()
            .map(|(k, v)| {
                format!(
                    "\"{}\": {{\"median_ms_per_sample\": {:.6}, \"mad_ms\": {:.6}, \"min_ms\": {:.6}, \"max_ms\": {:.6}, \"reps\": {}, \"samples_per_rep\": {}}}",
                    k, v.med, v.mad, v.min, v.max, v.reps, v.samples
                )
            })
            .collect::<Vec<_>>()
            .join(",\n    ");
        let doc = format!(
            "{{\n  \"note\": \"沙箱參考基線(2 核 x86-64,release fat-LTO)。單位 ms/樣本(n≥7 中位數 + MAD)。`null` 為環境參考內核:gate 以它對所有比值做漂移校正。重生成:cargo bench --bench hotpaths -- --emit-baseline bench/BASELINE.json\",\n  \"harness\": \"hotpaths-v2\",\n  \"kernels\": {{\n    {body}\n  }}\n}}\n"
        );
        std::fs::write(path, doc).expect("write baseline");
        println!("(基線已寫入 {path})");
    }
}
