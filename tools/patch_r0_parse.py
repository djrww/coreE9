# -*- coding: utf-8 -*-
# 在 src/r0.rs 的 #[cfg(test)] 之前插入 r0_parse(R₀ 完整解析器:CST + 節點級 unsupported)
CODE = r'''
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

/// R₀ 解析引擎的遞歸極限(機器界的如實申報;與 CL0' 的 RECURSION_LIMIT 同界)。
pub const R0_RECURSION_LIMIT: usize = 256;

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
    FnKw,
    StructKw,
    LetKw,
    MutKw,
    IfKw,
    ElseKw,
    WhileKw,
    LoopKw,
    ReturnKw,
    TrueKw,
    FalseKw,
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
    Slash,
    Percent,
    Ident,
    Number,
    RawString,
    Trivia,
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
    "trait", "impl", "use", "mod", "pub", "unsafe", "async", "match", "macro_rules", "dyn",
    "enum", "type", "static", "const", "extern", "where",
];

fn is_excluded_item_kw(w: &str) -> bool {
    EXCLUDED_ITEM_KWS.iter().any(|k| *k == w)
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
}

impl Absorb {
    fn stop_before(self, k: R0TokKind, kw_of_ident: Option<&str>) -> bool {
        match self {
            Absorb::Stmt => k == R0TokKind::RBrace,
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
    fn cur_kind(&self) -> Option<R0TokKind> {
        self.toks.get(self.pos).map(|t| t.kind)
    }
    fn cur_span(&self) -> Option<Span> {
        self.toks.get(self.pos).map(|t| t.span)
    }
    fn peek2_kind(&self) -> Option<R0TokKind> {
        self.toks.get(self.pos + 1).map(|t| t.kind)
    }
    fn peek3_kind(&self) -> Option<R0TokKind> {
        self.toks.get(self.pos + 2).map(|t| t.kind)
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
        while self.cur_kind() == Some(R0TokKind::Trivia) {
            let sp = self.cur_span().unwrap();
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
            Span { start: u32::MAX, end: 0 }
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
                .map(|t| t.span.start)
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

    fn close(&mut self) {
        let id = self.stack.pop().unwrap();
        self.depth -= 1;
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
                self.cur_text().filter(|w| is_excluded_item_kw(*w))
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
    fn stmt_err(&mut self, frame: usize, id: u32) {
        self.set_kind(id, R0Kind::Error);
        while self.stack.len() > frame + 1 {
            self.stack.pop();
            self.depth -= 1;
        }
        self.absorb_to(Absorb::Stmt);
        self.unwind_to(frame);
    }

    /// 一個項 = 一個原子錯誤區。
    fn item_err(&mut self, frame: usize, id: u32) {
        self.set_kind(id, R0Kind::Error);
        while self.stack.len() > frame + 1 {
            self.stack.pop();
            self.depth -= 1;
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
                Some(R0TokKind::Ident)
                    if self.cur_text().is_some_and(is_excluded_item_kw) =>
                {
                    self.unsupported_item()
                }
                _ => {
                    // 項層級的雜散 token:一項一錯誤區,吸收至下一項邊界。
                    let id = self.open(R0Kind::Error)?;
                    self.absorb_to(Absorb::Item);
                    self.unwind_to(self.stack.len());
                    let _ = id;
                    // absorb 後棧頂回到 Root 之上;關閉 Error 節點本身:
                    // open 已把它掛為 Root 的子節點,此處只需定稿。
                    self.finalize(id);
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
        self.parse_params()?;
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
                        self.parse_type()?;
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
                    self.parse_type()?;
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
                let id = self.open(R0Kind::Unsupported)?;
                self.set_note(id, self.bad_note());
                self.absorb_to(Absorb::Stmt);
                self.close();
                Ok(())
            }
            Some(R0TokKind::Ident) if self.cur_text() == Some("match") => {
                let id = self.open(R0Kind::Unsupported)?;
                self.set_note(id, "match 模式(排除)");
                self.absorb_to(Absorb::Stmt);
                self.close();
                Ok(())
            }
            _ => {
                let id = self.open(R0Kind::ExprStmt)?;
                if !self.expr_start() {
                    self.stmt_err(frame, id);
                    return Ok(());
                }
                match self.parse_expr() {
                    Ok(()) => {}
                    Err(R0ParseIssue::Depth) => return Err(R0ParseIssue::Depth),
                    Err(R0ParseIssue::Syntax) => {
                        self.stmt_err(frame, id);
                        return Ok(());
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
                Ok(()) => {}
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
                Ok(()) => {}
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
            Ok(()) => {}
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
            Ok(()) => {}
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

    fn expr_start(&self) -> bool {
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
    fn unsupported_wrap(&mut self, note: &'static str, mode: Absorb) {
        let id = self.open(R0Kind::Unsupported).unwrap_or_else(|_| {
            // Depth:退而求其次,直接吞到邊界(機器界如實申報的上限)。
            let id = self.nodes.len() as u32;
            self.nodes.push(R0Node {
                kind: R0Kind::Error,
                span: Span::new(0, 0),
                children: Vec::new(),
                note: Some("(深度界)"),
            });
            id
        });
        self.set_note(id, note);
        self.absorb_to(mode);
        self.close();
    }

    fn unsupported_item(&mut self) {
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
        self.unsupported_wrap(note, Absorb::Item);
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

    fn parse_expr(&mut self) -> Result<(), R0ParseIssue> {
        let id = self.open(R0Kind::Expr)?;
        self.parse_assign_body()?;
        self.close();
        let _ = id;
        Ok(())
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
        while self.cur_kind() == Some(R0TokKind::AndAnd) {
            self.bump(R0Kind::AndAnd)?;
            self.parse_and_body()?;
        }
        Ok(())
    }

    fn parse_and_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_eq_body()?;
        while self.cur_kind() == Some(R0TokKind::EqEq) || self.cur_kind() == Some(R0TokKind::NotEq) {
            let k = if self.cur_kind() == Some(R0TokKind::EqEq) {
                R0Kind::EqEq
            } else {
                R0Kind::NotEq
            };
            self.bump(k)?;
            self.parse_eq_body()?;
        }
        Ok(())
    }

    fn parse_eq_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_rel_body()?;
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
            self.parse_rel_body()?;
        }
        Ok(())
    }

    fn parse_rel_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_add_body()?;
        while matches!(self.cur_kind(), Some(R0TokKind::Plus) | Some(R0TokKind::Minus)) {
            let k = if self.cur_kind() == Some(R0TokKind::Plus) {
                R0Kind::Plus
            } else {
                R0Kind::Minus
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
            Some(R0TokKind::Star) | Some(R0TokKind::Slash) | Some(R0TokKind::Percent)
        ) {
            let k = match self.cur_kind().unwrap() {
                R0TokKind::Star => R0Kind::Star,
                R0TokKind::Slash => R0Kind::Slash,
                _ => R0Kind::Percent,
            };
            self.bump(k)?;
            self.parse_mul_body()?;
        }
        Ok(())
    }

    fn parse_mul_body(&mut self) -> Result<(), R0ParseIssue> {
        self.parse_unary()?;
        Ok(())
    }

    fn parse_unary(&mut self) -> Result<(), R0ParseIssue> {
        match self.cur_kind() {
            Some(R0TokKind::Amp) | Some(R0TokKind::AmpMut) | Some(R0TokKind::Star)
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
                    self.parse_expr()?;
                    if self.cur_kind() != Some(R0TokKind::RBrack) {
                        return Err(R0ParseIssue::Syntax);
                    }
                    self.bump(R0Kind::RBrack)?;
                }
                Some(R0TokKind::LParen) => {
                    self.bump(R0Kind::LParen)?;
                    loop {
                        if self.cur_kind() == Some(R0TokKind::RParen)
                            || self.cur_kind() == Some(R0TokKind::None)
                        {
                            break;
                        }
                        self.parse_expr()?;
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
                    self.unsupported_wrap("宏調用 `!`(排除)", Absorb::Stmt);
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
                self.parse_expr()?;
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
'''

path = 'src/r0.rs'
s = open(path, encoding='utf-8').read()
marker = '#[cfg(test)]\nmod tests {'
assert s.count(marker) == 1, s.count(marker)
# 模組交付清單更新
old_bullets = '''//!   4. `lalr1_clean(src)` —— LALR(1)-乾淨片段斷言(歧義構造不存在)。
'''
new_bullets = '''//!   4. `lalr1_clean(src)` —— LALR(1)-乾淨片段斷言(歧義構造不存在);
//!   5. `r0_parse(src)` —— 附錄 B EBNF → 表面語法樹(連續性/樹公理/laminar/
//!      無損回環 + 節點級 `unsupported` 申報,§9 如實申報的結構化形式)。
'''
assert s.count(old_bullets) == 1
s = s.replace(old_bullets, new_bullets)
# legacy 掃描關鍵字集擴充(與 EXCLUDED_ITEM_KWS 一致)
old_kws = '''    for kw in [
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
    ] {'''
new_kws = '''    for kw in [
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
    ] {'''
assert s.count(old_kws) == 1
s = s.replace(old_kws, new_kws)
# 補 legacy 掃描的標簽對(新增關鍵字)
old_tag = '''                        "macro_rules" => "macro_rules(排除)",
                        _ => "dyn(排除)",'''
new_tag = '''                        "macro_rules" => "macro_rules(排除)",
                        "enum" => "enum 項(排除)",
                        "type" => "type 項(排除)",
                        "static" => "static(排除)",
                        "const" => "const(排除)",
                        "extern" => "extern(排除)",
                        "where" => "where(排除)",
                        _ => "dyn(排除)",'''
assert s.count(old_tag) == 1
s = s.replace(old_tag, new_tag)
# 插入解析器主體
s = s.replace(marker, CODE + '\n' + marker)
open(path, 'w', encoding='utf-8').write(s)
print('ok src/r0.rs inserted, lines =', s.count('\n'))
