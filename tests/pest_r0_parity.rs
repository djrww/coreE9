//! Pest parity 律(EBNF 附錄 B 機器可讀面;`src/r0.pest` × 手寫 `r0_parse`)。
//!
//! 對帳合同:對**法律 R₀**(無 Error/Unsupported 節點)輸入,pest 生成解柝器
//! 與手寫解柝器的**非 Trivia token 流**(kind, span)逐項相等。
//! 側條件/非法輸入的邊界律:R₀ 產 Error 節點 ⟺ pest 解析失敗(同側)。
//!
//! 紀律:pest 只在 dev-dependencies(lib 依賴恆 0);本文法是 EBNF 的
//! 機器形式,不是第二解柝器載體 —— 語義面(`model.rs`)仍以 `r0_parse`
//! 為唯一輸入。

use cl0r0::gen::{gen_r0_semantic, Rng};
use cl0r0::r0::{r0_parse, R0Kind, R0Tree};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser, Debug)]
#[grammar = "r0.pest"]
struct R0Pest;

/// pest 規則名 → R0Kind 匿名 token 名(結構規則無 token,返回 None)。
fn pest_kind(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "fn_kw" => "FnKw",
        "struct_kw" => "StructKw",
        "let_kw" => "LetKw",
        "mut_kw" => "MutKw",
        "if_kw" => "IfKw",
        "else_kw" => "ElseKw",
        "while_kw" => "WhileKw",
        "loop_kw" => "LoopKw",
        "return_kw" => "ReturnKw",
        "true_kw" => "TrueKw",
        "false_kw" => "FalseKw",
        "amp_and" => "AndAnd",
        "amp_mut" => "AmpMut",
        "amp" => "Amp",
        "star" => "Star",
        "plus" => "Plus",
        "minus" => "Minus",
        "arrow" => "Arrow",
        "slash" => "Slash",
        "percent" => "Percent",
        "eq_eq" => "EqEq",
        "eq" => "Eq",
        "not_eq" => "NotEq",
        "not" => "Not",
        "le" => "Le",
        "lt" => "Lt",
        "ge" => "Ge",
        "gt" => "Gt",
        "dot" => "Dot",
        "semi" => "Semi",
        "colon" => "Colon",
        "comma" => "Comma",
        "lparen" => "LParen",
        "rparen" => "RParen",
        "lbrace" => "LBrace",
        "rbrace" => "RBrace",
        "lbrack" => "LBrack",
        "rbrack" => "RBrack",
        "or_or" => "OrOr",
        "IDENT" => "Ident",
        "NUMBER" => "Number",
        "RAWSTRING" => "RawString",
        _ => return None,
    })
}

/// R₀ token 流:全樹葉節點(非 Trivia)依源碼順序 (kind, start, end)。
fn r0_tokens(t: &R0Tree) -> Vec<(&'static str, u32, u32)> {
    let mut out = Vec::new();
    fn rec(t: &R0Tree, id: u32, out: &mut Vec<(&'static str, u32, u32)>) {
        let n = t.node(id);
        if n.children.is_empty() {
            if n.kind != R0Kind::Trivia {
                out.push((
                    match n.kind {
                        R0Kind::FnKw => "FnKw",
                        R0Kind::StructKw => "StructKw",
                        R0Kind::LetKw => "LetKw",
                        R0Kind::MutKw => "MutKw",
                        R0Kind::IfKw => "IfKw",
                        R0Kind::ElseKw => "ElseKw",
                        R0Kind::WhileKw => "WhileKw",
                        R0Kind::LoopKw => "LoopKw",
                        R0Kind::ReturnKw => "ReturnKw",
                        R0Kind::TrueKw => "TrueKw",
                        R0Kind::FalseKw => "FalseKw",
                        R0Kind::Amp => "Amp",
                        R0Kind::AmpMut => "AmpMut",
                        R0Kind::Star => "Star",
                        R0Kind::Plus => "Plus",
                        R0Kind::Minus => "Minus",
                        R0Kind::Eq => "Eq",
                        R0Kind::EqEq => "EqEq",
                        R0Kind::NotEq => "NotEq",
                        R0Kind::Lt => "Lt",
                        R0Kind::Le => "Le",
                        R0Kind::Gt => "Gt",
                        R0Kind::Ge => "Ge",
                        R0Kind::AndAnd => "AndAnd",
                        R0Kind::OrOr => "OrOr",
                        R0Kind::Not => "Not",
                        R0Kind::Dot => "Dot",
                        R0Kind::Semi => "Semi",
                        R0Kind::Colon => "Colon",
                        R0Kind::Comma => "Comma",
                        R0Kind::LParen => "LParen",
                        R0Kind::RParen => "RParen",
                        R0Kind::LBrace => "LBrace",
                        R0Kind::RBrace => "RBrace",
                        R0Kind::LBrack => "LBrack",
                        R0Kind::RBrack => "RBrack",
                        R0Kind::Arrow => "Arrow",
                        R0Kind::Slash => "Slash",
                        R0Kind::Percent => "Percent",
                        R0Kind::Ident => "Ident",
                        R0Kind::Number => "Number",
                        R0Kind::RawString => "RawString",
                        R0Kind::Bad => "Bad",
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
    let pairs = R0Pest::parse(Rule::program, src)?;
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

/// 樹內是否含側條件節點(Error / Unsupported)—— 含則不在 parity 面。
fn has_side_condition(t: &R0Tree) -> bool {
    t.nodes
        .iter()
        .any(|n| matches!(n.kind, R0Kind::Error | R0Kind::Unsupported))
}

/// 主律一:curated 語料(41 例)法律面 pest ≡ r0_parse。
#[test]
fn test_pest_r0_parity_corpus() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/curated");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    let mut n_parity = 0usize;
    let mut n_out_of_face = 0usize;
    for name in &files {
        if !name.ends_with(".rs") {
            continue;
        }
        let src = std::fs::read_to_string(dir.join(name)).unwrap();
        let t = r0_parse(&src).unwrap_or_else(|e| panic!("{name}: r0_parse 失敗: {e:?}"));
        if has_side_condition(&t) {
            n_out_of_face += 1;
            // 側條件/非法面:pest 必須同側失敗(邊界律在此順帶實測)。
            assert!(
                pest_tokens(&src).is_err(),
                "{name}: R₀ 有側條件節點但 pest 接受 —— 接受集偏大"
            );
            continue;
        }
        let (a, b) = (
            r0_tokens(&t),
            pest_tokens(&src).unwrap_or_else(|e| panic!("{name}: pest 拒絕法律 R₀: {e:?}")),
        );
        assert_eq!(a, b, "{name}: token 流分歧\nR₀:  {a:?}\npest: {b:?}");
        n_parity += 1;
    }
    assert_eq!(n_parity, 40, "法律面應為 40 例(41 語料 − 1 語法錯誤)");
    assert_eq!(n_out_of_face, 1, "側條件面應為 1 例(uncoded_syntax_error)");
}

/// 主律二:by-construction 生成樣本(500 輪)pest ≡ r0_parse。
#[test]
fn test_pest_r0_parity_fuzz() {
    let mut rng = Rng::new(0x0F42_2026_0004);
    for i in 0..500u64 {
        let s = gen_r0_semantic(&mut rng);
        let t = r0_parse(&s.src).unwrap_or_else(|e| panic!("round {i}: r0_parse: {e:?}"));
        assert!(
            !has_side_condition(&t),
            "round {i}: 生成樣本含側條件(生成器 BUG)"
        );
        let a = r0_tokens(&t);
        let b = pest_tokens(&s.src)
            .unwrap_or_else(|e| panic!("round {i}: pest 拒絕生成樣本: {e:?}\nsrc:\n{}", s.src));
        assert_eq!(a, b, "round {i}: token 流分歧\nsrc:\n{}", s.src);
    }
}

/// 邊界律:非法輸入 R₀ 產側條件節點(Error 或 Unsupported)⟺ pest 失敗
/// (接受集同側)。含 Bad token 的語句 R₀ 如實分類為 Unsupported(詞法側
/// 條件),不為 Error —— 兩者皆在 pest 法律面外。
#[test]
fn test_pest_boundary_illegal() {
    let cases: &[&str] = &[
        "let x = 1;",                     // 頂層語句(項面外)
        "fn main() { 1 }",                // 非塊表達式語句無 `;`
        "fn main() { let = 1; }",         // let 缺標識字
        "fn f(a,) {}",                    // 形參尾隨逗號(EBNF 不收)
        "struct S { , a: int }",          // 導前逗號
        "fn main() { a << b; }",          // `<<` = 詞法 Bad ⇒ Unsupported
        "fn main() { let x = 1; } stray", // 項層雜散
        "fn main() { let x = 1; 2 3; }",  // 表達式後非運算符
    ];
    for (i, src) in cases.iter().enumerate() {
        let t = r0_parse(src).unwrap_or_else(|e| panic!("case {i}: {e:?}"));
        assert!(
            t.has_error() || has_side_condition(&t),
            "case {i}: R₀ 對非法輸入未產側條件節點(測試集錯誤):\n{src}"
        );
        assert!(
            pest_tokens(src).is_err(),
            "case {i}: pest 接受 R₀ 判錯的輸入 —— 接受集偏大:\n{src}"
        );
    }
}
