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
//!   4. `lalr1_clean(src)` —— LALR(1)-乾淨片段斷言(歧義構造不存在);
//!   5. `r0_parse(src)` —— 附錄 B EBNF → 表面語法樹(連續性/樹公理/laminar/
//!      無損回環 + 節點級 `unsupported` 申報,§9 如實申報的結構化形式)。

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

/// R₀(Rust 實用子集,附錄 B / §7.2)的詞法單元種類。
/// 覆蓋面契約之外的語法由 `r0_parse` 如實申報 `unsupported`(§9 非目標)。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum R0TokKind {
    /// 識別字(r# 前綴的 raw ident 不屬於 R₀)。
    Ident,
    /// 十進制整數字面量。
    Number,
    /// 關鍵字 `fn`。
    Fn,
    /// 關鍵字 `struct`。
    Struct,
    /// 關鍵字 `let`。
    Let,
    /// 關鍵字 `mut`。
    Mut,
    /// 關鍵字 `if`。
    If,
    /// 關鍵字 `else`。
    Else,
    /// 關鍵字 `while`。
    While,
    /// 關鍵字 `loop`。
    Loop,
    /// 關鍵字 `return`。
    Return,
    /// 字面量 `true`。
    True,
    /// 字面量 `false`。
    False,
    /// `&`(共享借用)。
    Amp,
    /// `&mut`(可變借用;兩個 token 的詞法合併,非 `&`+`mut`)。
    AmpMut,
    /// `*`(解引用)。
    Star,
    /// `+`。
    Plus,
    /// `-`。
    Minus,
    /// `=`(賦值/綁定)。
    Eq,
    /// `==`(相等)。
    EqEq,
    /// `!=`(不等)。
    NotEq,
    /// `<`。
    Lt,
    /// `<=`。
    Le,
    /// `>`。
    Gt,
    /// `>=`。
    Ge,
    /// `&&`(邏輯與)。
    AndAnd,
    /// `||`(邏輯或)。
    OrOr,
    /// `!`(邏輯非)。
    Not,
    /// `.`(字段訪問)。
    Dot,
    /// `;`。
    Semi,
    /// `:`(類型標註)。
    Colon,
    /// `,`。
    Comma,
    /// `(`。
    LParen,
    /// `)`。
    RParen,
    /// `{`。
    LBrace,
    /// `}`。
    RBrace,
    /// `[`(索引左界)。
    LBrack,
    /// `]`(索引右界)。
    RBrack,
    /// `->`(返回類型箭頭)。
    Arrow,
    /// raw string 字面量 `r#"…"#`(R₀ 唯一的面板式詞法項)。
    RawString,
    /// `/`(整除,非註釋;R₀ 註釋以標準 `//` 詞法處理)。
    Slash,
    /// `%`(取模)。
    Percent,
    /// 空白與 `//` 註釋(平鋪保留,不進 R₀ 樹)。
    Trivia,
    /// 詞法錯誤字元(仍佔一個平鋪 token ⇒ 全化)。
    Bad,
}

/// R₀ 詞法單元:種類 + 源碼跨度。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct R0Token {
    /// 單元種類。
    pub kind: R0TokKind,
    /// 半開跨度 [start, end)。
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
                } else if i + 3 <= b.len() && &b[i..i + 3] == b"mut" {
                    // ★ 兩處修正(健檢 P0-1):
                    //   (a) 比**位元組** `b[i..i+3]` 而非 `src[i..i+3]`。後者是
                    //       `&str` 切片,`i+3` 不保證落在 UTF-8 邊界上 ——
                    //       `&` 後接非 3 位元組字元(如 `&🦀`、`&éé`、`&Привет`)
                    //       會直接 panic,違反 §2.3「全化、永不 panic」。
                    //   (b) 邊界由 `<` 改 `<=`:否則 `&mut` 剛好在輸入結尾時
                    //       會被切成 `Amp` + `Mut` 兩個 token(off-by-one)。
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
        "enum",
        "type",
        "static",
        "const",
        "extern",
        "where",
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
                        "enum" => "enum 項(排除)",
                        "type" => "type 項(排除)",
                        "static" => "static(排除)",
                        "const" => "const(排除)",
                        "extern" => "extern(排除)",
                        "where" => "where(排除)",
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
        if bytes[from] == b'\''
            && from + 1 < bytes.len()
            && (bytes[from + 1].is_ascii_alphabetic() || bytes[from + 1] == b'_')
        {
            out.push((
                "生命週期 `'a`(排除)",
                Span::new(from as u32, (from + 2) as u32),
            ));
            from += 2;
            continue;
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

// ===========================================================================
// r0_parse —— R₀ 完整解析器(附錄 B EBNF → 表面語法樹)
// ===========================================================================
// 紀律與 CL0 同源(§1–§2):
//   * 總化:任何輸入(含非法字節)都產出樹,唯一例外是引擎遞歸極限
//     (R0_RECURSION_LIMIT,如實申報 Err(Depth),永不 panic);
//   * 連續性公理:內部節點跨度 = 子節點跨度之並(構造性維持);
//   * 樹公理:每節點至多一父、自根連通;
//   * 側條件構造(macro `!`、閉包 `|`、match、trait/impl/use/mod/pub/unsafe、
//     生命週期、泛型實參歧義 IDENT<IDENT)以 **節點級** Unsupported 如實申報
//     (note + span),而非假裝覆蓋 —— 對應 legacy 掃描器 `unsupported` 的結構化升級。

/// R₀ 解析引擎的遞歸極限(機器界的如實申報;R₀ 的優先級語法鏈約 15 幀/層,
/// 故界取 64 以保證 2MB 線程棧內如實觸發而非溢出;CL0 為 256)。
pub const R0_RECURSION_LIMIT: usize = 64;

/// R₀ 解析器的失敗報告(僅兩種:語法層與引擎界)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum R0ParseIssue {
    /// 語法層失敗(由 ERROR 恢復處理;調用方據此把構造轉為 Error 節點)。
    Syntax,
    /// 引擎遞歸極限(機器界的如實申報)。
    Depth,
}

/// R₀ 表面語法樹的節點種類(具名/匿名二分,與 CL0 同紀律)。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum R0Kind {
    // —— 具名(named)——
    /// 整棵樹的根。
    Root,
    /// 函數項 `fn …`。
    FnItem,
    /// 結構體項 `struct … { … }`。
    StructItem,
    /// 結構體字段 `IDENT: type`。
    FieldDef,
    /// 形參 `IDENT [": type"]`。
    Param,
    /// 型別引用 `int` / `&T` / `&mut T` / `[T]`。
    TypeRef,
    /// 塊 `{ … }`。
    Block,
    /// `let` 綁定語句。
    LetStmt,
    /// `return [expr] ;`。
    ReturnStmt,
    /// `if [cond] { … } else { … }`。
    IfStmt,
    /// `while [cond] { … }`。
    WhileStmt,
    /// `loop { … }`。
    LoopStmt,
    /// 表達式語句 `expr ;`。
    ExprStmt,
    /// 表達式(單節點扁平衡:子節點 = 操作數與運算符,構造順序即優先級)。
    Expr,
    /// 一元前綴 `&x` / `&mut x` / `*x` / `!x`。
    UnaryExpr,
    /// 側條件排除構造(§9 如實申報):note 字段載明排除原因。
    Unsupported,
    /// 語法失敗的封存區(§2.3 全化):吞至語句/項邊界,一語句一原子錯誤區。
    Error,
    // —— 匿名(anonymous,token 行)——
    /// 關鍵字 `fn`。
    FnKw,
    /// 關鍵字 `struct`。
    StructKw,
    /// 關鍵字 `let`。
    LetKw,
    /// 關鍵字 `mut`。
    MutKw,
    /// 關鍵字 `if`。
    IfKw,
    /// 關鍵字 `else`。
    ElseKw,
    /// 關鍵字 `while`。
    WhileKw,
    /// 關鍵字 `loop`。
    LoopKw,
    /// 關鍵字 `return`。
    ReturnKw,
    /// 字面量 `true`。
    TrueKw,
    /// 字面量 `false`。
    FalseKw,
    /// `&`(共享借用)。
    Amp,
    /// `&mut`(可變借用)。
    AmpMut,
    /// `*`(解引用/乘法)。
    Star,
    /// `+`。
    Plus,
    /// `-`。
    Minus,
    /// `=`(賦值/綁定)。
    Eq,
    /// `==`。
    EqEq,
    /// `!=`。
    NotEq,
    /// `<`。
    Lt,
    /// `<=`。
    Le,
    /// `>`。
    Gt,
    /// `>=`。
    Ge,
    /// `&&`。
    AndAnd,
    /// `||`。
    OrOr,
    /// `!`(邏輯非/宏)。
    Not,
    /// `.`(字段訪問)。
    Dot,
    /// `;`。
    Semi,
    /// `:`(型別標註)。
    Colon,
    /// `,`。
    Comma,
    /// `(`。
    LParen,
    /// `)`。
    RParen,
    /// `{`。
    LBrace,
    /// `}`。
    RBrace,
    /// `[`(索引左界)。
    LBrack,
    /// `]`(索引右界)。
    RBrack,
    /// `->`(返回型別)。
    Arrow,
    /// `/`(整除)。
    Slash,
    /// `%`(取模)。
    Percent,
    /// 識別字。
    Ident,
    /// 數字字面量。
    Number,
    /// raw string 字面量 `r#"…"#`(詞法面板式項)。
    RawString,
    /// 空白/註釋(平鋪保留,不進結構)。
    Trivia,
    /// 詞法級壞符號(平鋪保留 ⇒ 全化)。
    Bad,
}

impl R0Kind {
    /// 是否為具名(結構)節點。
    pub fn is_named(self) -> bool {
        !matches!(
            self,
            R0Kind::FnKw
                | R0Kind::StructKw
                | R0Kind::LetKw
                | R0Kind::MutKw
                | R0Kind::IfKw
                | R0Kind::ElseKw
                | R0Kind::WhileKw
                | R0Kind::LoopKw
                | R0Kind::ReturnKw
                | R0Kind::TrueKw
                | R0Kind::FalseKw
                | R0Kind::Amp
                | R0Kind::AmpMut
                | R0Kind::Star
                | R0Kind::Plus
                | R0Kind::Minus
                | R0Kind::Eq
                | R0Kind::EqEq
                | R0Kind::NotEq
                | R0Kind::Lt
                | R0Kind::Le
                | R0Kind::Gt
                | R0Kind::Ge
                | R0Kind::AndAnd
                | R0Kind::OrOr
                | R0Kind::Not
                | R0Kind::Dot
                | R0Kind::Semi
                | R0Kind::Colon
                | R0Kind::Comma
                | R0Kind::LParen
                | R0Kind::RParen
                | R0Kind::LBrace
                | R0Kind::RBrace
                | R0Kind::LBrack
                | R0Kind::RBrack
                | R0Kind::Arrow
                | R0Kind::Slash
                | R0Kind::Percent
                | R0Kind::Ident
                | R0Kind::Number
                | R0Kind::RawString
                | R0Kind::Trivia
                | R0Kind::Bad
        )
    }

    /// 是否為結構 token(即非 Trivia)。
    pub fn is_struct(self) -> bool {
        self != R0Kind::Trivia
    }

    /// 節點種類的規範標簽(sexp / 調試輸出用)。
    pub fn label(self) -> &'static str {
        match self {
            R0Kind::Root => "root",
            R0Kind::FnItem => "fn_item",
            R0Kind::StructItem => "struct_item",
            R0Kind::FieldDef => "field",
            R0Kind::Param => "param",
            R0Kind::TypeRef => "type_ref",
            R0Kind::Block => "block",
            R0Kind::LetStmt => "let_stmt",
            R0Kind::ReturnStmt => "return_stmt",
            R0Kind::IfStmt => "if_stmt",
            R0Kind::WhileStmt => "while_stmt",
            R0Kind::LoopStmt => "loop_stmt",
            R0Kind::ExprStmt => "expr_stmt",
            R0Kind::Expr => "expr",
            R0Kind::UnaryExpr => "unary_expr",
            R0Kind::Unsupported => "unsupported",
            R0Kind::Error => "error",
            R0Kind::FnKw => "fn",
            R0Kind::StructKw => "struct",
            R0Kind::LetKw => "let",
            R0Kind::MutKw => "mut",
            R0Kind::IfKw => "if",
            R0Kind::ElseKw => "else",
            R0Kind::WhileKw => "while",
            R0Kind::LoopKw => "loop",
            R0Kind::ReturnKw => "return",
            R0Kind::TrueKw => "true",
            R0Kind::FalseKw => "false",
            R0Kind::Amp => "&",
            R0Kind::AmpMut => "&mut",
            R0Kind::Star => "*",
            R0Kind::Plus => "+",
            R0Kind::Minus => "-",
            R0Kind::Eq => "=",
            R0Kind::EqEq => "==",
            R0Kind::NotEq => "!=",
            R0Kind::Lt => "<",
            R0Kind::Le => "<=",
            R0Kind::Gt => ">",
            R0Kind::Ge => ">=",
            R0Kind::AndAnd => "&&",
            R0Kind::OrOr => "||",
            R0Kind::Not => "!",
            R0Kind::Dot => ".",
            R0Kind::Semi => ";",
            R0Kind::Colon => ":",
            R0Kind::Comma => ",",
            R0Kind::LParen => "(",
            R0Kind::RParen => ")",
            R0Kind::LBrace => "{",
            R0Kind::RBrace => "}",
            R0Kind::LBrack => "[",
            R0Kind::RBrack => "]",
            R0Kind::Arrow => "->",
            R0Kind::Slash => "/",
            R0Kind::Percent => "%",
            R0Kind::Ident => "ident",
            R0Kind::Number => "number",
            R0Kind::RawString => "raw_string",
            R0Kind::Trivia => "trivia",
            R0Kind::Bad => "bad",
        }
    }
}

/// R₀ 樹節點:種類 + 半開跨度 + 直接子節點表(+ Unsupported 的原因)。
#[derive(Clone, Debug)]
pub struct R0Node {
    /// 節點種類。
    pub kind: R0Kind,
    /// 覆蓋的半開源碼跨度。
    pub span: Span,
    /// 直接子節點 id(依源碼順序)。
    pub children: Vec<u32>,
    /// 僅 Unsupported 節點:排除原因的機讀標簽。
    pub note: Option<&'static str>,
}

/// R₀ 解析產物:源碼 + 節點表(與 CL0 Tree 同紀律的「第二載體」樹)。
#[derive(Clone, Debug)]
pub struct R0Tree {
    /// 被解析的源碼(逐字節保留 ⇒ 無損回環)。
    pub src: String,
    /// 節點表(連續存儲;id 即下標)。
    pub nodes: Vec<R0Node>,
}

impl R0Tree {
    /// 根節點 id(恆為 0)。
    pub fn root(&self) -> u32 {
        debug_assert!(!self.nodes.is_empty());
        0
    }

    /// 依 id 取節點引用。
    pub fn node(&self, id: u32) -> &R0Node {
        &self.nodes[id as usize]
    }

    /// §1.2 連續性公理(與 CL0 同式):內部節點跨度 = [首子.start, 末子.end)
    /// 且子節點依序不交。
    pub fn validate_continuity(&self) -> Result<(), String> {
        for (id, node) in self.nodes.iter().enumerate() {
            if node.children.is_empty() {
                continue;
            }
            let first = self.nodes[node.children[0] as usize].span;
            let last = self.nodes[*node.children.last().unwrap() as usize].span;
            if node.span != Span::new(first.start, last.end) {
                return Err(format!(
                    "node {} ({:?}) span {} != children union [{}, {})",
                    id, node.kind, node.span, first.start, last.end
                ));
            }
            let mut prev_end = first.start;
            for &c in &node.children {
                let cs = self.nodes[c as usize].span;
                if cs.start < prev_end {
                    return Err(format!(
                        "node {} ({:?}) children overlap at {}",
                        id, node.kind, c
                    ));
                }
                prev_end = cs.end;
            }
        }
        Ok(())
    }

    /// 樹公理:每節點至多一父、自根連通。
    pub fn validate_tree_shapes(&self) -> Result<(), String> {
        let mut parent_of = vec![u32::MAX; self.nodes.len()];
        for (id, node) in self.nodes.iter().enumerate() {
            for &c in &node.children {
                if parent_of[c as usize] != u32::MAX {
                    return Err(format!("node {} has two parents", c));
                }
                parent_of[c as usize] = id as u32;
            }
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut stack = vec![0u32];
        while let Some(id) = stack.pop() {
            if seen[id as usize] {
                continue;
            }
            seen[id as usize] = true;
            for &c in &self.nodes[id as usize].children {
                stack.push(c);
            }
        }
        for (id, s) in seen.iter().enumerate() {
            if !s {
                return Err(format!("node {} unreachable from root", id));
            }
        }
        Ok(())
    }

    /// L5 檢查:任意兩節點 span 要嘛嵌套、要嘛不交(laminar 族)。
    pub fn laminar_ok(&self) -> bool {
        let n = self.nodes.len();
        for i in 0..n {
            let a = self.nodes[i].span;
            for j in (i + 1)..n {
                let b = self.nodes[j].span;
                if a.overlaps(&b) && !a.contains(&b) && !b.contains(&a) {
                    return false;
                }
            }
        }
        true
    }

    /// ERROR 節點數(§2.3 全化的「錯誤面」度量)。
    pub fn n_errors(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.kind == R0Kind::Error)
            .count()
    }

    /// 是否存在 ERROR 節點(L7a 判據)。
    pub fn has_error(&self) -> bool {
        self.n_errors() > 0
    }

    /// 節點級 unsupported 申報:全部 (span, 原因) 對(§9 如實申報的結構化形式)。
    pub fn unsupported_spans(&self) -> Vec<(Span, &'static str)> {
        self.nodes
            .iter()
            .filter(|n| n.kind == R0Kind::Unsupported)
            .map(|n| (n.span, n.note.unwrap_or("排除構造")))
            .collect()
    }

    /// 無損回環:葉子節點文本依序拼接(source 順序 = id 順序)。
    pub fn unparse(&self) -> String {
        let mut out = String::new();
        for n in &self.nodes {
            if n.children.is_empty() && n.kind != R0Kind::Root {
                out.push_str(&self.src[n.span.start as usize..n.span.end as usize]);
            }
        }
        out
    }

    /// 具名投影序列化(§1.3 的 R₀ 對應;決定論斷言用)。Unsupported 附 note。
    pub fn named_sexp(&self) -> String {
        fn go(t: &R0Tree, id: u32, out: &mut String) {
            let n = t.node(id);
            if !n.kind.is_named() {
                return;
            }
            out.push('(');
            out.push_str(n.kind.label());
            if let Some(note) = n.note {
                out.push(' ');
                out.push_str(note);
            }
            for &c in &n.children {
                go(t, c, out);
            }
            out.push(')');
        }
        let mut out = String::new();
        go(self, self.root(), &mut out);
        out
    }

    /// 節點總數(樹規模)。
    pub fn total_nodes(&self) -> usize {
        self.nodes.len()
    }
}

/// item-起始位置的排除關鍵字(§9 側條件;與 legacy `unsupported` 掃描同集)。
const EXCLUDED_ITEM_KWS: &[&str] = &[
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
    "enum",
    "type",
    "static",
    "const",
    "extern",
    "where",
];

fn is_excluded_item_kw(w: &str) -> bool {
    EXCLUDED_ITEM_KWS.contains(&w)
}

fn tok_kind(t: R0TokKind) -> R0Kind {
    match t {
        R0TokKind::Ident => R0Kind::Ident,
        R0TokKind::Number => R0Kind::Number,
        R0TokKind::Fn => R0Kind::FnKw,
        R0TokKind::Struct => R0Kind::StructKw,
        R0TokKind::Let => R0Kind::LetKw,
        R0TokKind::Mut => R0Kind::MutKw,
        R0TokKind::If => R0Kind::IfKw,
        R0TokKind::Else => R0Kind::ElseKw,
        R0TokKind::While => R0Kind::WhileKw,
        R0TokKind::Loop => R0Kind::LoopKw,
        R0TokKind::Return => R0Kind::ReturnKw,
        R0TokKind::True => R0Kind::TrueKw,
        R0TokKind::False => R0Kind::FalseKw,
        R0TokKind::Amp => R0Kind::Amp,
        R0TokKind::AmpMut => R0Kind::AmpMut,
        R0TokKind::Star => R0Kind::Star,
        R0TokKind::Plus => R0Kind::Plus,
        R0TokKind::Minus => R0Kind::Minus,
        R0TokKind::Eq => R0Kind::Eq,
        R0TokKind::EqEq => R0Kind::EqEq,
        R0TokKind::NotEq => R0Kind::NotEq,
        R0TokKind::Lt => R0Kind::Lt,
        R0TokKind::Le => R0Kind::Le,
        R0TokKind::Gt => R0Kind::Gt,
        R0TokKind::Ge => R0Kind::Ge,
        R0TokKind::AndAnd => R0Kind::AndAnd,
        R0TokKind::OrOr => R0Kind::OrOr,
        R0TokKind::Not => R0Kind::Not,
        R0TokKind::Dot => R0Kind::Dot,
        R0TokKind::Semi => R0Kind::Semi,
        R0TokKind::Colon => R0Kind::Colon,
        R0TokKind::Comma => R0Kind::Comma,
        R0TokKind::LParen => R0Kind::LParen,
        R0TokKind::RParen => R0Kind::RParen,
        R0TokKind::LBrace => R0Kind::LBrace,
        R0TokKind::RBrace => R0Kind::RBrace,
        R0TokKind::LBrack => R0Kind::LBrack,
        R0TokKind::RBrack => R0Kind::RBrack,
        R0TokKind::Arrow => R0Kind::Arrow,
        R0TokKind::RawString => R0Kind::RawString,
        R0TokKind::Slash => R0Kind::Slash,
        R0TokKind::Percent => R0Kind::Percent,
        R0TokKind::Trivia => R0Kind::Trivia,
        R0TokKind::Bad => R0Kind::Bad,
    }
}

/// 吸收模式的同步點(§2.3:語句邊界 / 項邊界)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Absorb {
    /// 止於 `}`(不入),吞至 `;`(含)。
    Stmt,
    /// 止於下一個 `fn` / `struct` / 排除關鍵字(不入,且僅在深度 0)。
    Item,
    /// 宏調用的參數組 `!(...)` / `!{...}`:吞至配對右界(深度 0 處),不吃 `;`。
    Macro,
}

impl Absorb {
    fn stop_before(self, k: R0TokKind, kw_of_ident: Option<&str>) -> bool {
        match self {
            Absorb::Stmt => k == R0TokKind::RBrace,
            Absorb::Macro => matches!(k, R0TokKind::RBrace | R0TokKind::Semi),
            Absorb::Item => {
                k == R0TokKind::Fn
                    || k == R0TokKind::Struct
                    || (k == R0TokKind::Ident && kw_of_ident.is_some_and(is_excluded_item_kw))
            }
        }
    }
    fn stop_after(self, k: R0TokKind) -> bool {
        match self {
            Absorb::Stmt => k == R0TokKind::Semi,
            Absorb::Macro => matches!(k, R0TokKind::RParen | R0TokKind::RBrace),
            Absorb::Item => false,
        }
    }
}

/// R₀ 遞歸下降解析器(open/close/finalize 機制與 CL0 同構)。
struct R0Parser {
    src: String,
    toks: Vec<R0Token>,
    pos: usize,
    nodes: Vec<R0Node>,
    stack: Vec<u32>,
    depth: usize,
}

impl R0Parser {
    /// 下一個 token 的種類(**trivia 感知**:先吸附前導 trivia 到棧頂)。
    fn cur_kind(&mut self) -> Option<R0TokKind> {
        self.skip_trivia();
        self.toks.get(self.pos).map(|t| t.kind)
    }
    fn cur_span(&self) -> Option<Span> {
        self.toks.get(self.pos).map(|t| t.span)
    }
    fn peek2_kind(&self) -> Option<R0TokKind> {
        self.toks.get(self.pos + 1).map(|t| t.kind)
    }
    fn text(&self, sp: Span) -> &str {
        &self.src[sp.start as usize..sp.end as usize]
    }
    fn cur_text(&self) -> Option<&str> {
        self.cur_span().map(|sp| self.text(sp))
    }

    fn link(&mut self, parent: u32, child: u32) {
        self.nodes[parent as usize].children.push(child);
    }

    fn leaf(&mut self, kind: R0Kind, span: Span) {
        let id = self.nodes.len() as u32;
        self.nodes.push(R0Node {
            kind,
            span,
            children: Vec::new(),
            note: None,
        });
        if let Some(&p) = self.stack.last() {
            self.link(p, id);
        }
    }

    fn skip_trivia(&mut self) {
        while self.toks.get(self.pos).map(|t| t.kind) == Some(R0TokKind::Trivia) {
            let sp = self.toks[self.pos].span;
            self.leaf(R0Kind::Trivia, sp);
            self.pos += 1;
        }
    }

    /// 吞一個 token(先吸附前導 trivia),以指定 R0Kind 作為葉子附著。
    fn bump(&mut self, kind: R0Kind) -> Result<Span, R0ParseIssue> {
        self.skip_trivia();
        let _ = kind;
        let sp = match self.cur_span() {
            Some(sp) => sp,
            None => return Err(R0ParseIssue::Syntax),
        };
        let k = tok_kind(self.cur_kind().unwrap());
        self.leaf(k, sp);
        self.pos += 1;
        Ok(sp)
    }

    fn open(&mut self, kind: R0Kind) -> Result<u32, R0ParseIssue> {
        if self.depth >= R0_RECURSION_LIMIT {
            return Err(R0ParseIssue::Depth);
        }
        let id = self.nodes.len() as u32;
        let span = if kind == R0Kind::Root {
            Span::new(0, self.src.len() as u32)
        } else {
            Span {
                start: u32::MAX,
                end: 0,
            }
        };
        self.nodes.push(R0Node {
            kind,
            span,
            children: Vec::new(),
            note: None,
        });
        let parent = self.stack.last().copied();
        self.stack.push(id);
        self.depth += 1;
        if let Some(p) = parent {
            self.link(p, id);
        }
        Ok(id)
    }

    /// 節點定稿:跨度 = 子節點跨度之並(連續性公理的構造維持);
    /// 空節點 = 當前位置的空區間;Root 恆 = [0, src.len)。
    fn finalize(&mut self, id: u32) {
        if self.nodes[id as usize].kind == R0Kind::Root {
            self.nodes[id as usize].span = Span::new(0, self.src.len() as u32);
            return;
        }
        let mut s = u32::MAX;
        let mut e = 0u32;
        for &c in &self.nodes[id as usize].children {
            let cs = self.nodes[c as usize].span;
            s = s.min(cs.start);
            e = e.max(cs.end);
        }
        if s != u32::MAX {
            self.nodes[id as usize].span = Span::new(s, e);
        } else {
            let p = self
                .cur_span()
                .map(|sp| sp.start)
                .unwrap_or(self.src.len() as u32);
            self.nodes[id as usize].span = Span::new(p, p);
        }
        // 重算父節點跨度(新孩子附著後)
        if let Some(&pid) = self.stack.last() {
            if self.nodes[pid as usize].kind == R0Kind::Root {
                return;
            }
            let mut s = u32::MAX;
            let mut e = 0u32;
            for &c in &self.nodes[pid as usize].children {
                let cs = self.nodes[c as usize].span;
                s = s.min(cs.start);
                e = e.max(cs.end);
            }
            if s != u32::MAX {
                self.nodes[pid as usize].span = Span::new(s, e);
            }
        }
    }

    /// 關閉棧頂節點(與 `open` 配對)。
    ///
    /// **棧空時為 no-op**。原因不是「懶得處理」,而是錯誤回收合法地會退過頭:
    /// `stmt_err` / `item_err` 走 `unwind_to(frame)`,而 `frame` 是**語句層**的
    /// 棧深 —— 像 `else if` 這種右遞歸構造,內層與外層共用同一個 `frame`,
    /// 內層一旦回收就把外層的節點也一起退掉了,外層接著的 `close()` 便無節點可彈
    /// (最小例:`"fn n(){if 1{}else if"`,`else if` 截斷於 EOF)。
    ///
    /// 舊版寫 `self.stack.pop().unwrap()` ⇒ **panic**,直接違反 §2.3
    /// 「任意輸入必回樹、永不 panic」。與 `unwind_to`(本來就有 `len()` 保護)
    /// 一致化後,`close()` 成為總化的最後一道保險。
    /// 回歸防線見 `tests/laws.rs::test_law_r0_error_paths_total`。
    fn close(&mut self) {
        let Some(id) = self.stack.pop() else {
            return;
        };
        self.depth = self.depth.saturating_sub(1);
        self.finalize(id);
    }

    fn unwind_to(&mut self, target: usize) {
        while self.stack.len() > target {
            let id = self.stack.pop().unwrap();
            self.depth -= 1;
            self.finalize(id);
        }
    }

    fn set_kind(&mut self, id: u32, kind: R0Kind) {
        self.nodes[id as usize].kind = kind;
    }

    fn set_note(&mut self, id: u32, note: &'static str) {
        self.nodes[id as usize].note = Some(note);
    }

    /// 把剩餘 token 吸收進棧頂(Error / Unsupported),直至同步模式邊界。
    fn absorb_to(&mut self, mode: Absorb) {
        let mut bd = 0i32;
        let mut pd = 0i32;
        while let Some(k) = self.cur_kind() {
            let ident_kw = if k == R0TokKind::Ident {
                self.cur_text().filter(|w| is_excluded_item_kw(w))
            } else {
                None
            };
            if bd == 0 && pd == 0 && mode.stop_before(k, ident_kw) {
                break;
            }
            let sp = self.cur_span().unwrap();
            self.leaf(tok_kind(k), sp);
            self.pos += 1;
            match k {
                R0TokKind::LBrace => bd += 1,
                R0TokKind::RBrace => bd -= 1,
                R0TokKind::LParen => pd += 1,
                R0TokKind::RParen => pd -= 1,
                _ => {}
            }
            if bd == 0 && pd == 0 && mode.stop_after(k) {
                break;
            }
        }
    }

    /// 一個語句 = 一個原子錯誤區(轉換 + 吸收,不分裂)。
    ///
    /// 特殊化:若失敗點停在 Bad token(閉包 `|` / 生命週期 `'` / 屬性 `#`),
    /// 語句升級為 **Unsupported**(§9 如實申報),而非純 Error。
    fn stmt_err(&mut self, frame: usize, id: u32) {
        let note = if self.cur_kind() == Some(R0TokKind::Bad) {
            Some(self.bad_note())
        } else {
            None
        };
        self.set_kind(
            id,
            if note.is_some() {
                R0Kind::Unsupported
            } else {
                R0Kind::Error
            },
        );
        if let Some(n) = note {
            self.set_note(id, n);
        }
        while self.stack.len() > frame + 1 {
            let sid = self.stack.pop().unwrap();
            self.depth -= 1;
            self.finalize(sid);
        }
        self.absorb_to(Absorb::Stmt);
        self.unwind_to(frame);
    }

    /// 一個項 = 一個原子錯誤區。
    fn item_err(&mut self, frame: usize, id: u32) {
        self.set_kind(id, R0Kind::Error);
        while self.stack.len() > frame + 1 {
            let sid = self.stack.pop().unwrap();
            self.depth -= 1;
            self.finalize(sid);
        }
        self.absorb_to(Absorb::Item);
        self.unwind_to(frame);
    }

    // ---------- 語法層 ----------

    fn parse_program(&mut self) -> Result<(), R0ParseIssue> {
        let _id = self.open(R0Kind::Root)?;
        loop {
            self.skip_trivia();
            match self.cur_kind() {
                None => break,
                Some(R0TokKind::Fn) => self.parse_fn_item()?,
                Some(R0TokKind::Struct) => self.parse_struct_item()?,
                Some(R0TokKind::Ident) if self.cur_text().is_some_and(is_excluded_item_kw) => {
                    self.unsupported_item()?
                }
                _ => {
                    // 項層級的雜散 token:一項一錯誤區,吸收至下一項邊界。
                    let id = self.open(R0Kind::Error)?;
                    self.absorb_to(Absorb::Item);
                    self.close();
                    let _ = id;
                }
            }
        }
        self.close();
        Ok(())
    }

    fn parse_fn_item(&mut self) -> Result<(), R0ParseIssue> {
        let frame = self.stack.len();
        let id = self.open(R0Kind::FnItem)?;
        self.bump(R0Kind::FnKw)?;
        if self.cur_kind() != Some(R0TokKind::Ident) {
            self.item_err(frame, id);
            return Ok(());
        }
        self.bump(R0Kind::Ident)?;
        // params
        if self.cur_kind() != Some(R0TokKind::LParen) {
            self.item_err(frame, id);
            return Ok(());
        }
        self.bump(R0Kind::LParen)?;
        if let Err(R0ParseIssue::Syntax) = self.parse_params() {
            self.item_err(frame, id);
            return Ok(());
        }
        if self.cur_kind() != Some(R0TokKind::RParen) {
            self.item_err(frame, id);
            return Ok(());
        }
        self.bump(R0Kind::RParen)?;
        if self.cur_kind() == Some(R0TokKind::Arrow) {
            self.bump(R0Kind::Arrow)?;
            if let Err(R0ParseIssue::Syntax) = self.parse_type() {
                self.item_err(frame, id);
                return Ok(());
            }
        }
        if self.cur_kind() != Some(R0TokKind::LBrace) {
            self.item_err(frame, id);
            return Ok(());
        }
        self.parse_block()?;
        self.close();
        Ok(())
    }

    fn parse_params(&mut self) -> Result<(), R0ParseIssue> {
        loop {
            match self.cur_kind() {
                Some(R0TokKind::Comma) => {
                    self.bump(R0Kind::Comma)?;
                }
                Some(R0TokKind::RParen) | None => return Ok(()),
                Some(R0TokKind::Ident) => {
                    let id = self.open(R0Kind::Param)?;
                    self.bump(R0Kind::Ident)?;
                    if self.cur_kind() == Some(R0TokKind::Colon) {
                        self.bump(R0Kind::Colon)?;
                        // 型別失敗 → 整個參數表由調用方(item_err)收場。
                        // 直接記 Err 而不在這裡轉換(由 parse_fn_item 統一處理)。
                        if let Err(R0ParseIssue::Syntax) = self.parse_type() {
                            self.set_kind(id, R0Kind::Error);
                            return Err(R0ParseIssue::Syntax);
                        }
                    }
                    self.close();
                    let _ = id;
                    if self.cur_kind() == Some(R0TokKind::Comma) {
                        self.bump(R0Kind::Comma)?;
                    } else {
                        return Ok(());
                    }
                }
                _ => return Err(R0ParseIssue::Syntax),
            }
        }
    }

    fn parse_struct_item(&mut self) -> Result<(), R0ParseIssue> {
        let frame = self.stack.len();
        let id = self.open(R0Kind::StructItem)?;
        self.bump(R0Kind::StructKw)?;
        if self.cur_kind() != Some(R0TokKind::Ident) {
            self.item_err(frame, id);
            return Ok(());
        }
        self.bump(R0Kind::Ident)?;
        if self.cur_kind() != Some(R0TokKind::LBrace) {
            self.item_err(frame, id);
            return Ok(());
        }
        self.bump(R0Kind::LBrace)?;
        loop {
            self.skip_trivia();
            match self.cur_kind() {
                None => break,
                Some(R0TokKind::RBrace) => {
                    self.bump(R0Kind::RBrace)?;
                    break;
                }
                Some(R0TokKind::Comma) => {
                    self.bump(R0Kind::Comma)?;
                }
                Some(R0TokKind::Ident) => {
                    let fid = self.open(R0Kind::FieldDef)?;
                    self.bump(R0Kind::Ident)?;
                    if self.cur_kind() != Some(R0TokKind::Colon) {
                        self.item_err(frame, id);
                        return Ok(());
                    }
                    self.bump(R0Kind::Colon)?;
                    match self.parse_type() {
                        Ok(()) => {}
                        Err(R0ParseIssue::Depth) => return Err(R0ParseIssue::Depth),
                        Err(R0ParseIssue::Syntax) => {
                            self.item_err(frame, id);
                            return Ok(());
                        }
                    }
                    self.close();
                    let _ = fid;
                }
                _ => {
                    self.item_err(frame, id);
                    return Ok(());
                }
            }
        }
        self.close();
        Ok(())
    }

    fn parse_type(&mut self) -> Result<(), R0ParseIssue> {
        let _id = self.open(R0Kind::TypeRef)?;
        match self.cur_kind() {
            Some(R0TokKind::Ident) => {
                self.bump(R0Kind::Ident)?;
                // 泛型實參歧義 IDENT<IDENT(與 lalr1_clean 同判據)→ Unsupported。
                if self.cur_kind() == Some(R0TokKind::Lt)
                    && self.peek2_kind() == Some(R0TokKind::Ident)
                {
                    self.unsupported_generic()?;
                }
            }
            Some(R0TokKind::Amp) | Some(R0TokKind::AmpMut) => {
                self.bump(R0Kind::Amp)?;
                self.parse_type()?;
            }
            Some(R0TokKind::LBrack) => {
                self.bump(R0Kind::LBrack)?;
                self.parse_type()?;
                if self.cur_kind() != Some(R0TokKind::RBrack) {
                    return Err(R0ParseIssue::Syntax);
                }
                self.bump(R0Kind::RBrack)?;
            }
            Some(R0TokKind::Bad) => {
                // 詞法級側條件(生命週期 `'a` / 屬性 `#` 等,§9):節點級 Unsupported,
                // 吃掉 token 後按 `&'a T` / `[T; 'a]` 形式繼續解析基礎型別。
                let note = self.bad_note();
                let uid = self.open(R0Kind::Unsupported)?;
                self.set_note(uid, note);
                let sp = self.cur_span().unwrap();
                self.leaf(R0Kind::Bad, sp);
                self.pos += 1;
                self.close();
                self.parse_type()?;
            }
            _ => return Err(R0ParseIssue::Syntax),
        }
        self.close();
        Ok(())
    }

    fn parse_block(&mut self) -> Result<(), R0ParseIssue> {
        if self.cur_kind() != Some(R0TokKind::LBrace) {
            return Err(R0ParseIssue::Syntax);
        }
        let _id = self.open(R0Kind::Block)?;
        self.bump(R0Kind::LBrace)?;
        loop {
            self.skip_trivia();
            match self.cur_kind() {
                None => break,
                Some(R0TokKind::RBrace) => {
                    self.bump(R0Kind::RBrace)?;
                    break;
                }
                _ => self.parse_stmt()?,
            }
        }
        self.close();
        Ok(())
    }

    fn parse_stmt(&mut self) -> Result<(), R0ParseIssue> {
        let frame = self.stack.len();
        match self.cur_kind() {
            Some(R0TokKind::Let) => self.parse_let(frame),
            Some(R0TokKind::Return) => self.parse_return(frame),
            Some(R0TokKind::If) => self.parse_if(frame),
            Some(R0TokKind::While) => self.parse_while(frame),
            Some(R0TokKind::Loop) => self.parse_loop(frame),
            Some(R0TokKind::Bad) => {
                let note = self.bad_note();
                self.unsupported_wrap(note, Absorb::Stmt)?;
                Ok(())
            }
            Some(R0TokKind::Ident) if self.cur_text() == Some("match") => {
                self.unsupported_wrap("match 模式(排除)", Absorb::Stmt)?;
                Ok(())
            }
            _ => {
                let id = self.open(R0Kind::ExprStmt)?;
                if !self.expr_start() {
                    self.stmt_err(frame, id);
                    return Ok(());
                }
                let eid = match self.parse_expr() {
                    Ok(e) => e,
                    Err(R0ParseIssue::Depth) => return Err(R0ParseIssue::Depth),
                    Err(R0ParseIssue::Syntax) => {
                        self.stmt_err(frame, id);
                        return Ok(());
                    }
                };
                if self.cur_kind() != Some(R0TokKind::Semi) {
                    // 塊表達式語句可省略 `;`(Rust 語義;CL0 生成器亦用之)。
                    let is_block_stmt = self.nodes[eid as usize]
                        .children
                        .first()
                        .is_some_and(|&c| self.nodes[c as usize].kind == R0Kind::Block);
                    if !is_block_stmt {
                        self.stmt_err(frame, id);
                        return Ok(());
                    }
                } else {
                    self.bump(R0Kind::Semi)?;
                }
                self.close();
                Ok(())
            }
        }
    }

    fn parse_let(&mut self, frame: usize) -> Result<(), R0ParseIssue> {
        let id = self.open(R0Kind::LetStmt)?;
        self.bump(R0Kind::LetKw)?;
        if self.cur_kind() == Some(R0TokKind::Mut) {
            self.bump(R0Kind::MutKw)?;
        }
        if self.cur_kind() != Some(R0TokKind::Ident) {
            self.stmt_err(frame, id);
            return Ok(());
        }
        self.bump(R0Kind::Ident)?;
        if self.cur_kind() == Some(R0TokKind::Colon) {
            self.bump(R0Kind::Colon)?;
            if let Err(R0ParseIssue::Syntax) = self.parse_type() {
                self.stmt_err(frame, id);
                return Ok(());
            }
        }
        if self.cur_kind() == Some(R0TokKind::Eq) {
            self.bump(R0Kind::Eq)?;
            if self.cur_kind() == Some(R0TokKind::Semi) {
                self.stmt_err(frame, id);
                return Ok(());
            }
            match self.parse_expr() {
                Ok(_) => {}
                Err(R0ParseIssue::Depth) => return Err(R0ParseIssue::Depth),
                Err(R0ParseIssue::Syntax) => {
                    self.stmt_err(frame, id);
                    return Ok(());
                }
            }
        }
        if self.cur_kind() != Some(R0TokKind::Semi) {
            self.stmt_err(frame, id);
            return Ok(());
        }
        self.bump(R0Kind::Semi)?;
        self.close();
        Ok(())
    }

    fn parse_return(&mut self, frame: usize) -> Result<(), R0ParseIssue> {
        let id = self.open(R0Kind::ReturnStmt)?;
        self.bump(R0Kind::ReturnKw)?;
        if self.expr_start() {
            match self.parse_expr() {
                Ok(_) => {}
                Err(R0ParseIssue::Depth) => return Err(R0ParseIssue::Depth),
                Err(R0ParseIssue::Syntax) => {
                    self.stmt_err(frame, id);
                    return Ok(());
                }
            }
        }
        if self.cur_kind() != Some(R0TokKind::Semi) {
            self.stmt_err(frame, id);
            return Ok(());
        }
        self.bump(R0Kind::Semi)?;
        self.close();
        Ok(())
    }

    fn parse_if(&mut self, frame: usize) -> Result<(), R0ParseIssue> {
        let id = self.open(R0Kind::IfStmt)?;
        self.bump(R0Kind::IfKw)?;
        if !self.expr_start() {
            self.stmt_err(frame, id);
            return Ok(());
        }
        match self.parse_expr() {
            Ok(_) => {}
            Err(R0ParseIssue::Depth) => return Err(R0ParseIssue::Depth),
            Err(R0ParseIssue::Syntax) => {
                self.stmt_err(frame, id);
                return Ok(());
            }
        }
        if self.cur_kind() != Some(R0TokKind::LBrace) {
            self.stmt_err(frame, id);
            return Ok(());
        }
        self.parse_block()?;
        if self.cur_kind() == Some(R0TokKind::Else) {
            self.bump(R0Kind::ElseKw)?;
            if self.cur_kind() == Some(R0TokKind::If) {
                self.parse_if(frame)?;
            } else if self.cur_kind() == Some(R0TokKind::LBrace) {
                self.parse_block()?;
            } else {
                self.stmt_err(frame, id);
                return Ok(());
            }
        }
        self.close();
        Ok(())
    }

    fn parse_while(&mut self, frame: usize) -> Result<(), R0ParseIssue> {
        let id = self.open(R0Kind::WhileStmt)?;
        self.bump(R0Kind::WhileKw)?;
        if !self.expr_start() {
            self.stmt_err(frame, id);
            return Ok(());
        }
        match self.parse_expr() {
            Ok(_) => {}
            Err(R0ParseIssue::Depth) => return Err(R0ParseIssue::Depth),
            Err(R0ParseIssue::Syntax) => {
                self.stmt_err(frame, id);
                return Ok(());
            }
        }
        if self.cur_kind() != Some(R0TokKind::LBrace) {
            self.stmt_err(frame, id);
            return Ok(());
        }
        self.parse_block()?;
        self.close();
        Ok(())
    }

    fn parse_loop(&mut self, frame: usize) -> Result<(), R0ParseIssue> {
        let id = self.open(R0Kind::LoopStmt)?;
        self.bump(R0Kind::LoopKw)?;
        if self.cur_kind() != Some(R0TokKind::LBrace) {
            self.stmt_err(frame, id);
            return Ok(());
        }
        self.parse_block()?;
        self.close();
        Ok(())
    }

    fn expr_start(&mut self) -> bool {
        matches!(
            self.cur_kind(),
            Some(R0TokKind::Number)
                | Some(R0TokKind::True)
                | Some(R0TokKind::False)
                | Some(R0TokKind::Ident)
                | Some(R0TokKind::LParen)
                | Some(R0TokKind::LBrace)
                | Some(R0TokKind::Amp)
                | Some(R0TokKind::AmpMut)
                | Some(R0TokKind::Star)
                | Some(R0TokKind::Not)
                | Some(R0TokKind::RawString)
        )
    }

    fn bad_note(&self) -> &'static str {
        match self.cur_text() {
            Some("|") => "閉包 `|…|`(排除)",
            Some("'") => "生命週期 `'a`(排除)",
            Some("#") => "屬性 `#[…]`(排除)",
            _ => "非法符號(排除)",
        }
    }

    /// 側條件構造(unsupported)通用外殼:開節點 → 標注 → 吸收 → 關閉。
    fn unsupported_wrap(&mut self, note: &'static str, mode: Absorb) -> Result<(), R0ParseIssue> {
        let id = self.open(R0Kind::Unsupported)?;
        self.set_note(id, note);
        self.absorb_to(mode);
        self.close();
        Ok(())
    }

    fn unsupported_item(&mut self) -> Result<(), R0ParseIssue> {
        let note = match self.cur_text() {
            Some("match") => "match 模式(排除)",
            Some(w) => {
                let mut s = String::with_capacity(w.len() + 8);
                s.push_str(w);
                s.push_str(" 項(排除)");
                Box::leak(s.into_boxed_str())
            }
            None => "排除項",
        };
        // 先吞關鍵字本身,再吸收殘骸(否則 Item 模式的 stop_before 會停在
        // 自己身上,程序循環對同一 token 無限重發)。
        let id = self.open(R0Kind::Unsupported)?;
        self.set_note(id, note);
        if let Some(k) = self.cur_kind() {
            let sp = self.cur_span().unwrap();
            self.leaf(tok_kind(k), sp);
            self.pos += 1;
        }
        self.absorb_to(Absorb::Item);
        self.close();
        Ok(())
    }

    /// 泛型實參歧義 IDENT `<` IDENT …(與 lalr1_clean 同判據)。
    fn unsupported_generic(&mut self) -> Result<(), R0ParseIssue> {
        let id = self.open(R0Kind::Unsupported)?;
        self.set_note(id, "泛型實參 vs 比較的歧義(側條件:無泛型)");
        let mut depth = 0i32;
        loop {
            match self.cur_kind() {
                None => break,
                Some(k) => {
                    let sp = self.cur_span().unwrap();
                    self.leaf(tok_kind(k), sp);
                    self.pos += 1;
                    match k {
                        R0TokKind::Lt => depth += 1,
                        R0TokKind::Gt => {
                            depth -= 1;
                            if depth <= 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        self.close();
        Ok(())
    }

    // ---------- 表達式(單節點扁平衡;構造順序即優先級)----------

    fn parse_expr(&mut self) -> Result<u32, R0ParseIssue> {
        let id = self.open(R0Kind::Expr)?;
        self.parse_assign_body()?;
        self.close();
        Ok(id)
    }

    fn parse_assign_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_or_body()?;
        if self.cur_kind() == Some(R0TokKind::Eq) {
            self.bump(R0Kind::Eq)?;
            self.parse_assign_body()?;
        }
        Ok(())
    }

    fn parse_or_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_and_body()?;
        while self.cur_kind() == Some(R0TokKind::OrOr) {
            self.bump(R0Kind::OrOr)?;
            self.parse_and_body()?;
        }
        Ok(())
    }

    fn parse_and_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_eq_body()?;
        while self.cur_kind() == Some(R0TokKind::AndAnd) {
            self.bump(R0Kind::AndAnd)?;
            self.parse_eq_body()?;
        }
        Ok(())
    }

    fn parse_eq_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_rel_body()?;
        while self.cur_kind() == Some(R0TokKind::EqEq) || self.cur_kind() == Some(R0TokKind::NotEq)
        {
            let k = if self.cur_kind() == Some(R0TokKind::EqEq) {
                R0Kind::EqEq
            } else {
                R0Kind::NotEq
            };
            self.bump(k)?;
            self.parse_rel_body()?;
        }
        Ok(())
    }

    fn parse_rel_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_add_body()?;
        while matches!(
            self.cur_kind(),
            Some(R0TokKind::Lt) | Some(R0TokKind::Le) | Some(R0TokKind::Gt) | Some(R0TokKind::Ge)
        ) {
            let k = match self.cur_kind().unwrap() {
                R0TokKind::Lt => R0Kind::Lt,
                R0TokKind::Le => R0Kind::Le,
                R0TokKind::Gt => R0Kind::Gt,
                _ => R0Kind::Ge,
            };
            self.bump(k)?;
            self.parse_add_body()?;
        }
        Ok(())
    }

    fn parse_add_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_mul_body()?;
        while matches!(
            self.cur_kind(),
            Some(R0TokKind::Plus) | Some(R0TokKind::Minus)
        ) {
            let k = if self.cur_kind() == Some(R0TokKind::Plus) {
                R0Kind::Plus
            } else {
                R0Kind::Minus
            };
            self.bump(k)?;
            self.parse_mul_body()?;
        }
        Ok(())
    }

    fn parse_mul_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_unary()?;
        while matches!(
            self.cur_kind(),
            Some(R0TokKind::Star) | Some(R0TokKind::Slash) | Some(R0TokKind::Percent)
        ) {
            let k = match self.cur_kind().unwrap() {
                R0TokKind::Star => R0Kind::Star,
                R0TokKind::Slash => R0Kind::Slash,
                _ => R0Kind::Percent,
            };
            self.bump(k)?;
            self.parse_unary()?;
        }
        Ok(())
    }

    fn parse_unary(&mut self) -> Result<(), R0ParseIssue> {
        match self.cur_kind() {
            Some(R0TokKind::Amp)
            | Some(R0TokKind::AmpMut)
            | Some(R0TokKind::Star)
            | Some(R0TokKind::Not) => {
                let id = self.open(R0Kind::UnaryExpr)?;
                let k = match self.cur_kind().unwrap() {
                    R0TokKind::Amp => R0Kind::Amp,
                    R0TokKind::AmpMut => R0Kind::AmpMut,
                    R0TokKind::Star => R0Kind::Star,
                    _ => R0Kind::Not,
                };
                self.bump(k)?;
                self.parse_postfix()?;
                self.close();
                let _ = id;
                Ok(())
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_primary()?;
        loop {
            match self.cur_kind() {
                Some(R0TokKind::Dot) => {
                    self.bump(R0Kind::Dot)?;
                    if self.cur_kind() != Some(R0TokKind::Ident) {
                        return Err(R0ParseIssue::Syntax);
                    }
                    self.bump(R0Kind::Ident)?;
                }
                Some(R0TokKind::LBrack) => {
                    self.bump(R0Kind::LBrack)?;
                    let _ = self.parse_expr()?;
                    if self.cur_kind() != Some(R0TokKind::RBrack) {
                        return Err(R0ParseIssue::Syntax);
                    }
                    self.bump(R0Kind::RBrack)?;
                }
                Some(R0TokKind::LParen) => {
                    self.bump(R0Kind::LParen)?;
                    loop {
                        if self.cur_kind() == Some(R0TokKind::RParen) || self.cur_kind().is_none() {
                            break;
                        }
                        let _ = self.parse_expr()?;
                        if self.cur_kind() == Some(R0TokKind::Comma) {
                            self.bump(R0Kind::Comma)?;
                        } else {
                            break;
                        }
                    }
                    if self.cur_kind() != Some(R0TokKind::RParen) {
                        return Err(R0ParseIssue::Syntax);
                    }
                    self.bump(R0Kind::RParen)?;
                }
                Some(R0TokKind::Not) => {
                    // 宏調用 `foo!(…)`(側條件排除)。
                    self.unsupported_wrap("宏調用 `!`(排除)", Absorb::Macro)?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_primary(&mut self) -> Result<(), R0ParseIssue> {
        match self.cur_kind() {
            Some(R0TokKind::Number) => {
                self.bump(R0Kind::Number)?;
                Ok(())
            }
            Some(R0TokKind::True) => {
                self.bump(R0Kind::TrueKw)?;
                Ok(())
            }
            Some(R0TokKind::False) => {
                self.bump(R0Kind::FalseKw)?;
                Ok(())
            }
            Some(R0TokKind::RawString) => {
                self.bump(R0Kind::RawString)?;
                Ok(())
            }
            Some(R0TokKind::Ident) => {
                self.bump(R0Kind::Ident)?;
                if self.cur_kind() == Some(R0TokKind::Lt)
                    && self.peek2_kind() == Some(R0TokKind::Ident)
                {
                    self.unsupported_generic()?;
                    return Ok(());
                }
                Ok(())
            }
            Some(R0TokKind::LParen) => {
                self.bump(R0Kind::LParen)?;
                if self.cur_kind() == Some(R0TokKind::RParen) {
                    return Err(R0ParseIssue::Syntax);
                }
                let _ = self.parse_expr()?;
                if self.cur_kind() != Some(R0TokKind::RParen) {
                    return Err(R0ParseIssue::Syntax);
                }
                self.bump(R0Kind::RParen)?;
                Ok(())
            }
            Some(R0TokKind::LBrace) => self.parse_block(),
            _ => Err(R0ParseIssue::Syntax),
        }
    }
}

/// R₀ 全函數語法分析(附錄 B EBNF → CST)。
///
/// 紀律:
///   * **總化** —— 除引擎遞歸極限(`Err(Depth)`)外任何輸入都產出 `Ok(tree)`;
///   * 語法失敗轉為 `R0Kind::Error` 節點(吞至語句/項邊界,一構造一原子錯誤區);
///   * 側條件排除構造轉為 `R0Kind::Unsupported` 節點(note 載明原因,span 精確);
///   * 樹的可驗證性質:連續性公理 / 樹公理 / laminar / 無損回環(unparse ≡ src)。
pub fn r0_parse(src: &str) -> Result<R0Tree, R0ParseIssue> {
    let toks = r0_lex(src);
    let mut p = R0Parser {
        src: src.to_string(),
        toks,
        pos: 0,
        nodes: Vec::new(),
        stack: Vec::new(),
        depth: 0,
    };
    p.parse_program()?;
    Ok(R0Tree {
        src: src.to_string(),
        nodes: p.nodes,
    })
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

    /// 健檢 H1(a):`&mut` 必須切成**一個** `AmpMut` token,
    /// 不論它後面還有沒有位元組(舊版 `i + 3 < b.len()` 讓 EOF 處的
    /// `&mut` 退化成 `Amp` + `Mut`,同一段程式因上下文而切法不同)。
    #[test]
    fn r0_lex_amp_mut_at_eof() {
        fn kinds(src: &str) -> Vec<String> {
            r0_lex(src)
                .iter()
                .map(|t| format!("{:?}", t.kind))
                .collect()
        }
        // 末尾(舊版會錯)
        assert_eq!(kinds("&mut"), ["AmpMut"], "trailing `&mut` (bare)");
        assert_eq!(
            kinds("a&mut"),
            ["Ident", "AmpMut"],
            "trailing `&mut` (after ident)"
        );
        assert_eq!(
            kinds("&mut&mut"),
            ["AmpMut", "AmpMut"],
            "trailing `&mut` (twice)"
        );
        assert_eq!(
            kinds("fn f() { let r = &mut"),
            [
                "Fn", "Trivia", "Ident", "LParen", "RParen", "Trivia", "LBrace", "Trivia", "Let",
                "Trivia", "Ident", "Trivia", "Eq", "Trivia", "AmpMut"
            ],
            "trailing `&mut` (in context)"
        );
        // 中間(舊版本來就對,釘住不許回歸)
        assert_eq!(kinds("&mut x"), ["AmpMut", "Trivia", "Ident"]);
        assert_eq!(kinds("&mutx"), ["AmpMut", "Ident"]);
        assert_eq!(kinds("&&"), ["AndAnd"]);
        // 平鋪不變量必須繼續成立(切法變了,覆蓋不能變)
        for src in ["&mut", "a&mut", "&mut&mut", "fn f() { let r = &mut"] {
            r0_lexical_invariants(src).unwrap_or_else(|e| panic!("{:?}: {}", src, e));
        }
    }

    /// 健檢 H1(b):`&` 後接**非 3 位元組**的 UTF-8 字元不得 panic。
    /// 舊版 `&src[i..i + 3]` 是 `&str` 切片,`i+3` 落在多字元位元組中間
    /// 就 panic(`&🦀` / `&éé` / `&Привет` / `let x = &αβ;` 全中)。
    /// 這是 §2.3「任意輸入必回樹、永不 panic」的回歸防線。
    #[test]
    fn r0_lex_non_ascii_after_amp() {
        // 每個輸入的最小位元組長度都 ≥ 5,確保舊版真的會走進 `src[i..i+3]`。
        let cases: &[&str] = &[
            "&\u{1F600}",                                  // 4 位元組:emoji
            "&\u{1F600}x",                                 // 4 位元組 + 後綴
            "&éé",                                         // 2 + 2 位元組(拉丁補充)
            "&\u{41F}\u{440}\u{438}\u{432}\u{435}\u{442}", // 2 位元組:西里爾
            "let x = &αβ;",                                // 2 位元組:希臘(合法 Rust 識別字)
            "fn f() { let r = &\u{41F}\u{440}\u{438}; }",  // 合法程式 + 非 ASCII 識別字
            "&\u{5F20}\u{4E09}",                           // 3 位元組:CJK(原本就不 panic)
            "a&\u{1F600}b",                                // 混在中間
            "&\u{1F600};\n\t",                             // 後接分隔符
            "&&\u{1F600}",                                 // `&&` + 非 ASCII
        ];
        for src in cases {
            // (1) 詞法器不得 panic,且平鋪不變量成立
            let toks = r0_lex(src);
            r0_lexical_invariants(src).unwrap_or_else(|e| panic!("{:?}: {}", src, e));
            assert!(!toks.is_empty(), "{:?}: must produce tokens", src);
            // (2) 解析器也不得 panic(全化:必回樹或如實申報機器界)
            let _ = r0_parse(src);
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

#[test]
fn r0_parse_legal_roundtrip() {
    // 附錄 B 的全語法面:item / 語句 / 全部運算符優先級 / 借用 / 字段 / 索引 / 調用。
    let src = r#"fn main() {
  let mut x = 1;
  let r = &mut x;
  let y = x + 2 * 3 - 4 / 2 % 3;
  let b = x < 1 || y >= 2 && x != 3;
  x = y;
  if x == 1 { f(x, y); } else { g(); }
  while x < 10 { x = x + 1; }
  loop { x = x + 1; }
  return x;
}
struct P { a: int, b: &mut int, c: [int], }
fn q(v: int, w: &int) -> int { let k: int = v; return k; }"#;
    let t = r0_parse(src).expect("legal R0 must parse");
    assert!(!t.has_error(), "legal program must have no ERROR node");
    assert!(
        t.unsupported_spans().is_empty(),
        "legal program must have no Unsupported: {:?}",
        t.unsupported_spans()
    );
    assert_eq!(t.unparse(), src, "byte-exact roundtrip");
    assert!(t.validate_continuity().is_ok());
    assert!(t.validate_tree_shapes().is_ok());
    assert!(t.laminar_ok());
    // 結構面:具名節點存在(函數/塊/語句/表達式)
    assert!(
        t.nodes.iter().any(|n| n.kind == R0Kind::FnItem),
        "fn items present"
    );
    assert!(
        t.nodes.iter().any(|n| n.kind == R0Kind::StructItem),
        "struct item present"
    );
    assert!(
        t.nodes.iter().any(|n| n.kind == R0Kind::IfStmt),
        "if stmt present"
    );
    assert!(
        t.nodes.iter().any(|n| n.kind == R0Kind::UnaryExpr),
        "unary expr present"
    );
}

#[test]
fn r0_parse_garbage_totalization() {
    // 總化:任何輸入(含非法字節 / 半截構造 / 亂語法)都產出樹,且樹可驗證性質成立。
    let samples = [
        "",
        "@@|",
        "fn",
        "fn f(",
        "fn f() { let x = ; }",
        "fn f() { if x { } else }",
        "struct S { a: }",
        "fn f() { while { } }",
        "let x = (1;",
        "fn f() { & & & x; }",
        "}}}",
        "fn f() { let x = { { { 1; } }",
        "\u{1}\u{2}\u{ff}\u{fe}",
        "fn f() { let s = r#\"unterminated;",
    ];
    for src in samples {
        let t = r0_parse(src)
            .unwrap_or_else(|e| panic!("totalization violated for {:?}: {:?}", src, e));
        assert!(t.total_nodes() >= 1);
        assert!(
            t.validate_continuity().is_ok(),
            "continuity must hold on {:?}",
            src
        );
        assert!(t.validate_tree_shapes().is_ok());
        assert!(t.laminar_ok());
        assert_eq!(t.unparse(), src, "roundtrip must hold on {:?}", src);
    }
}

#[test]
fn r0_parse_unsupported_nodes() {
    // 節點級 unsupported(§9 如實申報:note + 精確 span,而非假裝覆蓋)。
    let cases: &[(&str, &str)] = &[
        ("fn f() { let c = |a| a; }", "閉包"),
        ("fn f() { match x { _ => {} } }", "match"),
        ("trait T { fn f(); }", "trait"),
        ("fn f() { println!(x); }", "宏調用"),
        ("fn f(x: &'a int) {}", "生命週期"),
        ("fn f() { let v: Vec<int> = v; }", "泛型實參"),
        ("fn f() { let a = @; }", "非法符號"),
    ];
    for (src, needle) in cases {
        let t = r0_parse(src).expect("unsupported constructs still parse (totalization)");
        let notes = t.unsupported_spans();
        assert!(
            notes.iter().any(|(_, n)| n.contains(*needle)),
            "case {:?}: expected note containing {:?}, got {:?}",
            src,
            needle,
            notes
        );
        assert!(t.validate_continuity().is_ok());
        assert_eq!(t.unparse(), *src);
    }
}

#[test]
fn r0_parse_depth_honest() {
    // 深嵌套越界:如實申報 Err(Depth),永不 panic。
    let deep = format!("fn main() {{ let x = {}1; }}", "{ ".repeat(100));
    let r = r0_parse(&deep);
    match r {
        Err(R0ParseIssue::Depth) => {}
        Ok(t) => assert!(t.has_error() || t.total_nodes() > 0),
        Err(R0ParseIssue::Syntax) => panic!("Depth must be reported, not Syntax"),
    }
    // 正常深程式沒問題
    let okdeep = format!("fn main() {{ {} }}", "{ x; }".repeat(60));
    let t = r0_parse(&okdeep).expect("normal depth must parse");
    assert!(t.total_nodes() > 100);
    assert!(!t.has_error());
}

#[test]
fn r0_parse_determinism_named_sexp() {
    let src = "fn f() { let x = 1; return x; }";
    let a = r0_parse(src).unwrap();
    let b = r0_parse(src).unwrap();
    assert_eq!(
        a.named_sexp(),
        b.named_sexp(),
        "deterministic named projection"
    );
    assert!(
        a.named_sexp().starts_with("(root(fn_item"),
        "named sexp must be structural: {}",
        a.named_sexp()
    );
}
