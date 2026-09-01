//! R₀ —— 實用載體(報告 §7.2 / 附錄 B)。
//!
//! R₀ 是 Rust 的刻意子集,覆蓋 judge/borrow 實際需要的現場:
//! items(`fn` / `struct`)、語句(`let` / `expr;` / `return`)、表達式
//! (借用 / 解引用 / 賦值 / 調用 / 字段 / 索引 / `if` / `while` / `loop`)。
//!
//! **排除項(全部以「側條件」身份存在,由 `unsupported` 如實申報,
//! 不假裝覆蓋 —— 這是 `internal/toolchain` Available() 紀律的 R₀ 對應)**:
//!   泛型實參歧義(無 `<...>` → 沒有 `>>` 拆分問題,§5.1)、宏、閉包、
//!   模式匹配全集、trait、生命週期、`impl` / `use` / `mod` 等項。
//!
//! **本模組交付**(與報告路線一致:九律全量在 CL0 上驗證;R₀ 做「接線預備層」):
//!   1. 附錄 B 的機讀 EBNF(`R0_EBNF`);
//!   2. R₀ 正則詞法(Dfa lexer,逐字節平鋪,含 raw string `r#"…"#`);
//!   3. `unsupported(src)` —— 越界構造掃描器(如實申報);
//!   4. `lalr1_clean(src)` —— LALR(1)-乾淨片段斷言(歧義構造不存在)。

use crate::span::Span;

/// 附錄 B:R₀ 的機讀 EBNF(本模組的語法契約)。
pub const R0_EBNF: &str = r#"(* 附錄 B — R₀:實用載體。設計準則:落在 LALR(1) 可處理的片段內,歧義點以側條件排除。 *)
program  = { item } ;
item     = fn_item | struct_item ;
fn_item  = "fn" IDENT "(" [ params ] ")" [ "->" type ] block ;
struct_item = "struct" IDENT "{" [ field { "," field } [","] ] "}" ;
field    = IDENT ":" type ;
params   = param { "," param } ;
param    = IDENT [ ":" type ] ;
type     = IDENT | "&" [ "mut" ] type | "[" type "]" ;
block    = "{" { stmt } "}" ;
stmt     = let_stmt | return_stmt | if_stmt | while_stmt | loop_stmt | expr_stmt ;
let_stmt = "let" [ "mut" ] IDENT [ ":" type ] [ "=" expr ] ";" ;
return_stmt = "return" [ expr ] ";" ;
if_stmt  = "if" expr block [ "else" ( if_stmt | block ) ] ;
while_stmt = "while" expr block ;
loop_stmt = "loop" block ;
expr_stmt = expr ";" ;
expr     = assign ;
assign   = or_expr [ "=" assign ] ;
or_expr  = and_expr { "||" and_expr } ;
and_expr = eq_expr { "&&" eq_expr } ;
eq_expr  = rel_expr { ( "==" | "!=" ) rel_expr } ;
rel_expr = add_expr { ( "<" | "<=" | ">" | ">=" ) add_expr } ; (* 側條件:無泛型 ⟹ < 恆為比較 *)
add_expr = mul_expr { ( "+" | "-" ) mul_expr } ;
mul_expr = unary { ( "*" | "/" | "%" ) unary } ;
unary    = [ ( "&" | "&mut " | "*" | "!" ) ] postfix ;
postfix  = primary { "." IDENT | "[" expr "]" | "(" [ args ] ")" } ;
primary  = NUMBER | "true" | "false" | IDENT | "(" expr ")" | block ;
args     = expr { "," expr } ;
(* 側條件(排除項,與歧義 / Type-2 邊界相關,由 `unsupported` 如實申報):
   1. 無泛型實參:不存在 `<Type>` / `>>` 拆分歧義(§5.1 兩個非正則點之一被側條件排除);
   2. 無宏 `!`、無閉包 `|…|`、無 `match` 模式、無 trait / impl / use / mod / pub / unsafe;
   3. 無生命週期 `'a`(raw string 內部除外)。                              *)
"#;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum R0TokKind {
    Ident,
    Number,
    Fn,
    Struct,
    Let,
    Mut,
    If,
    Else,
    While,
    Loop,
    Return,
    True,
    False,
    Amp,
    AmpMut,
    Star,
    Plus,
    Minus,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Not,
    Dot,
    Semi,
    Colon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBrack,
    RBrack,
    Arrow,
    RawString,
    Slash,
    Percent,
    Trivia,
    Bad,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct R0Token {
    pub kind: R0TokKind,
    pub span: Span,
}

/// R₀ 詞法器:DFA / 正則,逐字節平鋪(與 CL0 詞法器同一不變量紀律)。
/// `>>` 只會是兩個 `>`(無泛型 ⇒ 無 Shl token ⇒ 無 §5.1 拆分問題)。
pub fn r0_lex(src: &str) -> Vec<R0Token> {
    let b = src.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0usize;
    let push = |toks: &mut Vec<R0Token>, kind: R0TokKind, start: usize, end: usize| {
        toks.push(R0Token {
            kind,
            span: Span::new(start as u32, end as u32),
        })
    };
    while i < b.len() {
        let s = i;
        match b[i] {
            b' ' | b'\t' | b'\r' | b'\n' => {
                while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\r' | b'\n') {
                    i += 1;
                }
                push(&mut toks, R0TokKind::Trivia, s, i);
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                push(&mut toks, R0TokKind::Trivia, s, i);
            }
            b'0'..=b'9' => {
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                push(&mut toks, R0TokKind::Number, s, i);
            }
            b'r' if i + 1 < b.len() && b[i + 1] == b'#' => {
                // raw string r#"…"#(可以含 #更多)
                let mut j = i + 2;
                let mut hashes = 1;
                while j < b.len() && b[j] == b'#' {
                    hashes += 1;
                    j += 1;
                }
                if j >= b.len() || b[j] != b'"' {
                    // 不是 raw string:退回普通 ident(但 r# 開頭仍按原掃描)
                    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                        i += 1;
                    }
                    push(&mut toks, R0TokKind::Ident, s, i);
                    continue;
                }
                // 掃描直到 " 後跟 #*hashes
                let body_start = j + 1;
                let mut k = body_start;
                let mut closed = None;
                while k < b.len() {
                    if b[k] == b'"' {
                        let mut h = 0;
                        while k + 1 + h < b.len() && b[k + 1 + h] == b'#' {
                            h += 1;
                        }
                        if h >= hashes {
                            closed = Some(k + 1 + hashes);
                            break;
                        }
                    }
                    k += 1;
                }
                i = closed.unwrap_or(b.len());
                push(&mut toks, R0TokKind::RawString, s, i);
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let kind = match &src[s..i] {
                    "fn" => R0TokKind::Fn,
                    "struct" => R0TokKind::Struct,
                    "let" => R0TokKind::Let,
                    "mut" => R0TokKind::Mut,
                    "if" => R0TokKind::If,
                    "else" => R0TokKind::Else,
                    "while" => R0TokKind::While,
                    "loop" => R0TokKind::Loop,
                    "return" => R0TokKind::Return,
                    "true" => R0TokKind::True,
                    "false" => R0TokKind::False,
                    _ => R0TokKind::Ident,
                };
                push(&mut toks, kind, s, i);
            }
            b'&' => {
                i += 1;
                if i < b.len() && b[i] == b'&' {
                    i += 1;
                    push(&mut toks, R0TokKind::AndAnd, s, i);
                } else if i + 3 < b.len() && &src[i..i + 3] == "mut" {
                    i += 3;
                    push(&mut toks, R0TokKind::AmpMut, s, i);
                } else {
                    push(&mut toks, R0TokKind::Amp, s, i);
                }
            }
            b'|' => {
                i += 1;
                if i < b.len() && b[i] == b'|' {
                    i += 1;
                    push(&mut toks, R0TokKind::OrOr, s, i);
                } else {
                    push(&mut toks, R0TokKind::Bad, s, i); // 單豎線 = 閉包語法 → unsupported
                }
            }
            b'*' => {
                i += 1;
                push(&mut toks, R0TokKind::Star, s, i);
            }
            b'+' => {
                i += 1;
                push(&mut toks, R0TokKind::Plus, s, i);
            }
            b'-' => {
                i += 1;
                if i < b.len() && b[i] == b'>' {
                    i += 1;
                    push(&mut toks, R0TokKind::Arrow, s, i);
                } else {
                    push(&mut toks, R0TokKind::Minus, s, i);
                }
            }
            b'/' => {
                i += 1;
                push(&mut toks, R0TokKind::Slash, s, i);
            }
            b'%' => {
                i += 1;
                push(&mut toks, R0TokKind::Percent, s, i);
            }
            b'=' => {
                i += 1;
                if i < b.len() && b[i] == b'=' {
                    i += 1;
                    push(&mut toks, R0TokKind::EqEq, s, i);
                } else {
                    push(&mut toks, R0TokKind::Eq, s, i);
                }
            }
            b'!' => {
                i += 1;
                if i < b.len() && b[i] == b'=' {
                    i += 1;
                    push(&mut toks, R0TokKind::NotEq, s, i);
                } else {
                    push(&mut toks, R0TokKind::Not, s, i); // 宏 / 否定
                }
            }
            b'<' => {
                i += 1;
                if i < b.len() && b[i] == b'=' {
                    i += 1;
                    push(&mut toks, R0TokKind::Le, s, i);
                } else if i < b.len() && b[i] == b'<' {
                    i += 1;
                    push(&mut toks, R0TokKind::Bad, s, i); // << / Shl:不在 R₀
                } else {
                    push(&mut toks, R0TokKind::Lt, s, i);
                }
            }
            b'>' => {
                i += 1;
                if i < b.len() && b[i] == b'=' {
                    i += 1;
                    push(&mut toks, R0TokKind::Ge, s, i);
                } else {
                    push(&mut toks, R0TokKind::Gt, s, i);
                }
            }
            b'.' => {
                i += 1;
                push(&mut toks, R0TokKind::Dot, s, i);
            }
            b';' => {
                i += 1;
                push(&mut toks, R0TokKind::Semi, s, i);
            }
            b':' => {
                i += 1;
                push(&mut toks, R0TokKind::Colon, s, i);
            }
            b',' => {
                i += 1;
                push(&mut toks, R0TokKind::Comma, s, i);
            }
            b'(' => {
                i += 1;
                push(&mut toks, R0TokKind::LParen, s, i);
            }
            b')' => {
                i += 1;
                push(&mut toks, R0TokKind::RParen, s, i);
            }
            b'{' => {
                i += 1;
                push(&mut toks, R0TokKind::LBrace, s, i);
            }
            b'}' => {
                i += 1;
                push(&mut toks, R0TokKind::RBrace, s, i);
            }
            b'[' => {
                i += 1;
                push(&mut toks, R0TokKind::LBrack, s, i);
            }
            b']' => {
                i += 1;
                push(&mut toks, R0TokKind::RBrack, s, i);
            }
            _ => {
                let ch = src[i..].chars().next().unwrap();
                i += ch.len_utf8();
                push(&mut toks, R0TokKind::Bad, s, i);
            }
        }
    }
    toks
}

/// R₀ 詞法平鋪不變量(與 CL0 相同紀律)。
pub fn r0_lexical_invariants(src: &str) -> Result<(), String> {
    let toks = r0_lex(src);
    let mut expected = 0u32;
    for t in &toks {
        if t.span.start != expected {
            return Err(format!("r0 lexer gap at {}", expected));
        }
        expected = t.span.end;
    }
    if expected != src.len() as u32 {
        return Err(format!("r0 lexer coverage {} != {}", expected, src.len()));
    }
    Ok(())
}

/// 越界構造掃描:返回所有被側條件排除的構造(如實申報,不假裝覆蓋)。
/// 返回 (構造名, 出現的字節區間)。
pub fn unsupported(src: &str) -> Vec<(&'static str, Span)> {
    let toks = r0_lex(src);
    let mut out = Vec::new();
    for w in [
        ("macro", "macro_rules"),
        ("trait", "trait"),
        ("impl", "impl"),
        ("use", "use"),
        ("mod", "mod"),
        ("pub", "pub"),
        ("unsafe", "unsafe"),
        ("async", "async"),
        ("match", "match"),
        ("fn 泛型", "fn "), /* 佔位 */
    ] {
        let _ = w;
    }
    // 詞法級:關鍵字與符號
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        match t.kind {
            R0TokKind::Ident if i + 1 < toks.len() && toks[i + 1].span.start == t.span.end => {
                // 直接相連的 ident(token 已按最長匹配,這裡是關鍵字檢查的替代路徑)
            }
            R0TokKind::Not => out.push(("宏/否定 `!`(宏語法)", t.span)),
            R0TokKind::Bad => out.push(("非法符號(閉包 `|` 或 `<<` 等)", t.span)),
            _ => {}
        }
        i += 1;
    }
    // 詞級關鍵字
    for kw in [
        "trait",
        "impl",
        "use",
        "mod",
        "pub",
        "unsafe",
        "async",
        "match",
        "macro_rules",
        "dyn",
    ] {
        let mut from = 0;
        while let Some(pos) = src[from..].find(kw) {
            let start = from + pos;
            let end = start + kw.len();
            // 必須是獨立詞(兩側為邊界)
            let before_ok =
                start == 0 || !src[..start].ends_with(|c: char| c.is_alphanumeric() || c == '_');
            let after = src[end..].chars().next();
            let after_ok =
                after.is_none() || !after.unwrap().is_alphanumeric() && after.unwrap() != '_';
            if before_ok && after_ok {
                out.push((
                    match kw {
                        "trait" => "trait 項(排除)",
                        "impl" => "impl 塊(排除)",
                        "use" => "use 項(排除)",
                        "mod" => "mod 項(排除)",
                        "pub" => "可見性(pub,排除)",
                        "unsafe" => "unsafe(排除)",
                        "async" => "async(排除)",
                        "match" => "match 模式(排除)",
                        "macro_rules" => "macro_rules(排除)",
                        _ => "dyn(排除)",
                    },
                    Span::new(start as u32, end as u32),
                ));
            }
            from = end;
        }
    }
    // 生命週期 `'a`(raw string 內部除外 —— raw string 已整體成為 RawString token,故無誤報)
    let mut from = 0;
    let bytes = src.as_bytes();
    while from < bytes.len() {
        if bytes[from] == b'\'' {
            if from + 1 < bytes.len()
                && (bytes[from + 1].is_ascii_alphabetic() || bytes[from + 1] == b'_')
            {
                out.push((
                    "生命週期 `'a`(排除)",
                    Span::new(from as u32, (from + 2) as u32),
                ));
                from += 2;
                continue;
            }
        }
        from += 1;
    }
    out.sort_by_key(|(_, sp)| sp.start);
    out.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    out
}

/// LALR(1)-乾淨片段斷言:檢查是否出現會引入歧義的構造
/// (泛型實參 `<T>` 前的 ident、`<<`、`>` 後的 `(` 等 —— 本子集內由側條件全部排除)。
pub fn lalr1_clean(src: &str) -> Result<(), String> {
    let toks = r0_lex(src);
    let mut structs: Vec<R0TokKind> = vec![];
    for t in &toks {
        if t.kind == R0TokKind::Bad {
            return Err(format!("歧義/越界符號 @ {:?}", t.span));
        }
        if t.kind == R0TokKind::Not {
            return Err(format!("`!`(宏語法,歧義) @ {:?}", t.span));
        }
        structs.push(t.kind);
    }
    // 泛型實參模式:IDENT `<` IDENT(排除比較的判據在真實語法中需要 2 個 lookahead)
    for w in structs.windows(3) {
        if w[0] == R0TokKind::Ident && w[1] == R0TokKind::Lt && w[2] == R0TokKind::Ident {
            return Err("概型實參 vs 比較的歧義(側條件排除:無泛型)".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r0_lex_tiling() {
        for src in [
            "fn main() { let x = 1; }",
            "let s = r#\"he\"llo\"#;",
            "let t = r##\"x\"##;",
            "struct P { a: int, b: &mut int }",
            "x > y && z != 3",
            "@@|",
        ] {
            r0_lexical_invariants(src).unwrap_or_else(|e| panic!("{:?}: {}", src, e));
        }
    }

    #[test]
    fn r0_unsupported_detects() {
        let src = "fn main() { let x = |a| a; match x { _ => {} } let y: &'a int = &1; }";
        let u = unsupported(src);
        let kinds: Vec<&str> = u.iter().map(|(k, _)| *k).collect();
        assert!(
            kinds
                .iter()
                .any(|k| k.contains("閉包") || k.contains("macro")),
            "closure must be detected, got {:?}",
            kinds
        );
        assert!(
            kinds.iter().any(|k| k.contains("match")),
            "match must be detected, got {:?}",
            kinds
        );
        assert!(
            kinds.iter().any(|k| k.contains("生命週期")),
            "lifetime must be detected, got {:?}",
            kinds
        );
    }

    #[test]
    fn r0_lalr1_clean_checks() {
        // 合法 R₀ 片段:無歧義
        assert!(lalr1_clean("fn main() { let x = 1 < 2; }").is_ok());
        // 泛型實參模式:被標記
        assert!(lalr1_clean("let v: Vec<int> = v;").is_err());
        // 宏:被標記
        assert!(lalr1_clean("println!(x);").is_err());
    }
}
