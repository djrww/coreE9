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
