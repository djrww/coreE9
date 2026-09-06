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

// ===========================================================================
// P4-2:R₀ 語義感知生成器(by-construction 期望判決)
//
// 參照 RustSmith 的「生成時即知合法性」:樣本不是「隨機文本問 rustc 怎麼說」,
// 而是**構造上**已知期望判決 —— 借用紀律由生成器內部的活躍借用簿記把關,
// 衝突模式被明確注入。由此「生成器期望 ≠ rustc 判決」即是 BUG 候選
// (生成器推理錯誤或真發現),「Lexical 軌在借用衝突碼下漏報」即是下界律違反。
// 判定權不轉移:這裡的「期望」是生成器的構造知識,不是第二裁判。
// ===========================================================================

/// R₀ 樣本的期望判決(生成時已知)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum R0Expect {
    /// 構造上無借用衝突/無 use-after-move ⇒ rustc 應接受。
    Accept,
    /// 構造上注入了借用衝突 ⇒ rustc 應拒絕,且碼落借用衝突類。
    RejectBorrow,
    /// 構造上注入了 use-after-move ⇒ rustc 應拒絕(移動類)。
    RejectMove,
}

/// 一個生成的 R₀ 樣本 + 其 by-construction 期望。
#[derive(Clone, Debug)]
pub struct R0Sample {
    /// 源碼(純 R₀、合法 Rust 子集;rustc 可直接裁判)。
    pub src: String,
    /// 生成時已知的期望判決。
    pub expect: R0Expect,
}

const R0_HELPERS: &str = "\
fn g(p: &mut i32) {
    *p = *p + 1;
}
fn h(q: &i32) -> i32 {
    return *q;
}
";

struct R0Body {
    out: String,
    counter: usize,
}

impl R0Body {
    fn fresh(&mut self, base: &str) -> String {
        let n = self.counter;
        self.counter += 1;
        format!("{base}{n}")
    }
    fn stmt(&mut self, s: &str) {
        let body = s.trim_end_matches(';');
        self.out.push_str("    ");
        self.out.push_str(body);
        // R₀ 文法:控制流語句(if/while 以 `}` 結尾)不收分號;其餘語句收。
        if !body.ends_with('}') {
            self.out.push(';');
        }
        self.out.push('\n');
    }
}

#[derive(Clone, Copy, PartialEq)]
enum R0Mode {
    Safe,
    ConflictBorrow,
    ConflictMove,
}

/// 生成一個 R₀ 語義樣本(by-construction 期望判決;確定性,同種子可重現)。
///
/// 構造不變式(Safe 模式的 rustc 接受性由以下紀律保證):
///   * 對變量 v 的**寫**(直接寫 / 經 `*r = …` 寫穿)只在 v 上**無活躍 let 借用**時發出;
///   * `let r = &mut v` 的創造要求 v 上無任何活躍借用;`let r = &v` 同;
///     (⇒ 兩個 &mut let 借用從不同時活躍,sh 與 mut 也不同時活躍);
///   * 借用的「最後使用」由生成器明確發出(terminate 操作)或落在 while 條件
///     (回邊求值 ⇒ 活性止於迴圈出口)——此後對目標的寫合法;
///   * call-arg 借用(`g(&mut v)` / `h(&v)`)死於呼叫返回,構造上永不與後續衝突;
///   * Safe 模式從不發出引用移動(`let m2 = m`)——移動只在 ConflictMove 注入。
pub fn gen_r0_semantic(rng: &mut Rng) -> R0Sample {
    let mode = match rng.below(10) {
        0..=3 => R0Mode::Safe,
        4..=8 => R0Mode::ConflictBorrow,
        _ => R0Mode::ConflictMove,
    };
    let mut b = R0Body {
        out: String::new(),
        counter: 0,
    };
    b.out.push_str(R0_HELPERS);
    b.out.push_str("fn f() {\n");

    // 變量池:(名字, 是否 mut)。名字唯一(counter)⇒ 無遮蔽細微差。
    let mut vars: Vec<(String, bool)> = Vec::new();
    let mut borrows: Vec<(String, String, bool)> = Vec::new(); // (ref, target, 是否 mut)
    let free =
        |vars: &[(String, bool)], borrows: &[(String, String, bool)]| -> Vec<(String, bool)> {
            vars.iter()
                .filter(|(n, _)| !borrows.iter().any(|(_, t, _)| t == n))
                .cloned()
                .collect()
        };

    // 前導:1–3 個 int 變量(ConflictMove 只建 1 個,其餘由模式自建)。
    let nv = if mode == R0Mode::ConflictMove {
        1
    } else {
        1 + rng.below(3) as usize
    };
    for _ in 0..nv {
        let mut_ = rng.chance(2, 3);
        let v = b.fresh("v");
        let head = if mut_ { format!("mut {v}") } else { v.clone() };
        b.stmt(&format!("let {head} = {};", rng.below(9)));
        vars.push((v, mut_));
    }

    match mode {
        R0Mode::Safe => {
            // 操作帶顯式語義:Raw = 只發語句;BorrowUse = 使用借用;
            // BorrowEnd = 發出「最後使用」並移除借用(此後目標可自由寫)。
            enum Op {
                Raw(String),
                BorrowUse(usize),
                BorrowEnd(usize),
            }
            let nops = 2 + rng.below(5) as usize;
            for _ in 0..nops {
                let mut ops: Vec<(u64, Op)> = Vec::new();
                for (v, mut_) in free(&vars, &borrows) {
                    if mut_ {
                        ops.push((3, Op::Raw(format!("{v} = {v} + 1")))); // 寫
                        ops.push((2, Op::Raw(format!("g(&mut {v})")))); // call-arg mut
                        ops.push((3, Op::Raw(format!("let {} = &mut {v};", b.fresh("r")))));
                        // 創造 mut
                    }
                    ops.push((2, Op::Raw(format!("let {} = h(&{v});", b.fresh("t"))))); // call-arg sh
                    ops.push((3, Op::Raw(format!("let {} = &{v};", b.fresh("r"))))); // 創造 sh
                    if mut_ {
                        ops.push((1, Op::Raw(format!("while {v} > 3 {{ {v} = {v} - 1; }}"))));
                    }
                    ops.push((
                        1,
                        Op::Raw(format!("if {v} > 3 {{ let {} = {v} + 1; }}", b.fresh("t"))),
                    ));
                }
                for (i, (_r, _, mut_)) in borrows.iter().enumerate() {
                    if *mut_ {
                        ops.push((3, Op::BorrowUse(i))); // 使用(寫穿;唯一活躍借用 ⇒ 安全)
                        ops.push((3, Op::BorrowEnd(i))); // 終止(最後使用)
                    } else {
                        ops.push((3, Op::BorrowUse(i))); // 使用
                        ops.push((3, Op::BorrowEnd(i))); // 終止
                    }
                }
                // while 條件用借用(回邊):條件是最後使用,出口後借用死亡;
                // 迴圈體只寫「其他」自由變量 ⇒ 構造安全;選中即同時終止借用。
                let while_cond: Option<(String, String)> = borrows
                    .iter()
                    .find(|(_, _, m)| !m)
                    .map(|(r, _, _)| r.clone())
                    .zip(
                        free(&vars, &borrows)
                            .iter()
                            .find(|(_n, m)| *m)
                            .map(|(n, _)| n.clone()),
                    );

                if let Some((r, w)) = &while_cond {
                    ops.push((1, Op::Raw(format!("while *{r} > 3 {{ {w} = {w} + 1; }}"))));
                }
                if ops.is_empty() {
                    break;
                }
                let total: u64 = ops.iter().map(|(w, _)| *w).sum();
                let pick = rng.below(total);
                let mut acc = 0u64;
                let idx = ops
                    .iter()
                    .position(|(w, _)| {
                        acc += w;
                        pick < acc
                    })
                    .unwrap();
                match &ops[idx].1 {
                    Op::Raw(code) => {
                        b.stmt(code);
                        // 選中的是 while-條件借用操作 ⇒ 條件即最後使用,終止該借用。
                        if let Some((r, w)) = &while_cond {
                            if code == &format!("while *{r} > 3 {{ {w} = {w} + 1; }}") {
                                if let Some(i) = borrows.iter().position(|(rb, _, _)| rb == r) {
                                    borrows.remove(i);
                                }
                            }
                        }
                    }
                    Op::BorrowUse(i) => {
                        let (r, _, mut_) = &borrows[*i];
                        let code = if *mut_ {
                            format!("*{r} = *{r} + 1;")
                        } else {
                            format!("let {} = *{r};", b.fresh("t"))
                        };
                        b.stmt(&code);
                    }
                    Op::BorrowEnd(i) => {
                        let (r, _, mut_) = &borrows[*i];
                        let code = if *mut_ {
                            format!("*{r} = *{r} + 1;")
                        } else {
                            format!("let {} = *{r};", b.fresh("t"))
                        };
                        b.stmt(&code);
                        borrows.remove(*i);
                    }
                }
            }
        }
        R0Mode::ConflictBorrow => {
            // 注入一個構造上必為借用衝突的模式(目標為全新 mut 變量,
            // 前導借用不介入)。
            let p = rng.below(5);
            let v = b.fresh("v");
            b.stmt(&format!("let mut {v} = {};", rng.below(9)));
            match p {
                0 => {
                    let r = b.fresh("r");
                    let t = b.fresh("t");
                    b.stmt(&format!("let {r} = &{v};"));
                    b.stmt(&format!("{v} = {v} + 1;"));
                    b.stmt(&format!("let {t} = *{r};"));
                }
                1 => {
                    let r = b.fresh("r");
                    let t = b.fresh("t");
                    b.stmt(&format!("let {r} = &{v};"));
                    b.stmt(&format!("g(&mut {v});"));
                    b.stmt(&format!("let {t} = *{r};"));
                }
                2 => {
                    let a = b.fresh("a");
                    let c = b.fresh("c");
                    let t = b.fresh("t");
                    b.stmt(&format!("let {a} = &{v};"));
                    b.stmt(&format!("let {c} = &mut {v};"));
                    b.stmt(&format!("let {t} = *{a} + *{c};"));
                }
                3 => {
                    let r = b.fresh("r");
                    b.stmt(&format!("let {r} = &{v};"));
                    b.stmt(&format!("while *{r} > 3 {{ g(&mut {v}); }}"));
                }
                _ => {
                    let m = b.fresh("m");
                    b.stmt(&format!("let {m} = &mut {v};"));
                    b.stmt(&format!("*{m} = 1;"));
                    b.stmt(&format!("{v} = {v} + 1;"));
                    b.stmt(&format!("*{m} = 2;"));
                }
            }
        }
        R0Mode::ConflictMove => {
            // use-after-move:引用非 Copy,let-init 移動後再使用。
            let v = b.fresh("v");
            b.stmt(&format!("let mut {v} = {};", rng.below(9)));
            let m = b.fresh("m");
            b.stmt(&format!("let {m} = &mut {v};"));
            let m2 = b.fresh("m");
            b.stmt(&format!("let {m2} = {m};"));
            if rng.chance(1, 2) {
                b.stmt(&format!("*{m} = 1;"));
            } else {
                let t = b.fresh("t");
                b.stmt(&format!("let {t} = *{m};"));
            }
        }
    }

    b.out.push('}');
    let expect = match mode {
        R0Mode::Safe => R0Expect::Accept,
        R0Mode::ConflictBorrow => R0Expect::RejectBorrow,
        R0Mode::ConflictMove => R0Expect::RejectMove,
    };
    R0Sample { src: b.out, expect }
}
