//! CL0 詞法層(§5.1 詞法層:DFA——CL0 詞法完全正則)。
//!
//! 不變量(詞法不變量,由測試 `lex_lexical_invariants` 機械驗證):
//!   * **平鋪(tiling)**:`lex(s)` 的 token 序列逐字節覆蓋 `s`,無縫隙、無重疊、
//!     無遺漏:∀i: tᵢ.end == tᵢ₊₁.start,且 t₀.start == 0、t_last.end == s.len()。
//!     這是 L1(無損回環)的字節級前提。
//!   * 空白與 `//` 註釋一律是 trivia token,參與樹的序列化與回環(L1),
//!     因此「重寫不重排用戶代碼」在 CL0 上可機械驗證。
//!   * 任何非法字字節也會得到一個 `Bad` token(長度為該字元的 utf8 長度),
//!     所以詞法器是**全函數**:對任何輸入都返回完整平鋪。
//!   * 關鍵字是保留字:fn / let / mut / if / else / while / true / false。

use crate::span::Span;

/// CL0 詞法器輸出(有限自動機的字母表;§5.1:平鋪 = 每個字節恰屬一個 token)。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TokKind {
    /// 識別字:`[a-z][a-z0-9]*`(CL0 只用小寫;R₀ 擴展見 `r0` 模塊)。
    Ident,
    /// 十進制整數字面量。
    Number,
    /// 布林字面量 `true`。
    True,
    /// 布林字面量 `false`。
    False,
    /// 關鍵字 `fn`(函數項的開頭)。
    Fn,
    /// 關鍵字 `let`(綁定語句的開頭)。
    Let,
    /// 關鍵字 `mut`(可變綁定修飾)。
    Mut,
    /// 關鍵字 `if`。
    If,
    /// 關鍵字 `else`。
    Else,
    /// 關鍵字 `while`。
    While,
    /// `&` 借用運算符(規格中的 &amp; / &amp;amp;)。
    Amp,
    /// `*` 解引用運算符。
    Star,
    /// `+` 二元運算符。
    Plus,
    /// `-` 二元運算符。
    Minus,
    /// `==` 相等比較。
    EqEq,
    /// `<` 小於比較。
    Lt,
    /// `=` 綁定/賦值。
    Eq,
    /// `(`(參數/實參列表左界)。
    LParen,
    /// `)`(右界)。
    RParen,
    /// `{`(塊左界)。
    LBrace,
    /// `}`(塊右界;同時是語句同步點,§2.3)。
    RBrace,
    /// `;`(語句終止符)。
    Semi,
    /// `:`(型別標註)。
    Colon,
    /// `,`(參數/實參分隔)。
    Comma,
    /// 空白與註釋:平鋪保留但**不**進入結構樹(§1.3 投影的物象)。
    Trivia,
    /// 詞法錯誤字元(仍佔一個平鋪 token ⇒ 全化,L7 的詞法基石)。
    Bad,
}

impl TokKind {
    /// 是否為結構 token(即非 Trivia;§5.1 平鋪/結構二分)。
    pub fn is_struct(self) -> bool {
        self != TokKind::Trivia
    }
}

/// 詞法單元:種類 + 源碼跨度(半開)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    /// token 種類。
    pub kind: TokKind,
    /// 在源碼中的平鋪跨度。
    pub span: Span,
}

const TRANSLIT: &str = "fnletmutifelsewhiletruefalse";

fn keyword_of(word: &str) -> Option<TokKind> {
    match word {
        "fn" => Some(TokKind::Fn),
        "let" => Some(TokKind::Let),
        "mut" => Some(TokKind::Mut),
        "if" => Some(TokKind::If),
        "else" => Some(TokKind::Else),
        "while" => Some(TokKind::While),
        "true" => Some(TokKind::True),
        "false" => Some(TokKind::False),
        _ => None,
    }
}

/// DFA 詞法:單遍掃描,平鋪返回 token 序列(全函數,永不失敗)。
pub fn lex(src: &str) -> Vec<Token> {
    let bytes = src.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let start = i;
        let b = bytes[i];
        match b {
            // trivia:空白
            b' ' | b'\t' | b'\r' | b'\n' => {
                while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n') {
                    i += 1;
                }
                toks.push(Token {
                    kind: TokKind::Trivia,
                    span: Span::new(start as u32, i as u32),
                });
            }
            // trivia:// 註釋(至行尾,含換行)
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                toks.push(Token {
                    kind: TokKind::Trivia,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'0'..=b'9' => {
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                toks.push(Token {
                    kind: TokKind::Number,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &src[start..i];
                toks.push(Token {
                    kind: keyword_of(word).unwrap_or(TokKind::Ident),
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'&' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::Amp,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'*' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::Star,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'+' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::Plus,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'-' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::Minus,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'=' => {
                i += 1;
                let kind = if i < bytes.len() && bytes[i] == b'=' {
                    i += 1;
                    TokKind::EqEq
                } else {
                    TokKind::Eq
                };
                toks.push(Token {
                    kind,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'<' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::Lt,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'(' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::LParen,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b')' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::RParen,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'{' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::LBrace,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b'}' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::RBrace,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b';' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::Semi,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b':' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::Colon,
                    span: Span::new(start as u32, i as u32),
                });
            }
            b',' => {
                i += 1;
                toks.push(Token {
                    kind: TokKind::Comma,
                    span: Span::new(start as u32, i as u32),
                });
            }
            _ => {
                // 非 ASCII 或非法符號:一個 Bad token,長度 = 該字元的 utf8 長度。
                let ch = src[i..].chars().next().unwrap();
                i += ch.len_utf8();
                toks.push(Token {
                    kind: TokKind::Bad,
                    span: Span::new(start as u32, i as u32),
                });
            }
        }
    }
    let _ = TRANSLIT; // (保留:關鍵字表來源標記)
    toks
}

/// 詞法不變量檢驗(「平鋪」公理,L1 的字節級地基)。
pub fn lexical_invariants(src: &str) -> Result<(), String> {
    let toks = lex(src);
    let mut expected = 0u32;
    for t in &toks {
        if t.span.start != expected {
            return Err(format!(
                "gap/overlap at byte {}: token {:?} starts at {}",
                expected, t.kind, t.span.start
            ));
        }
        expected = t.span.end;
    }
    if expected != src.len() as u32 {
        return Err(format!(
            "lexer did not cover source: covered {} of {} bytes",
            expected,
            src.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_lexical_invariants() {
        // 對任意(含髒)輸入,詞法器必須平鋪全源碼。用一小段窮舉驗證。
        let alphabet = [
            "x", "1", " ", "(", ")", "{", "}", ";", "&", "=", "//", "\n", "@", "fn", "let", "mut",
            "if", "else", "while", "true", "false", "*", "+", "-", "<", ":", ",",
        ];
        fn gen(alphabet: &[&str], len: usize, cur: &mut String, n: usize, out: &mut Vec<String>) {
            if n >= len {
                out.push(cur.clone());
                return;
            }
            for a in alphabet {
                cur.push_str(a);
                gen(alphabet, len, cur, n + 1, out);
                let k = a.len();
                cur.truncate(cur.len() - k);
            }
        }
        let mut samples = Vec::new();
        for l in 1..=3 {
            gen(&alphabet, l, &mut String::new(), 0, &mut samples);
        }
        for s in samples {
            match lexical_invariants(&s) {
                Ok(()) => {}
                Err(e) => panic!("lexical tiling violated for {:?}: {}", s, e),
            }
        }
    }
}
