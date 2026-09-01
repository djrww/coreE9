//! 屬性測試的輸入宇宙(§7.3 定律 × 載體矩陣的「窮舉 / 抽樣」輸入來源)。
//!
//! 兩類輸入:
//!   * `gen_legal` —— 依附錄 A EBNF 生成的**合法** CL0 程式(驗 L7a 不假報、
//!     L1 無損回環、L5 嵌套)。
//!   * `gen_garbage` / `gen_half_file` —— 任意字節串與「寫一半的檔案」
//!     (驗 L1 全輸入回環、L7b 良構極大)。
//!   * `gen_edit` —— 隨機編輯(編輯單體、增量層工具的輸入)。
//!
//! 全部使用自帶的 xorshift64(零依賴、確定性、可重現)。

use crate::edit::Edit;

/// 確定性 xorshift64 生成器(屬性測試的輸入宇宙;同種子 ⇒ 完全可重現)。
pub struct Rng {
    state: u64,
}

impl Rng {
    /// 以種子構造生成器;0 規範化為 1(xorshift 狀態不可為 0)。
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed.max(1) }
    }

    /// 下一個 u64(原生 xorshift64,零依賴)。
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// [0, n) 均勻整數(n = 0 時回 0)。
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }

    /// 以機率 num/den 回 true。
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.below(den) < num
    }

    /// 均勻選取切片中的一個元素(借用返回;生成器的關鍵隨機原語)。
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

const NAMES: [&str; 6] = ["x", "y", "z", "r", "s", "w"];
const FNS: [&str; 4] = ["f", "g", "h", "k"];
const TYPES: [&str; 2] = ["int", "thunk"];

/// 生成一個語法合法的 CL0 程式(依附錄 A 的 EBNF 結構遞歸生成)。
pub fn gen_legal(rng: &mut Rng) -> String {
    let mut out = String::new();
    let items = 1 + rng.below(2) as usize; // 1..=2
    for i in 0..items {
        if i > 0 {
            out.push('\n');
        }
        gen_item(rng, &mut out, 0);
    }
    out
}

fn gen_item(rng: &mut Rng, out: &mut String, depth: usize) {
    let name = FNS[rng.below(FNS.len() as u64) as usize];
    out.push_str("fn ");
    out.push_str(name);
    out.push('(');
    let nparams = rng.below(3) as usize;
    for i in 0..nparams {
        if i > 0 {
            out.push_str(", ");
        }
        let p = NAMES[rng.below(NAMES.len() as u64) as usize];
        out.push_str(p);
        if rng.chance(1, 3) {
            out.push_str(": ");
            if rng.chance(1, 2) {
                out.push('&');
                if rng.chance(1, 2) {
                    out.push_str("mut ");
                }
            }
            out.push_str(TYPES[rng.below(TYPES.len() as u64) as usize]);
        }
    }
    out.push_str(") ");
    gen_block(rng, out, depth);
}

fn gen_block(rng: &mut Rng, out: &mut String, depth: usize) {
    out.push_str("{\n");
    let nstmts = rng.below(3) as usize;
    for _ in 0..nstmts {
        gen_stmt(rng, out, depth + 1);
    }
    out.push('}');
}

fn gen_stmt(rng: &mut Rng, out: &mut String, depth: usize) {
    if depth > 3 {
        out.push_str("  f();\n");
        return;
    }
    match rng.below(8) {
        0 | 1 => {
            out.push_str("  let ");
            if rng.chance(1, 3) {
                out.push_str("mut ");
            }
            out.push_str(NAMES[rng.below(NAMES.len() as u64) as usize]);
            if rng.chance(3, 4) {
                out.push_str(" = ");
                let p = rng.below(5);
                if p == 0 {
                    // 借了一個既有名字(& 或 &mut):制造 liveness 素材
                    let src = NAMES[rng.below(NAMES.len() as u64) as usize];
                    out.push('&');
                    if rng.chance(1, 2) {
                        out.push_str("mut ");
                    }
                    out.push_str(src);
                } else {
                    gen_expr(rng, out, depth + 1);
                }
            }
            out.push_str(";\n");
        }
        2 | 3 => {
            out.push_str("  let ");
            if rng.chance(1, 2) {
                out.push_str("mut ");
            }
            out.push_str(NAMES[rng.below(NAMES.len() as u64) as usize]);
            out.push_str(" = ");
            gen_expr(rng, out, depth + 1);
            out.push_str(";\n");
        }
        4 => {
            out.push_str("  if ");
            gen_expr(rng, out, depth + 1);
            out.push(' ');
            gen_block(rng, out, depth);
            if rng.chance(1, 2) {
                out.push_str(" else ");
                gen_block(rng, out, depth);
            }
            out.push('\n');
        }
        5 => {
            out.push_str("  while ");
            gen_expr(rng, out, depth + 1);
            out.push(' ');
            gen_block(rng, out, depth);
            out.push('\n');
        }
        6 => {
            out.push_str("  ");
            gen_expr(rng, out, depth + 1);
            out.push_str(";\n");
        }
        _ => {
            // 移動語義載體:函數調用
            out.push_str("  ");
            out.push_str(FNS[rng.below(FNS.len() as u64) as usize]);
            out.push('(');
            let n = rng.below(3) as usize;
            for i in 0..n {
                if i > 0 {
                    out.push_str(", ");
                }
                gen_expr(rng, out, depth + 1);
            }
            out.push_str(");\n");
        }
    }
}

fn gen_expr(rng: &mut Rng, out: &mut String, depth: usize) {
    // unary { binop unary }
    gen_unary(rng, out, depth);
    if rng.chance(1, 2) {
        match rng.below(5) {
            0 => out.push_str(" + "),
            1 => out.push_str(" - "),
            2 => out.push_str(" * "),
            3 => out.push_str(" == "),
            _ => out.push_str(" < "),
        }
        gen_unary(rng, out, depth);
    }
}

fn gen_unary(rng: &mut Rng, out: &mut String, depth: usize) {
    if rng.chance(1, 4) {
        out.push('&');
        if rng.chance(1, 2) {
            out.push_str("mut ");
        }
    } else if rng.chance(1, 8) {
        out.push('*');
    }
    gen_primary(rng, out, depth);
}

fn gen_primary(rng: &mut Rng, out: &mut String, depth: usize) {
    match rng.below(5) {
        0 => {
            out.push_str(&rng.below(99).to_string());
        }
        1 => out.push_str(if rng.chance(1, 2) { "true" } else { "false" }),
        2 => out.push_str(NAMES[rng.below(NAMES.len() as u64) as usize]),
        3 => {
            out.push_str(FNS[rng.below(FNS.len() as u64) as usize]);
            out.push('(');
            let n = rng.below(3) as usize;
            for i in 0..n {
                if i > 0 {
                    out.push_str(", ");
                }
                gen_expr(rng, out, depth + 1);
            }
            out.push(')');
        }
        _ => {
            if depth < 3 {
                gen_block(rng, out, depth);
            } else {
                out.push('1');
            }
        }
    }
}

/// 任意(髒)字節串:覆蓋所有 token 種類、關鍵字碎片、註釋、非法字元。
pub fn gen_garbage(rng: &mut Rng, max_len: usize) -> String {
    let vocab: &[&str] = &[
        "x",
        "y",
        "f",
        "1",
        "77",
        "let",
        "mut",
        "fn",
        "if",
        "else",
        "while",
        "true",
        "false ",
        "&",
        "&mut ",
        "*",
        "+",
        "-",
        "==",
        "<",
        "=",
        "(",
        ")",
        "{",
        "}",
        ";",
        ":",
        ",",
        " ",
        " ",
        "  ",
        "\n",
        "\t",
        "//",
        "// comment\n",
        "@",
        "#",
        "%",
        "'",
        "\"",
        "\\",
        "`",
        "..",
    ];
    let mut out = String::new();
    let n = rng.below(max_len as u64 + 1) as usize;
    for _ in 0..n {
        out.push_str(rng.pick(vocab));
        if out.len() > max_len * 3 {
            break;
        }
    }
    out
}

/// 「寫一半的檔案」:對合法程式做截斷 / 在中點插入髒文本(§2.3 / L7b 注入式測試)。
pub fn gen_half_file(rng: &mut Rng, legal: &str) -> String {
    match rng.below(4) {
        0 | 1 => {
            // 截斷
            let p = rng.below(legal.len() as u64 + 1) as usize;
            legal[..p].to_string()
        }
        2 => {
            // 中點插入髒文本
            let p = rng.below(legal.len() as u64 + 1) as usize;
            let mut out = String::new();
            out.push_str(&legal[..p]);
            out.push_str(rng.pick(&[
                "@@", "}", "; }", "& &", "let x =", "if", "((", "\n\n", "fn z() {", "== =",
            ]));
            out.push_str(&legal[p..]);
            out
        }
        _ => gen_garbage(rng, 40),
    }
}

/// 隨機編輯(在 old 源碼坐標空間;替換文本可任意)。
pub fn gen_edit(rng: &mut Rng, src_len: usize) -> Edit {
    let p = rng.below(src_len as u64 + 1) as usize;
    let max_old = (src_len - p).min(4);
    let old_len = rng.below(max_old as u64 + 1) as usize;
    let text_len = rng.below(6) as usize;
    let mut text = String::new();
    let chips: &[&str] = &[
        "x", "1", " ", "&", "mut ", "*", ";", "{", "}", "(", ")", "=", "+", "f()", "let ", "\n",
        "//c\n", "@", "== ", "0", ", ",
    ];
    for _ in 0..text_len {
        text.push_str(rng.pick(chips));
    }
    Edit::new(p as u32, (p + old_len) as u32, &text)
}
