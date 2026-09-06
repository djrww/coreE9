//! oracle —— Tier-A 語料對帳跑者(P4-0;docs/PIVOT-RUSTC-ORACLE.md §四)。
//!
//! 用法:
//!   cargo run --features oracle --bin oracle -- run (--json, corpus_dir 皆可選)
//!   cargo run --features oracle --bin oracle -- version
//!
//! 語料檔名即期望(零配置;`cl0r0::oracle::Expectation::from_stem`):
//!   `accept_<name>.rs`     → 期望 rustc 接受
//!   `e<四位碼>_<name>.rs`   → 期望 rustc 拒絕且含該錯誤碼(如 `e0502_classic.rs`)
//!   `uncoded_<name>.rs`    → 期望 rustc 拒絕且含無碼診斷(語法錯誤類)
//!
//! 輸出:人讀表格 + 匯總;`--json` 追加一行 `ORACLE_JSON:{…}`
//! (供 `tools/oracle_gate.py` 消費)。任何案例不符期望(BUG 候選)
//! 或檔名無法推導期望 → exit 1 —— 律不過,碼不合。

use cl0r0::model;
use cl0r0::oracle::{CliOracle, Expectation, Oracle, Verdict};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("version") => {
            let o = CliOracle::new();
            match o.toolchain_version() {
                Ok(v) => println!("{v}"),
                Err(e) => {
                    eprintln!("oracle 錯誤:{e}");
                    std::process::exit(1);
                }
            }
        }
        Some("parity") => {
            let json = args.iter().any(|a| a == "--json");
            let dir = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with("--"))
                .cloned()
                .unwrap_or_else(|| "corpus/curated".to_string());
            std::process::exit(run_parity(&dir, json));
        }
        Some("run") => {
            let json = args.iter().any(|a| a == "--json");
            let dir = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with("--"))
                .cloned()
                .unwrap_or_else(|| "corpus/curated".to_string());
            std::process::exit(run_corpus(&dir, json));
        }
        other => {
            eprintln!(
                "用法:oracle run [--json] [dir] | oracle parity [--json] [dir] | oracle version(實得參數 {other:?})"
            );
            std::process::exit(2);
        }
    }
}

/// parity 跑者:rustc × 模型三軌,逐案例對帳;違規(未註冊分歧/stale/lexical 漏報)
/// → exit 1。JSON 行 ORACLE_PARITY_JSON:{…} 供 gate 消費。
fn run_parity(dir: &str, json: bool) -> i32 {
    let o = CliOracle::new();
    let version = match o.toolchain_version() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("oracle 錯誤:{e}");
            return 1;
        }
    };
    let cases = match model::parity_dir(dir, &o) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("parity 失敗:{e}");
            return 1;
        }
    };
    let reg_text = std::fs::read_to_string("corpus/PARITY-REGISTRY.json")
        .unwrap_or_else(|_| "{\"entries\":[]}".to_string());
    let reg = match model::ParityRegistry::from_json(&reg_text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("註冊表解析失敗:{e}");
            return 1;
        }
    };
    let violations = model::parity_violations(&cases, &reg);

    println!("======================================================================");
    println!(" parity 對帳:rustc × 模型三軌(語料 {dir})");
    println!(" rustc {version}");
    println!("======================================================================");
    for c in &cases {
        let l = c
            .tracks
            .iter()
            .find(|t| t.track == "lexical")
            .map(|t| if t.reject { "R" } else { "." })
            .unwrap_or("-");
        let n = c
            .tracks
            .iter()
            .find(|t| t.track == "nll")
            .map(|t| if t.reject { "R" } else { "." })
            .unwrap_or("-");
        let r = c
            .tracks
            .iter()
            .find(|t| t.track == "referent")
            .map(|t| if t.reject { "R" } else { "." })
            .unwrap_or("-");
        let rustc = if c.rustc_accept {
            "accept".to_string()
        } else {
            format!("reject[{}]", c.rustc_codes.join(","))
        };
        let scope = if c.in_scope {
            String::new()
        } else {
            format!("  ← 範圍外:{}", c.scope_note.as_deref().unwrap_or("?"))
        };
        let mm: Vec<String> = c
            .mismatches
            .iter()
            .map(|(t, d)| format!("{t}:{d}"))
            .collect();
        let mm_s = if mm.is_empty() {
            String::new()
        } else {
            format!("  ⚠ {}", mm.join(" "))
        };
        println!(
            "  {:<44} rustc {:<18} L:{} N:{} R:{}{}{}",
            c.file, rustc, l, n, r, scope, mm_s
        );
    }
    let in_scope = cases.iter().filter(|c| c.in_scope).count();
    let mism = cases.iter().map(|c| c.mismatches.len()).sum::<usize>();
    println!("----------------------------------------------------------------------");
    println!(
        " {}/{} 在模型範圍內;gate 軌道分歧 {} 項(註冊表 {} 項)",
        in_scope,
        cases.len(),
        mism,
        reg.entries.len()
    );

    if json {
        let cases_js: Vec<String> = cases
            .iter()
            .map(|c| {
                format!(
                    "{{\"file\":{},\"rustc_accept\":{},\"rustc_codes\":[{}],\"in_scope\":{},\"tracks\":[{}],\"mismatches\":[{}]}}",
                    jstr(&c.file),
                    c.rustc_accept,
                    c.rustc_codes.iter().map(|x| jstr(x)).collect::<Vec<_>>().join(","),
                    c.in_scope,
                    c.tracks
                        .iter()
                        .map(|t| format!(
                            "{{\"track\":{},\"reject\":{},\"red_edges\":{},\"first_red\":{}}}",
                            jstr(t.track),
                            t.reject,
                            t.red_edges,
                            t.first_red
                                .map(|s| format!("[{},{}]", s.start, s.end))
                                .unwrap_or_else(|| "null".to_string())
                        ))
                        .collect::<Vec<_>>()
                        .join(","),
                    c.mismatches
                        .iter()
                        .map(|(t, d)| {
                            format!("{{\"track\":{},\"direction\":{}}}", jstr(t), jstr(d))
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect();
        println!(
            "ORACLE_PARITY_JSON:{{\"rustc_version\":{},\"cases\":[{}],\"violations\":[{}]}}",
            jstr(&version),
            cases_js.join(","),
            violations
                .iter()
                .map(|v| jstr(v))
                .collect::<Vec<_>>()
                .join(",")
        );
    }

    if !violations.is_empty() {
        for v in &violations {
            eprintln!("[RED] {v}");
        }
        1
    } else {
        println!("[GREEN] parity:已註冊分歧全數歸檔,無 BUG 候選,無 stale 項");
        0
    }
}

/// 單案例結果(gate 的消費單位)。
struct CaseOut {
    file: String,
    expect: String,
    verdict: String,
    codes: Vec<String>,
    spans: Vec<(u32, u32)>,
    pass: bool,
    note: Option<String>,
}

fn verdict_label(v: &Verdict) -> String {
    if v.is_accept() {
        "accept".to_string()
    } else {
        format!("reject[{}]", v.codes().join(","))
    }
}

fn run_corpus(dir: &str, json: bool) -> i32 {
    let o = CliOracle::new();
    let version = match o.toolchain_version() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("oracle 錯誤:{e}");
            return 1;
        }
    };
    let dir_path = PathBuf::from(dir);
    let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir_path) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "rs"))
            .collect(),
        Err(e) => {
            eprintln!("語料目錄不可讀({dir}):{e}");
            return 1;
        }
    };
    if files.is_empty() {
        eprintln!("語料目錄空或無 .rs 案例:{dir}");
        return 1;
    }
    files.sort();

    let mut outs = Vec::new();
    for f in files {
        let file = f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = f
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let src = match std::fs::read_to_string(&f) {
            Ok(s) => s,
            Err(e) => {
                outs.push(CaseOut {
                    file,
                    expect: "?".to_string(),
                    verdict: "?".to_string(),
                    codes: Vec::new(),
                    spans: Vec::new(),
                    pass: false,
                    note: Some(format!("讀取失敗:{e}")),
                });
                continue;
            }
        };
        let (expect, note) = match Expectation::from_stem(&stem) {
            Some(e) => (e, None),
            None => (
                Expectation::Accept,
                Some("檔名無法推導期望(約定:accept_* / e<四位碼>_* / uncoded_*)".to_string()),
            ),
        };
        match o.check(&src) {
            Ok(rep) => {
                let pass = note.is_none() && expect.matches(&rep.verdict);
                outs.push(CaseOut {
                    file,
                    expect: expect.label(),
                    verdict: verdict_label(&rep.verdict),
                    codes: rep.verdict.codes(),
                    spans: rep
                        .verdict
                        .primary_spans()
                        .iter()
                        .map(|s| (s.start, s.end))
                        .collect(),
                    pass,
                    note,
                });
            }
            Err(e) => {
                outs.push(CaseOut {
                    file,
                    expect: expect.label(),
                    verdict: "?".to_string(),
                    codes: Vec::new(),
                    spans: Vec::new(),
                    pass: false,
                    note: Some(format!("oracle 錯誤:{e}")),
                });
            }
        }
    }

    println!("======================================================================");
    println!(" Tier-A oracle 對帳:rustc 判決 vs 檔名期望(語料 {dir})");
    println!(" rustc {version}");
    println!("======================================================================");
    for c in &outs {
        let mark = if c.pass { "✅" } else { "❌" };
        let note = c
            .note
            .as_deref()
            .map(|n| format!("  ← {n}"))
            .unwrap_or_default();
        println!(
            "  {:<36} 期望 {:<8} 實得 {:<26} {}{}",
            c.file, c.expect, c.verdict, mark, note
        );
    }
    let fail = outs.iter().filter(|c| !c.pass).count();
    println!("----------------------------------------------------------------------");
    println!(
        " {}/{} 通過;BUG 候選 {} 個(不符期望)",
        outs.len() - fail,
        outs.len(),
        fail
    );

    if json {
        let cases: Vec<String> = outs
            .iter()
            .map(|c| {
                format!(
                    "{{\"file\":{},\"expect\":{},\"verdict\":{},\"codes\":[{}],\"spans\":[{}],\"pass\":{},\"note\":{}}}",
                    jstr(&c.file),
                    jstr(&c.expect),
                    jstr(&c.verdict),
                    c.codes.iter().map(|x| jstr(x)).collect::<Vec<_>>().join(","),
                    c.spans
                        .iter()
                        .map(|(a, b)| format!("[{a},{b}]"))
                        .collect::<Vec<_>>()
                        .join(","),
                    c.pass,
                    c.note.as_deref().map(jstr).unwrap_or_else(|| "null".to_string()),
                )
            })
            .collect();
        println!(
            "ORACLE_JSON:{{\"rustc_version\":{},\"cases\":[{}]}}",
            jstr(&version),
            cases.join(",")
        );
    }

    if fail > 0 {
        1
    } else {
        0
    }
}

/// JSON 字串字面量(轉義 `"`、`\`、控制字元;本跑者輸出僅含字串/整數/布林)。
fn jstr(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
