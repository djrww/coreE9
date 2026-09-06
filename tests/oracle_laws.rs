#![cfg(feature = "oracle")]
//! 外律 O 系(P4-0)—— Tier-A oracle 腳手架的自證。
//! 對應路線:docs/PIVOT-RUSTC-ORACLE.md §八 P4-0。
//!
//! 這裡測的是 **oracle 通道本身**(判決抽取/決定論/期望語法/全化),
//! 不是「本模型 vs rustc」的 parity —— 那是 P4-1 的 `docs/ORACLE-TRACE.md`
//! 範圍,本檔不宣稱。O 系命名與九律並列,標示其為「外律」:
//! 裁判在外的定律,同樣律不過、碼不合。

use cl0r0::oracle::{CliOracle, Expectation, Oracle, RustcError, Verdict};
use cl0r0::span::Span;

/// O-1:經典借用衝突 → rustc 判決 E0502,且主 span 落在 `g(&mut x);` 涉事行
/// (判決攜帶可回跳位址;§1.2 半開區間代數在 oracle 面的延續)。
#[test]
fn oracle_smoke_e0502() {
    let src = r"
fn g(p: &mut i32) { *p = *p + 1; }
fn f() {
    let mut x = 5;
    let r = &x;
    g(&mut x);
    let z = *r;
}
";
    let o = CliOracle::new();
    let rep = o.check(src).expect("Tier-A oracle 應可用(PATH 需有 rustc)");
    assert!(rep.verdict.has_code("E0502"), "判決 = {:?}", rep.verdict);
    let site = src.find("g(&mut x)").expect("測試源碼布局錯誤") as u32;
    let hit = rep
        .verdict
        .primary_spans()
        .iter()
        .any(|s| s.overlaps(&Span::new(site, site + 10)));
    assert!(
        hit,
        "主 span 未落在涉事行:spans = {:?}",
        rep.verdict.primary_spans()
    );
}

/// O-2:乾淨 R₀ 程式 → Accept(無假錯誤;L7a「不產生假錯誤」的 oracle 外延)。
#[test]
fn oracle_accept_clean() {
    let src = r"
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
fn f() {
    let mut i = 0;
    let mut acc = 0;
    while i < 10 {
        acc = acc + add(i, 1);
        i = i + 1;
    }
    if acc > 5 {
        acc = acc - 1;
    } else {
        acc = acc + 1;
    }
}
";
    let o = CliOracle::new();
    let rep = o.check(src).expect("Tier-A oracle 應可用");
    assert!(rep.verdict.is_accept(), "判決 = {:?}", rep.verdict);
    assert!(rep.verdict.codes().is_empty());
}

/// O-3:語法錯誤 → 無碼拒絕(任何輸入皆有判決、絕不 panic;
/// L7 全化律的 oracle 外延。「aborting due to…」摘要行必須已被排除,
/// 否則此測試會把幻影 uncoded 誤判為多餘錯誤)。
#[test]
fn oracle_reject_uncoded_syntax_error() {
    let src = "fn f( {\n    let x = 1;\n}\n";
    let o = CliOracle::new();
    let rep = o.check(src).expect("Tier-A oracle 應可用");
    assert!(!rep.verdict.is_accept(), "判決 = {:?}", rep.verdict);
    let codes = rep.verdict.codes();
    assert!(
        codes.iter().any(|c| c == "uncoded"),
        "應含無碼診斷,codes = {codes:?}"
    );
    // 無碼診斷必須攜帶位址(主 span;可為零字節「插入點」—— span.rs §1.2
    // 明文語義,rustc 對「此處缺某符號」類語法錯誤正是這樣報的)
    assert!(
        !rep.verdict.primary_spans().is_empty(),
        "無碼診斷應攜帶主 span:spans = {:?}",
        rep.verdict.primary_spans()
    );
}

/// O-4:同輸入兩趟判決全等(L2 決定論的 oracle 外延;亦即 oracle_gate
/// 「兩趟全等」null 紀律的單測版)。
#[test]
fn oracle_determinism() {
    let src = r"
fn f() {
    let mut x = 5;
    let r = &x;
    x = 6;
    let z = *r;
}
";
    let o = CliOracle::new();
    let a = o.check(src).expect("Tier-A oracle 應可用");
    let b = o.check(src).expect("Tier-A oracle 應可用");
    assert_eq!(a.verdict, b.verdict, "兩趟判決必須全等");
    assert_eq!(a.rustc_version, b.rustc_version);
    assert!(!a.rustc_version.is_empty(), "版本見證不可為空");
}

/// O-5:檔名 → 期望 的語法與匹配語義(純函數,不觸 rustc;
/// 語料約定本身也是被測對象 —— 約定錯了,gate 會系統性失真)。
#[test]
fn oracle_expectation_grammar() {
    assert_eq!(
        Expectation::from_stem("accept_clean"),
        Some(Expectation::Accept)
    );
    assert_eq!(
        Expectation::from_stem("e0502_classic"),
        Some(Expectation::RejectCode("E0502".to_string()))
    );
    assert_eq!(
        Expectation::from_stem("uncoded_syntax"),
        Some(Expectation::RejectUncoded)
    );
    assert_eq!(Expectation::from_stem("accept"), None, "無 '_' 分隔");
    assert_eq!(Expectation::from_stem("e50_x"), None, "碼非四位");
    assert_eq!(Expectation::from_stem("weird_name"), None, "未知前綴");
    assert_eq!(Expectation::from_stem(""), None);

    // 「含」語義:RejectCode 只要求包含該碼(容許伴生碼)
    let reject = Verdict::Reject {
        errors: vec![RustcError {
            code: Some("E0502".to_string()),
            span: None,
        }],
    };
    assert!(Expectation::RejectCode("E0502".to_string()).matches(&reject));
    assert!(Expectation::RejectCode("e0502".to_string()).matches(&reject));
    assert!(!Expectation::RejectCode("E0384".to_string()).matches(&reject));
    assert!(!Expectation::Accept.matches(&reject));
}

/// O-6(P4-1 主律):rustc × 模型三軌的 parity 矩陣。
/// 三項斷言:
///   ① 分歧全數歸檔 —— 每個 gate 軌道分歧都精確命中 `corpus/PARITY-REGISTRY.json`
///      (檔+軌+向),且註冊項無 stale(分歧不再發生)—— BUG 候選 = 0;
///   ② 幾何保守下界定律:**rustc 以借用衝突類碼(E0499/E0502/E0503/E0506)拒絕
///      的每個案例,Lexical 軌必拒絕** —— 區間幾何下界覆蓋全部借用衝突現場;
///   ③ blanket:Lexical 軌永不漏報(rustc 拒絕而 Lexical 接受 = 不可接受)。
#[test]
fn oracle_parity_borrow_matrix() {
    use cl0r0::model::{parity_dir, parity_violations, ParityRegistry};
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/corpus/curated");
    let reg_path = concat!(env!("CARGO_MANIFEST_DIR"), "/corpus/PARITY-REGISTRY.json");
    let o = CliOracle::new();
    let cases = parity_dir(dir, &o).expect("parity 語料應可跑(PATH 需有 rustc)");
    let reg = ParityRegistry::from_json(&std::fs::read_to_string(reg_path).unwrap())
        .expect("註冊表應可解析");

    assert!(
        cases.len() >= 35,
        "種子語料應 ≥35 案例(實得 {})",
        cases.len()
    );
    let in_scope = cases.iter().filter(|c| c.in_scope).count();
    assert!(in_scope >= cases.len() - 1, "至多 1 案例(語法錯誤)範圍外");

    // ① 分歧 = 註冊表(精確集合相等)
    let violations = parity_violations(&cases, &reg);
    assert!(
        violations.is_empty(),
        "parity 違規 {} 條:{violations:?}",
        violations.len()
    );

    // ② 幾何保守下界定律:借用衝突類碼 ⇒ Lexical 必報
    let borrow_codes = cl0r0::model::BORROW_CONFLICT_CODES;
    let mut n_borrow = 0usize;
    for c in &cases {
        if c.rustc_codes
            .iter()
            .any(|x| borrow_codes.contains(&x.as_str()))
        {
            let lx = c
                .tracks
                .iter()
                .find(|t| t.track == "lexical")
                .unwrap_or_else(|| panic!("{} 應有 lexical 軌判決", c.file));
            assert!(
                lx.reject,
                "幾何下界定律被違反:{} rustc 報借用衝突而 Lexical 接受",
                c.file
            );
            n_borrow += 1;
        }
    }
    assert!(n_borrow >= 8, "借用衝突類案例應 ≥8(實得 {n_borrow})");

    // ③ blanket 復證:Lexical 的任何 under 必屬「範圍外家族」
    //(移動 E0382/E0505、不可變性 E0384、逃逸 E0597 —— 已申報的模型邊界)
    for c in &cases {
        if !c.in_scope || c.rustc_accept {
            continue;
        }
        let lx_under = c
            .tracks
            .iter()
            .find(|t| t.track == "lexical")
            .is_some_and(|t| !t.reject);
        if lx_under {
            let out_of_scope_family = c
                .rustc_codes
                .iter()
                .all(|x| !borrow_codes.contains(&x.as_str()));
            assert!(
                out_of_scope_family,
                "{}:Lexical under 但碼 {:?} 不屬範圍外家族",
                c.file, c.rustc_codes
            );
        }
    }
}

/// O-7(P4-2 主律):生成式差分 agreement。
/// N 輪 by-construction 樣本(`gen::gen_r0_semantic`)× rustc 判決 × 模型三軌:
///   ① 生成器期望 × rustc 判決 0 失配(生成時即知判決的構造正確性);
///   ② 逐樣本幾何保守下界:rustc 以借用衝突類碼拒絕 ⇒ Lexical 軌必拒;
///   ③ 全部樣本 in-scope(純 R₀,受驗對象在模型域內)。
/// 失敗時:ddmin 縮到最小反例 → 歸檔 `tests/fixtures/`(P0 #3 防線沿用)。
#[test]
fn oracle_fuzz_agreement() {
    use cl0r0::model::{fuzz_agreement, model_check};
    use cl0r0::shrink::shrink_to_minimal;
    let rounds: u64 = std::env::var("CL0R0_ORACLE_FUZZ_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(250);
    let o = CliOracle::new();
    let rep = fuzz_agreement(&o, rounds, 0x0F42_2026_0002);
    assert_eq!(
        rep.rustc_version,
        o.cached_version().expect("版本見證"),
        "版本見證不一致"
    );
    if !rep.failures.is_empty() {
        let f = &rep.failures[0];
        // shrink 保持謂詞(依律選取):s 仍使該律失敗。
        let prop: Box<dyn Fn(&str) -> bool> =
            match f.law.as_str() {
                "expectation-accept-rejected" => {
                    Box::new(|s| !o.check(s).map(|r| r.verdict.is_accept()).unwrap_or(true))
                }
                "expectation-reject-accepted" => {
                    Box::new(|s| o.check(s).map(|r| r.verdict.is_accept()).unwrap_or(false))
                }
                "expectation-reject-wrong-code" => Box::new(|s| {
                    o.check(s)
                        .map(|r| {
                            !r.verdict.is_accept()
                                && !r.verdict.codes().iter().any(|c| {
                                    cl0r0::model::BORROW_CONFLICT_CODES.contains(&c.as_str())
                                })
                        })
                        .unwrap_or(false)
                }),
                "lower-bound" => Box::new(|s| {
                    let m = model_check(s);
                    m.in_scope
                        && o.check(s)
                            .map(|r| {
                                !r.verdict.is_accept()
                                    && r.verdict.codes().iter().any(|c| {
                                        cl0r0::model::BORROW_CONFLICT_CODES.contains(&c.as_str())
                                    })
                                    && !m
                                        .tracks
                                        .iter()
                                        .find(|t| t.track == "lexical")
                                        .expect("lexical 軌")
                                        .reject
                            })
                            .unwrap_or(false)
                }),
                _ => Box::new(|s| !model_check(s).in_scope),
            };
        let m = shrink_to_minimal(&f.src, &prop);
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
        let _ = std::fs::create_dir_all(dir);
        let path = format!("{dir}/oracle_fuzz_{}.txt", f.law);
        let _ = std::fs::write(&path, &m);
        eprintln!("O-7 失敗[{}]:最小反例已歸檔 {path}:\n{m}", f.law);
    }
    assert!(
        rep.failures.is_empty(),
        "fuzz agreement 失敗 {} 例(首三:{:#?})",
        rep.failures.len(),
        rep.failures.iter().take(3).collect::<Vec<_>>()
    );
    // 樣本宇宙不退化(構造紀律的分布見證)。
    let third = (rounds / 3) as usize;
    assert!(
        rep.n_accept_expect >= third,
        "accept 樣本應 ≥1/3(實得 {})",
        rep.n_accept_expect
    );
    assert!(
        rep.n_reject_borrow_expect + rep.n_reject_move_expect >= third,
        "衝突樣本應 ≥1/3(實得 {})",
        rep.n_reject_borrow_expect + rep.n_reject_move_expect
    );
}

/// O-8(P4-3 主律之一):place 敏感度正交性。
/// `&mut s.a` 與 `&mut s.b` 作用於**不同 place** ⇒ Nll/Referent 軌不衝突
/// (雙錨:模型放行 ∧ rustc 接受);同 place(`&mut s.a` ×2)⇒ 必拒
/// (雙錨:模型拒絕 ∧ rustc 拒絕)。Lexical 軌保持綁定粒度(下界,可過報)。
#[test]
fn test_law_semantic_place_orthogonality() {
    use cl0r0::model::model_check;
    let o = CliOracle::new();

    // 正交:不同字段
    let disjoint = "\
struct S {
    a: i32,
    b: i32,
}
fn f(s: &mut S) {
    let p = &mut s.a;
    let q = &mut s.b;
    *p = 1;
    *q = 2;
}
";
    let rep = o.check(disjoint).expect("oracle 應可用");
    assert!(
        rep.verdict.is_accept(),
        "rustc 應接受正交字段借用:{:?}",
        rep.verdict
    );
    let m = model_check(disjoint);
    assert!(m.in_scope);
    let by = |l: &str| m.tracks.iter().find(|t| t.track == l).unwrap().reject;
    assert!(!by("nll"), "Nll 應對正交字段放行");
    assert!(!by("referent"), "Referent 應對正交字段放行");

    // 對照:同 place(同一字段兩次 &mut)必拒
    let same = "\
struct S {
    a: i32,
}
fn f(s: &mut S) {
    let p = &mut s.a;
    let q = &mut s.a;
    *p = 1;
    *q = 2;
}
";
    let rep = o.check(same).expect("oracle 應可用");
    assert!(!rep.verdict.is_accept(), "rustc 應拒絕同 place 雙 &mut");
    let m = model_check(same);
    assert!(m.in_scope);
    let by = |l: &str| m.tracks.iter().find(|t| t.track == l).unwrap().reject;
    assert!(by("nll"), "Nll 必拒同 place 雙 &mut");
    assert!(by("referent"), "Referent 必拒同 place 雙 &mut");
}

/// O-9(P4-3 主律之二):CFG 精確 killer(分支不相交 + 迴圈回邊)。
///   (a) if/else 分支不相交:借用只在 then 用、寫只在 else ⇒ 模型放行 ∧ rustc 接受;
///   (b) while 回邊:`while *m > 3 { x = 6; }` —— 條件重複求值使借用跨越
///       source 線性最後使用點 ⇒ 模型必拒 ∧ rustc 拒絕(F-E 家族,已消滅);
///   (c) 迴圈後才借用:迴圈內寫不影響 ⇒ 模型放行 ∧ rustc 接受。
#[test]
fn test_law_semantic_cfg_liveness() {
    use cl0r0::model::model_check;
    let o = CliOracle::new();

    // (a) 分支不相交
    let branches = "\
fn f() {
    let mut x = 5;
    let r = &x;
    if x > 3 {
        let u = *r;
    } else {
        x = 6;
    }
}
";
    let rep = o.check(branches).expect("oracle 應可用");
    assert!(
        rep.verdict.is_accept(),
        "rustc 應接受分支不相交:{:?}",
        rep.verdict
    );
    let m = model_check(branches);
    let by = |l: &str| m.tracks.iter().find(|t| t.track == l).unwrap().reject;
    assert!(!by("nll") && !by("referent"), "模型應對分支不相交放行");

    // (b) 迴圈回邊(F-E 形状)
    let backedge = "\
fn f() {
    let mut x = 5;
    let m = &mut x;
    while *m > 3 {
        x = 6;
    }
}
";
    let rep = o.check(backedge).expect("oracle 應可用");
    assert!(!rep.verdict.is_accept(), "rustc 應拒絕回邊冲突");
    let m = model_check(backedge);
    let by = |l: &str| m.tracks.iter().find(|t| t.track == l).unwrap().reject;
    assert!(by("nll"), "Nll 必拒回邊冲突(S2 回邊活性)");
    assert!(by("referent"), "Referent 必拒回邊冲突(S2 回邊活性)");

    // (c) 迴圈後借用
    let after_loop = "\
fn f() {
    let mut x = 5;
    while x > 3 {
        x = x - 1;
    }
    let r = &x;
    let z = *r;
}
";
    let rep = o.check(after_loop).expect("oracle 應可用");
    assert!(
        rep.verdict.is_accept(),
        "rustc 應接受迴圈後借用:{:?}",
        rep.verdict
    );
    let m = model_check(after_loop);
    let by = |l: &str| m.tracks.iter().find(|t| t.track == l).unwrap().reject;
    assert!(!by("nll") && !by("referent"), "模型應對迴圈後借用放行");
}
