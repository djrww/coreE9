//! Pest parity 律 —— CL0 載體(主語言)面:`src/cl0.pest` × 手寫 `parse`。
//!
//! 對帳合同:對**法律 CL0**(無 Error 節點)輸入,pest 生成解柝器與手寫
//! `lex` + `parse` 的**非 Trivia token 流**(kind, span)逐項相等;側條件
//! 輸入(含 Bad token / 語法錯)CL0 一律 Error 化 ⟺ pest 失敗(同側)。
//!
//! 紀律同 R₀ 面(pest_r0_parity.rs):pest 只在 dev-dependencies;
//! pest 文法是 EBNF 機器形式,非第二解柝器載體。
//!
//! CL0 特有面(pest 文法如實反映,見 src/cl0.pest 頭註):無優先級扁平
//! 五運算符 / 無指派語句 / 無括號表達式 / let 無型別 / 塊語句必 `;` /
//! 形參實參寬容裸逗號 / `&mut` = Amp+Mut 雙 token 可堆疊 / 型別可重複引用。

use cl0r0::gen::{gen_legal, Rng};
use cl0r0::parse::{parse, Kind, Tree};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser, Debug)]
#[grammar = "cl0.pest"]
struct Cl0Pest;

/// pest 規則名 → CL0 Kind 匿名 token 名(結構規則無 token,返回 None)。
fn pest_kind(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "fn_kw" => "FnKw",
        "let_kw" => "LetKw",
        "mut_kw" => "MutKw",
        "if_kw" => "IfKw",
        "else_kw" => "ElseKw",
        "while_kw" => "WhileKw",
        "true_kw" => "TrueKw",
        "false_kw" => "FalseKw",
        "amp" => "Amp",
        "star" => "Star",
        "plus" => "Plus",
        "minus" => "Minus",
        "eq_eq" => "EqEq",
        "eq" => "Eq",
        "lt" => "Lt",
        "lparen" => "LParen",
        "rparen" => "RParen",
        "lbrace" => "LBrace",
        "rbrace" => "RBrace",
        "semi" => "Semi",
        "colon" => "Colon",
        "comma" => "Comma",
        "IDENT" => "Ident",
        "NUMBER" => "Number",
        _ => return None,
    })
}

/// CL0 token 流:全樹葉節點(非 Trivia)依源碼順序 (kind, start, end)。
fn cl0_tokens(t: &Tree) -> Vec<(&'static str, u32, u32)> {
    let mut out = Vec::new();
    fn rec(t: &Tree, id: u32, out: &mut Vec<(&'static str, u32, u32)>) {
        let n = t.node(id);
        if n.children.is_empty() {
            if n.kind != Kind::Trivia {
                out.push((
                    match n.kind {
                        Kind::FnKw => "FnKw",
                        Kind::LetKw => "LetKw",
                        Kind::MutKw => "MutKw",
                        Kind::IfKw => "IfKw",
                        Kind::ElseKw => "ElseKw",
                        Kind::WhileKw => "WhileKw",
                        Kind::TrueKw => "TrueKw",
                        Kind::FalseKw => "FalseKw",
                        Kind::Amp => "Amp",
                        Kind::Star => "Star",
                        Kind::Plus => "Plus",
                        Kind::Minus => "Minus",
                        Kind::EqEq => "EqEq",
                        Kind::Eq => "Eq",
                        Kind::Lt => "Lt",
                        Kind::LParen => "LParen",
                        Kind::RParen => "RParen",
                        Kind::LBrace => "LBrace",
                        Kind::RBrace => "RBrace",
                        Kind::Semi => "Semi",
                        Kind::Colon => "Colon",
                        Kind::Comma => "Comma",
                        Kind::Ident => "Ident",
                        Kind::Number => "Number",
                        Kind::BadTok => "BadTok",
                        _ => panic!("非 Trivia 葉節點無 token 對應:{:?}", n.kind),
                    },
                    n.span.start,
                    n.span.end,
                ));
            }
            return;
        }
        for &c in &n.children {
            rec(t, c, out);
        }
    }
    rec(t, t.root(), &mut out);
    out
}

/// pest token 流:全部 innermost token pair 依源碼順序 (kind, start, end)。
fn pest_tokens(src: &str) -> Result<Vec<(&'static str, u32, u32)>, pest::error::Error<Rule>> {
    let pairs = Cl0Pest::parse(Rule::program, src)?;
    let mut out = Vec::new();
    fn rec(pairs: pest::iterators::Pairs<Rule>, out: &mut Vec<(&'static str, u32, u32)>) {
        for p in pairs {
            // 生成枚舉變體名 = 規則名(derive Debug 給出)。
            match pest_kind(&format!("{:?}", p.as_rule())) {
                Some(k) => out.push((k, p.as_span().start() as u32, p.as_span().end() as u32)),
                None => rec(p.into_inner(), out),
            }
        }
    }
    rec(pairs, &mut out);
    Ok(out)
}

/// 主律一:手構法律矩陣(覆蓋生成器不觸及的語法角落)pest ≡ 手寫 parse。
#[test]
fn test_pest_cl0_parity_matrix() {
    let cases: &[&str] = &[
        "fn f() {}",
        "fn f(a, b: int, c: &mut int) {}",
        "fn f(, a,, ) {}",       // 寬容:導前/雙/尾隨裸逗號
        "fn g() { g(, a,, ); }", // 寬容:實參裸逗號
        "fn g() { g(); g(a, b,); }",
        "fn h(p: &mut &int) {}",           // 型別可重複引用
        "fn i() { let x = &mut &mut a; }", // 一元堆疊
        "fn j() { let x = *&b; }",
        "fn k() { let x = &&c; }",
        "fn l() { let x; let mut y = 1; let z = true < false + 2 * 3 == 4 - 1; }",
        "fn m() { if a == b { } else if b < c { } else { } }",
        "fn n() { while a < b { c; } }",
        "fn o() { { a + 1; }; }", // 塊語句必 `;`
        "fn p() { return; }",     // `return` 非 CL0 關鍵字 = 標識字
        "fn q() {\n  // c\n  let x = 1; // trail\n}\n",
        "fn f() {}\nfn g() { f(); }", // 多項
    ];
    for (i, src) in cases.iter().enumerate() {
        let t = parse(src).unwrap_or_else(|e| panic!("case {i}: parse: {e:?}\n{src}"));
        assert!(
            t.n_errors() == 0,
            "case {i}: 矩陣案例含 Error(測試集錯誤):\n{src}"
        );
        let a = cl0_tokens(&t);
        let b =
            pest_tokens(src).unwrap_or_else(|e| panic!("case {i}: pest 拒絕法律 CL0: {e}\n{src}"));
        assert_eq!(
            a, b,
            "case {i}: token 流分歧\nsrc: {src}\nCL0: {a:?}\npest: {b:?}"
        );
    }
}

/// 主律二:by-construction 生成樣本(500 輪)`gen_legal` pest ≡ 手寫 parse。
#[test]
fn test_pest_cl0_parity_fuzz() {
    let mut rng = Rng::new(0x0F42_2026_0005);
    for i in 0..500u64 {
        let src = gen_legal(&mut rng);
        let t = parse(&src).unwrap_or_else(|e| panic!("round {i}: parse: {e:?}\n{src}"));
        assert_eq!(
            t.n_errors(),
            0,
            "round {i}: 生成樣本含 Error(生成器 BUG):\n{src}"
        );
        let a = cl0_tokens(&t);
        let b = pest_tokens(&src)
            .unwrap_or_else(|e| panic!("round {i}: pest 拒絕生成樣本: {e}\n{src}"));
        assert_eq!(a, b, "round {i}: token 流分歧\nsrc:\n{src}");
    }
}

/// 邊界律:非法輸入 CL0 產 Error ⟺ pest 失敗(接受集同側)。
#[test]
fn test_pest_cl0_boundary_illegal() {
    let cases: &[&str] = &[
        "fn f() { (a + b); }",        // 括號表達式(CL0 primary 無 LParen 臂)
        "fn f() { a = 1; }",          // 指派語句(CL0 無;僅 let 綁定)
        "fn f() { let x: int = 1; }", // let 型別標註(CL0 不收)
        "fn f() { { a; } }",          // 塊語句無 `;`(無 is_block_stmt 例外)
        "let x = 1;",                 // 頂層語句(項面 = fn)
        "fn f() { a !b; }",           // `!` = 詞法 Bad
        "fn f() { a < ; }",           // 運算符後無操作數
        "fn f() { let; }",            // let 缺標識字
    ];
    for (i, src) in cases.iter().enumerate() {
        let t = parse(src).unwrap_or_else(|e| panic!("case {i}: {e:?}"));
        assert!(
            t.n_errors() > 0,
            "case {i}: CL0 對非法輸入未產 Error(測試集錯誤):\n{src}"
        );
        assert!(
            pest_tokens(src).is_err(),
            "case {i}: pest 接受 CL0 判錯的輸入 —— 接受集偏大:\n{src}"
        );
    }
}
