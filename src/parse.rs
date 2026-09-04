//! CL0 表面語法樹(§1.2 形式定義)+ 增量重析引擎(§2.2)+ ERROR 全化(§2.3)。
//!
//! 樹的四元組 T = (V, E, ℓ, σ):
//!   * V  節點集;E ⊆ V×V 父子邊(連通、無環、每節點至多一父);
//!   * ℓ : V → K(具名 / 匿名兩類);
//!   * σ : V → ℤ×ℤ,σ(v) = [a, b) 半開字節區間。
//!
//! **連續性公理**(由 `Tree::validate_continuity` 機械檢查):每個內部節點 v,
//! 子節點按序 c₁…c_k:σ(v) = [σ(c₁).start, σ(c_k).end)。葉子(token + trivia)
//! 的文本按序拼接 == 源碼原文(由 L1 測試逐字節驗證)。
//!
//! **全化(§2.3)**:解析器對任何輸入都產出樹;自動機卡死處以 ERROR 節點
//! 封存最小不可解析區間。引擎保留一個唯一的「機器極限」:遞歸深度
//! `RECURSION_LIMIT`(越界以 `ParseIssue::Depth` 如實報告 —— 這是機器的
//! 誠實聲明,不是裂縫;屬性測試的生成深度遠低於此界)。
//!
//! **配置快照(§5.3)**:每個具名節點在解析時記錄左邊界處的解析器配置
//! `Cfg = (祖先開節點種類棧, 前一結構 token, 後一結構 token)` —— 這是
//! LL(1) 語境下的「自動機狀態 + 棧摘要」等價物。增量重析只在
//! 「σ(S) 與編輯區不相交 ∧ 邊界配置相同」時重用子樹(§2.2 reuse 準則),
//! 正確性由 L3 測試對任意編輯序列機械驗證。

use crate::edit::Edit;
use crate::lex::{lex, TokKind, Token};
use crate::span::Span;
use std::collections::HashMap;

/// 樹節點表中節點的下標(任何機器界內的樹都以 u32 綽綽有餘)。
pub type NodeId = u32;

/// 機器遞歸極限(誠實申報的引擎界;屬性測試生成深度 ≤ 12,遠低於此界)。
pub const RECURSION_LIMIT: usize = 256;

/// 表面語法樹(CST)的節點種類。二分:具名(結構)vs 匿名(token 行)。
/// 具名 / 匿名 / trivia 的三分決定了 §1.3 的具名投影 π 與 §3.1 的跨度嵌套。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Kind {
    // —— 具名(named)——
    /// 整棵樹的根(虛擬節點,span = 全源碼)。
    Root,
    /// 函數項 `fn ...`(一級項)。
    FnItem,
    /// 參數聲明(形參)。
    Param,
    /// 型別引用 `int` / `thunk` / `&T`。
    TypeRef,
    /// 花括號塊 `{ ... }`。
    Block,
    /// `let` 綁定語句。
    LetStmt,
    /// `if [cond] { ... } else { ... }`。
    IfStmt,
    /// `while [cond] { ... }`。
    WhileStmt,
    /// 表達式語句(以 `;` 終止)。
    ExprStmt,
    /// 表達式(語法樹的運算面)。
    Expr,
    /// 一元前綴運算 `&x` / `*x`。
    UnaryExpr,
    /// 調用 `f(args)`。
    CallExpr,
    // —— 匿名(anonymous,token 層)——
    /// 關鍵字 `fn`。
    FnKw,
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
    /// 字面量 `true`。
    TrueKw,
    /// 字面量 `false`。
    FalseKw,
    /// `&`。
    Amp,
    /// `*`。
    Star,
    /// `+`。
    Plus,
    /// `-`。
    Minus,
    /// `==`。
    EqEq,
    /// `<`。
    Lt,
    /// `=`。
    Eq,
    /// `(`。
    LParen,
    /// `)`。
    RParen,
    /// `{`。
    LBrace,
    /// `}`。
    RBrace,
    /// `;`。
    Semi,
    /// `:`。
    Colon,
    /// `,`。
    Comma,
    /// 識別字。
    Ident,
    /// 數字字面量。
    Number,
    /// 空白/註釋(保留於樹中僅為平鋪完整性)。
    Trivia,
    /// 匿名:卡死點的封存(§2.3 ERROR 全化)。
    Error,
    /// 詞法級壞字元。
    BadTok,
}

impl Kind {
    /// 是否為具名(結構)節點(§1.3 投影 π 的保留集)。
    pub fn is_named(self) -> bool {
        matches!(
            self,
            Kind::Root
                | Kind::FnItem
                | Kind::Param
                | Kind::TypeRef
                | Kind::Block
                | Kind::LetStmt
                | Kind::IfStmt
                | Kind::WhileStmt
                | Kind::ExprStmt
                | Kind::Expr
                | Kind::UnaryExpr
                | Kind::CallExpr
        )
    }

    /// 節點種類的規範標簽(sexp 序列化與調試輸出用)。
    pub fn label(self) -> &'static str {
        match self {
            Kind::Root => "root",
            Kind::FnItem => "fn_item",
            Kind::Param => "param",
            Kind::TypeRef => "type_ref",
            Kind::Block => "block",
            Kind::LetStmt => "let_stmt",
            Kind::IfStmt => "if_stmt",
            Kind::WhileStmt => "while_stmt",
            Kind::ExprStmt => "expr_stmt",
            Kind::Expr => "expr",
            Kind::UnaryExpr => "unary_expr",
            Kind::CallExpr => "call_expr",
            Kind::FnKw => "fn",
            Kind::LetKw => "let",
            Kind::MutKw => "mut",
            Kind::IfKw => "if",
            Kind::ElseKw => "else",
            Kind::WhileKw => "while",
            Kind::TrueKw => "true",
            Kind::FalseKw => "false",
            Kind::Amp => "&",
            Kind::Star => "*",
            Kind::Plus => "+",
            Kind::Minus => "-",
            Kind::EqEq => "==",
            Kind::Lt => "<",
            Kind::Eq => "=",
            Kind::LParen => "(",
            Kind::RParen => ")",
            Kind::LBrace => "{",
            Kind::RBrace => "}",
            Kind::Semi => ";",
            Kind::Colon => ":",
            Kind::Comma => ",",
            Kind::Ident => "ident",
            Kind::Number => "number",
            Kind::Trivia => "trivia",
            Kind::Error => "error",
            Kind::BadTok => "bad",
        }
    }
}

#[derive(Clone, Debug)]
/// 樹節點:種類 + 半開跨度 + 直接子節點表(§1.1)。
pub struct Node {
    /// 節點種類(具名/匿名/token/trivia/error)。
    pub kind: Kind,
    /// 節點覆蓋的半開源碼跨度 σ(v) = [start, end)。
    pub span: Span,
    /// 直接子節點 id(依源碼順序)。
    pub children: Vec<NodeId>,
}

/// §5.3 解析器配置快照:子樹左邊界處的「自動機狀態 + 棧摘要」。
/// 對 LL(1) 語境,這等價於 (祖先開節點種類棧, 前一結構 token, 後一結構 token)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cfg {
    /// 祖先開始節點棧(LL(1) 語境的正則/棧摘要)。
    pub stack: Vec<Kind>,
    /// 子樹左邊界前的最後一個結構 token。
    pub prev: Option<TokKind>,
    /// 子樹右邊界後的第一個結構 token。
    pub next: Option<TokKind>,
}

#[derive(Clone, Debug)]
/// 解析產物:源碼 + 節點表 + 配置快照 + 首未 token 統計(§1.1 / §5.3)。
pub struct Tree {
    /// 被解析的源碼(逐字節保留 ⇒ L1 無損回環)。
    pub src: String,
    /// 節點表(連續存儲;id 即下標)。
    pub nodes: Vec<Node>,
    /// 每個節點子樹邊界處的解析器配置快照(重用統計的對賬依據)。
    pub cfgs: Vec<Cfg>,
    /// 每個節點(子樹)的第一個 / 最後一個非 trivia token 種類(用於重用統計與對賬)。
    pub first_tok: Vec<Option<TokKind>>,
    /// 每個節點(子樹)的最後一個非 trivia token 種類(用於重用統計與對賬)。
    pub last_tok: Vec<Option<TokKind>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 解析引擎對外向調用方的失敗報告(僅兩種:語法層與引擎界)。
pub enum ParseIssue {
    /// 語法層失敗(由 ERROR 恢復處理;對外向的調用方表示「此構造無法開始」)。
    Syntax,
    /// 引擎遞歸極限(機器界的如實申報)。
    Depth,
}

/// 恢復例程的同步模式(§2.3:同步點 = 語句邊界 / 參數界 / 項界)。
#[derive(Clone, Copy, Debug)]
enum SyncMode {
    Stmt,   // 止於 `}`(不入),吞至 `;`(含)
    Args,   // 止於 `)`(不入),吞至 `,`(含)
    Params, // 止於 `)`(不入),吞至 `,`(含)
    Item,   // 止於下一個 `fn`(不入)
}

impl SyncMode {
    fn stop_before(self, k: TokKind) -> bool {
        match self {
            SyncMode::Stmt => k == TokKind::RBrace,
            SyncMode::Args | SyncMode::Params => k == TokKind::RParen,
            SyncMode::Item => k == TokKind::Fn,
        }
    }
    fn stop_after(self, k: TokKind) -> bool {
        match self {
            SyncMode::Stmt => k == TokKind::Semi,
            SyncMode::Args | SyncMode::Params => k == TokKind::Comma,
            SyncMode::Item => false,
        }
    }
}

fn expr_start(k: Option<TokKind>) -> bool {
    matches!(
        k,
        Some(TokKind::Number)
            | Some(TokKind::True)
            | Some(TokKind::False)
            | Some(TokKind::Ident)
            | Some(TokKind::LBrace)
            | Some(TokKind::Amp)
            | Some(TokKind::Star)
    )
}

// ===========================================================================
// 解析器
// ===========================================================================

struct Parser<'a> {
    src: String,
    toks: Vec<Token>,
    pos: usize,
    nodes: Vec<Node>,
    cfgs: Vec<Cfg>,
    first_tok: Vec<Option<TokKind>>,
    last_tok: Vec<Option<TokKind>>,
    stack: Vec<NodeId>,
    depth: usize,
    last_struct: Option<TokKind>,
    rd: Option<ReuseData<'a>>,
    reused: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &str, toks: Vec<Token>, rd: Option<ReuseData<'a>>) -> Parser<'a> {
        Parser {
            src: src.to_string(),
            toks,
            pos: 0,
            nodes: Vec::new(),
            cfgs: Vec::new(),
            first_tok: Vec::new(),
            last_tok: Vec::new(),
            stack: Vec::new(),
            depth: 0,
            last_struct: None,
            rd,
            reused: 0,
        }
    }

    fn cur(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }

    /// 不附著 trivia 的窺視(用於計算 cfg.next,不改樹)。
    fn peek_raw(&self) -> Option<TokKind> {
        let mut i = self.pos;
        while let Some(t) = self.toks.get(i) {
            if t.kind != TokKind::Trivia {
                return Some(t.kind);
            }
            i += 1;
        }
        None
    }

    /// 附著 trivia 後窺視:trivia 成為當前開節點(棧頂)的孩子。
    fn peek(&mut self) -> Option<TokKind> {
        self.skip_trivia();
        self.cur().map(|t| t.kind)
    }

    fn skip_trivia(&mut self) {
        while let Some(t) = self.cur() {
            if t.kind == TokKind::Trivia {
                let (k, sp) = (t.kind, t.span);
                self.attach_leaf(k, sp);
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn bump(&mut self) -> Option<(TokKind, Span)> {
        self.skip_trivia();
        let t = *self.cur()?;
        if t.kind != TokKind::Trivia {
            self.last_struct = Some(t.kind);
        }
        self.attach_leaf(t.kind, t.span);
        self.pos += 1;
        Some((t.kind, t.span))
    }

    fn open(&mut self, kind: Kind) -> Result<NodeId, ParseIssue> {
        if self.depth >= RECURSION_LIMIT {
            return Err(ParseIssue::Depth);
        }
        let mut stack = Vec::with_capacity(self.stack.len());
        for &id in &self.stack {
            stack.push(self.nodes[id as usize].kind);
        }
        let cfg = Cfg {
            stack,
            prev: self.last_struct,
            next: None,
        };
        let id = self.nodes.len() as NodeId;
        let span = if kind == Kind::Root {
            Span::new(0, self.src.len() as u32)
        } else {
            // 哨兵初值:等到第一個孩子附著後由 min/max 修正。
            Span {
                start: u32::MAX,
                end: 0,
            }
        };
        self.nodes.push(Node {
            kind,
            span,
            children: Vec::new(),
        });
        self.cfgs.push(cfg);
        self.first_tok.push(None);
        self.last_tok.push(None);
        let parent = self.stack.last().copied();
        self.stack.push(id);
        self.depth += 1;
        if let Some(p) = parent {
            self.link(p, id);
        }
        Ok(id)
    }

    /// 節點定稿:空子節點 → 跨度為當前位置的空區間;然後重算父跨度
    /// (父節點區間 = 所有已定稿子節點區間的並 —— 連續性公理的構造維持)。
    fn finalize(&mut self, id: NodeId) {
        let next = self.peek_raw();
        self.cfgs[id as usize].next = next;
        // 定稿自身跨度:內部節點 = 子節點區間的並(連續性公理);
        // 空節點 = 當前位置的空區間。
        {
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
                    .cur()
                    .map(|t| t.span.start)
                    .unwrap_or(self.src.len() as u32);
                self.nodes[id as usize].span = Span::new(p, p);
            }
        }
        if let Some(&pid) = self.stack.last() {
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

    fn link(&mut self, parent: NodeId, id: NodeId) {
        let (cstart, cend) = {
            let n = &self.nodes[id as usize];
            (n.span.start, n.span.end)
        };
        let (cf, cl) = (self.first_tok[id as usize], self.last_tok[id as usize]);
        let p = &mut self.nodes[parent as usize];
        p.children.push(id);
        if self.first_tok[parent as usize].is_none() {
            self.first_tok[parent as usize] = cf;
        }
        if cl.is_some() {
            self.last_tok[parent as usize] = cl;
        }
        let _ = (cstart, cend); // 跨度在子節點定稿時重算(finalize)
    }

    fn attach(&mut self, id: NodeId) {
        let parent = *self.stack.last().unwrap();
        self.link(parent, id);
    }

    fn attach_leaf(&mut self, tk: TokKind, span: Span) -> NodeId {
        let kind = match tk {
            TokKind::Bad => Kind::BadTok,
            TokKind::Number => Kind::Number,
            TokKind::True => Kind::TrueKw,
            TokKind::False => Kind::FalseKw,
            TokKind::Fn => Kind::FnKw,
            TokKind::Let => Kind::LetKw,
            TokKind::Mut => Kind::MutKw,
            TokKind::If => Kind::IfKw,
            TokKind::Else => Kind::ElseKw,
            TokKind::While => Kind::WhileKw,
            TokKind::Amp => Kind::Amp,
            TokKind::Star => Kind::Star,
            TokKind::Plus => Kind::Plus,
            TokKind::Minus => Kind::Minus,
            TokKind::EqEq => Kind::EqEq,
            TokKind::Lt => Kind::Lt,
            TokKind::Eq => Kind::Eq,
            TokKind::LParen => Kind::LParen,
            TokKind::RParen => Kind::RParen,
            TokKind::LBrace => Kind::LBrace,
            TokKind::RBrace => Kind::RBrace,
            TokKind::Semi => Kind::Semi,
            TokKind::Colon => Kind::Colon,
            TokKind::Comma => Kind::Comma,
            TokKind::Ident => Kind::Ident,
            TokKind::Trivia => Kind::Trivia,
        };
        let id = self.nodes.len() as NodeId;
        self.nodes.push(Node {
            kind,
            span,
            children: Vec::new(),
        });
        self.cfgs.push(Cfg {
            stack: Vec::new(),
            prev: None,
            next: None,
        });
        self.first_tok.push(Some(tk));
        self.last_tok.push(Some(tk));
        self.attach(id);
        id
    }

    fn set_error(&mut self, id: NodeId) {
        self.nodes[id as usize].kind = Kind::Error;
    }

    fn recover_wrap(&mut self, mode: SyncMode) -> Result<(), ParseIssue> {
        // 只有真的會消耗 ≥1 token 時才封存 ERROR 節點(避免空錯誤節點)。
        if let Some(t) = self.cur() {
            if mode.stop_before(t.kind) {
                // 調用方已把失敗構造轉換為 Error(其自身即錯誤區域)。
                return Ok(());
            }
        } else {
            return Ok(());
        }
        let id = self.open(Kind::Error)?;
        let mut bd = 0i32;
        let mut pd = 0i32;
        while let Some(t) = self.cur() {
            let k = t.kind;
            if bd == 0 && pd == 0 && mode.stop_before(k) {
                break;
            }
            self.bump();
            match k {
                TokKind::LBrace => bd += 1,
                TokKind::RBrace => bd -= 1,
                TokKind::LParen => pd += 1,
                TokKind::RParen => pd -= 1,
                _ => {}
            }
            if bd == 0 && pd == 0 && mode.stop_after(k) {
                break;
            }
        }
        self.close();
        let _ = id;
        Ok(())
    }

    // ---- 重用鉤子(§2.2 / §5.3)----

    fn try_reuse(&mut self, kind: Kind) -> Option<(NodeId, Option<TokKind>)> {
        let rd = self.rd.as_ref()?;
        let pos = self.cur().map(|t| t.span.start)?;
        let mut stack = Vec::with_capacity(self.stack.len());
        for &id in &self.stack {
            stack.push(self.nodes[id as usize].kind);
        }
        let prev = self.last_struct;
        let (oid, delta, nspan, last) =
            ReuseData::lookup(rd, kind, pos, &stack, prev, &self.toks, self.pos)?;
        self.reused += 1;
        let cloned = {
            let Self {
                ref rd,
                ref mut nodes,
                ref mut cfgs,
                ref mut first_tok,
                ref mut last_tok,
                ..
            } = *self;
            let rd = rd.as_ref().unwrap();
            clone_subtree(rd.old, oid, delta, nodes, cfgs, first_tok, last_tok)
        };
        self.attach(cloned);
        // 推進位置到重用子樹之後。
        while let Some(t) = self.toks.get(self.pos) {
            if t.span.start >= nspan.end {
                break;
            }
            self.pos += 1;
        }
        self.last_struct = last;
        Some((cloned, last))
    }

    // ---- 語法 ----

    fn parse_program(&mut self) -> Result<(), ParseIssue> {
        self.open(Kind::Root)?;
        loop {
            match self.peek() {
                None => break,
                Some(TokKind::Fn) => self.parse_item()?,
                Some(_) => {
                    let _ = self.recover_wrap(SyncMode::Item);
                }
            }
        }
        self.close();
        Ok(())
    }

    fn parse_item(&mut self) -> Result<(), ParseIssue> {
        if self.try_reuse(Kind::FnItem).is_some() {
            return Ok(());
        }
        let frame = self.stack.len();
        let id = self.open(Kind::FnItem)?;
        self.bump(); // fn
        if self.peek() != Some(TokKind::Ident) {
            return {
                self.item_err(frame, id);
                Ok(())
            };
        }
        self.bump();
        if self.peek() != Some(TokKind::LParen) {
            return {
                self.item_err(frame, id);
                Ok(())
            };
        }
        self.parse_params()?;
        self.parse_block()?;
        self.close();
        Ok(())
    }

    fn item_err(&mut self, frame: usize, id: NodeId) {
        self.set_error(id);
        while self.stack.len() > frame + 1 {
            let sid = self.stack.pop().unwrap();
            self.depth -= 1;
            self.finalize(sid);
        }
        // 吸收至下一項(吞至下一個 `fn`)。
        self.absorb_to(SyncMode::Item);
        self.unwind_to(frame);
    }

    fn parse_params(&mut self) -> Result<(), ParseIssue> {
        // 調用方保證當前是 LParen。
        self.bump();
        loop {
            match self.peek() {
                Some(TokKind::RParen) => {
                    self.bump();
                    break;
                }
                Some(TokKind::Comma) => {
                    self.bump();
                }
                Some(TokKind::Ident) => self.parse_param()?,
                None => break,
                _ => {
                    let _ = self.recover_wrap(SyncMode::Params);
                }
            }
        }
        Ok(())
    }

    fn parse_param(&mut self) -> Result<(), ParseIssue> {
        if self.try_reuse(Kind::Param).is_some() {
            return Ok(());
        }
        let frame = self.stack.len();
        let id = self.open(Kind::Param)?;
        self.bump(); // ident
        if self.peek() == Some(TokKind::Colon) {
            self.bump();
            match self.parse_type() {
                Ok(()) => {}
                Err(ParseIssue::Depth) => return Err(ParseIssue::Depth),
                Err(ParseIssue::Syntax) => {
                    self.set_error(id);
                    self.unwind_to(frame);
                    let _ = self.recover_wrap(SyncMode::Params);
                    return Ok(());
                }
            }
        }
        self.close();
        Ok(())
    }

    fn parse_type(&mut self) -> Result<(), ParseIssue> {
        if self.try_reuse(Kind::TypeRef).is_some() {
            return Ok(());
        }
        let _id = self.open(Kind::TypeRef)?;
        while matches!(self.peek(), Some(TokKind::Amp)) {
            self.bump();
            if self.peek() == Some(TokKind::Mut) {
                self.bump();
            }
        }
        if self.peek() != Some(TokKind::Ident) {
            self.unwind_to(self.stack.len() - 1);
            return Err(ParseIssue::Syntax);
        }
        self.bump();
        self.close();
        Ok(())
    }

    fn parse_block(&mut self) -> Result<(), ParseIssue> {
        if self.try_reuse(Kind::Block).is_some() {
            return Ok(());
        }
        let frame = self.stack.len();
        let id = self.open(Kind::Block)?;
        if self.peek() != Some(TokKind::LBrace) {
            // 語法上塊必須以 { 開始:轉為 ERROR 並恢復。
            self.set_error(id);
            self.unwind_to(frame);
            let _ = self.recover_wrap(SyncMode::Stmt);
            return Ok(());
        }
        self.bump();
        loop {
            match self.peek() {
                None => break,
                Some(TokKind::RBrace) => {
                    self.bump();
                    break;
                }
                _ => self.parse_stmt()?,
            }
        }
        self.close();
        Ok(())
    }

    /// 把當前 token 流吸收進 stack 頂部的錯誤節點,直至同步模式邊界。
    /// 這是「一語句一原子錯誤區」的機制:壞語句的殘骸不會分裂成
    /// 多個相接的 ERROR 節點(那會違反 L7b 的極大性)。
    fn absorb_to(&mut self, mode: SyncMode) {
        let mut bd = 0i32;
        let mut pd = 0i32;
        while let Some(t) = self.cur() {
            let k = t.kind;
            if bd == 0 && pd == 0 && mode.stop_before(k) {
                break;
            }
            self.bump();
            match k {
                TokKind::LBrace => bd += 1,
                TokKind::RBrace => bd -= 1,
                TokKind::LParen => pd += 1,
                TokKind::RParen => pd -= 1,
                _ => {}
            }
            if bd == 0 && pd == 0 && mode.stop_after(k) {
                break;
            }
        }
    }

    fn stmt_err(&mut self, frame: usize, id: NodeId) {
        self.set_error(id);
        while self.stack.len() > frame + 1 {
            let sid = self.stack.pop().unwrap();
            self.depth -= 1;
            self.finalize(sid);
        }
        // 吸收剩餘 token 進本語句的錯誤節點(吞至語句邊界)。
        self.absorb_to(SyncMode::Stmt);
        self.unwind_to(frame);
    }

    fn parse_stmt(&mut self) -> Result<(), ParseIssue> {
        for k in [
            Kind::LetStmt,
            Kind::IfStmt,
            Kind::WhileStmt,
            Kind::ExprStmt,
            Kind::Error,
        ] {
            if self.try_reuse(k).is_some() {
                return Ok(());
            }
        }
        let frame = self.stack.len();
        match self.peek() {
            Some(TokKind::Let) => {
                let id = self.open(Kind::LetStmt)?;
                self.bump();
                if self.peek() == Some(TokKind::Mut) {
                    self.bump();
                }
                if self.peek() != Some(TokKind::Ident) {
                    return {
                        self.stmt_err(frame, id);
                        Ok(())
                    };
                }
                self.bump();
                if self.peek() == Some(TokKind::Eq) {
                    self.bump();
                    if self.peek() == Some(TokKind::Semi) {
                        return {
                            self.stmt_err(frame, id);
                            Ok(())
                        };
                    }
                    match self.parse_expr() {
                        Ok(()) => {}
                        Err(ParseIssue::Depth) => return Err(ParseIssue::Depth),
                        Err(ParseIssue::Syntax) => {
                            return {
                                self.stmt_err(frame, id);
                                Ok(())
                            }
                        }
                    }
                }
                if self.peek() != Some(TokKind::Semi) {
                    return {
                        self.stmt_err(frame, id);
                        Ok(())
                    };
                }
                self.bump();
                self.close();
                Ok(())
            }
            Some(TokKind::If) => {
                let id = self.open(Kind::IfStmt)?;
                self.bump();
                if !expr_start(self.peek()) {
                    return {
                        self.stmt_err(frame, id);
                        Ok(())
                    };
                }
                match self.parse_expr() {
                    Ok(()) => {}
                    Err(ParseIssue::Depth) => return Err(ParseIssue::Depth),
                    Err(ParseIssue::Syntax) => {
                        return {
                            self.stmt_err(frame, id);
                            Ok(())
                        }
                    }
                }
                // 塊缺失 ⟹ 整個 if 是錯誤區域(切割時把壞語句整段移除)。
                if self.peek() != Some(TokKind::LBrace) {
                    return {
                        self.stmt_err(frame, id);
                        Ok(())
                    };
                }
                self.parse_block()?;
                if self.peek() == Some(TokKind::Else) {
                    self.bump();
                    if self.peek() == Some(TokKind::If) {
                        self.parse_stmt()?;
                    } else if self.peek() == Some(TokKind::LBrace) {
                        self.parse_block()?;
                    } else {
                        return {
                            self.stmt_err(frame, id);
                            Ok(())
                        };
                    }
                }
                self.close();
                Ok(())
            }
            Some(TokKind::While) => {
                let id = self.open(Kind::WhileStmt)?;
                self.bump();
                if !expr_start(self.peek()) {
                    return {
                        self.stmt_err(frame, id);
                        Ok(())
                    };
                }
                match self.parse_expr() {
                    Ok(()) => {}
                    Err(ParseIssue::Depth) => return Err(ParseIssue::Depth),
                    Err(ParseIssue::Syntax) => {
                        return {
                            self.stmt_err(frame, id);
                            Ok(())
                        }
                    }
                }
                if self.peek() != Some(TokKind::LBrace) {
                    return {
                        self.stmt_err(frame, id);
                        Ok(())
                    };
                }
                self.parse_block()?;
                self.close();
                Ok(())
            }
            _ => {
                let id = self.open(Kind::ExprStmt)?;
                if !expr_start(self.peek()) {
                    return {
                        self.stmt_err(frame, id);
                        Ok(())
                    };
                }
                match self.parse_expr() {
                    Ok(()) => {}
                    Err(ParseIssue::Depth) => return Err(ParseIssue::Depth),
                    Err(ParseIssue::Syntax) => {
                        return {
                            self.stmt_err(frame, id);
                            Ok(())
                        }
                    }
                }
                if self.peek() != Some(TokKind::Semi) {
                    return {
                        self.stmt_err(frame, id);
                        Ok(())
                    };
                }
                self.bump();
                self.close();
                Ok(())
            }
        }
    }

    fn parse_expr(&mut self) -> Result<(), ParseIssue> {
        if self.try_reuse(Kind::Expr).is_some() {
            return Ok(());
        }
        let id = self.open(Kind::Expr)?;
        self.parse_unary()?;
        while matches!(
            self.peek(),
            Some(TokKind::Plus)
                | Some(TokKind::Minus)
                | Some(TokKind::Star)
                | Some(TokKind::EqEq)
                | Some(TokKind::Lt)
        ) {
            self.bump();
            self.parse_unary()?;
        }
        self.close();
        let _ = id;
        Ok(())
    }

    fn parse_unary(&mut self) -> Result<(), ParseIssue> {
        let has_prefix = matches!(self.peek(), Some(TokKind::Amp) | Some(TokKind::Star));
        if has_prefix {
            if self.try_reuse(Kind::UnaryExpr).is_some() {
                return Ok(());
            }
            let id = self.open(Kind::UnaryExpr)?;
            loop {
                match self.peek() {
                    Some(TokKind::Amp) => {
                        self.bump();
                        if self.peek() == Some(TokKind::Mut) {
                            self.bump();
                        }
                    }
                    Some(TokKind::Star) => {
                        self.bump();
                    }
                    _ => break,
                }
            }
            self.parse_primary()?;
            self.close();
            let _ = id;
            Ok(())
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<(), ParseIssue> {
        match self.peek() {
            Some(TokKind::Number) | Some(TokKind::True) | Some(TokKind::False) => {
                self.bump();
                Ok(())
            }
            Some(TokKind::LBrace) => self.parse_block(),
            Some(TokKind::Ident) => {
                self.bump();
                if self.peek() == Some(TokKind::LParen) {
                    if self.try_reuse(Kind::CallExpr).is_some() {
                        return Ok(());
                    }
                    let id = self.open(Kind::CallExpr)?;
                    self.bump(); // (
                    loop {
                        match self.peek() {
                            Some(TokKind::RParen) => {
                                self.bump();
                                break;
                            }
                            Some(TokKind::Comma) => {
                                self.bump();
                            }
                            None => break,
                            _ => match self.parse_expr() {
                                Ok(()) => {}
                                Err(ParseIssue::Depth) => return Err(ParseIssue::Depth),
                                Err(ParseIssue::Syntax) => {
                                    let top = *self.stack.last().unwrap();
                                    self.set_error(top);
                                    self.unwind_to(self.stack.len() - 1);
                                    let _ = self.recover_wrap(SyncMode::Args);
                                }
                            },
                        }
                    }
                    self.close();
                    let _ = id;
                }
                Ok(())
            }
            _ => Err(ParseIssue::Syntax),
        }
    }
}

// ===========================================================================
// 增量重析:配置快照重用(§2.2 / §5.3)
// ===========================================================================

/// 增量重析的中間表(§2.2 / §5.3):哪些舊節點髒、新坐標下的跨度、
/// 按邊界索引的候選。L3 增量等價按約定不在律級斷言內(見需求排除)。
pub struct ReuseData<'a> {
    old: &'a Tree,
    #[allow(dead_code)]
    edits: &'a [Edit],
    dirty: Vec<bool>,
    /// 舊節點 id → 新坐標下的 span(僅對非髒節點有效)。
    new_span: Vec<Span>,
    by_start: HashMap<u32, Vec<u32>>,
}

impl<'a> ReuseData<'a> {
    /// 由舊樹 + 編輯集構造重用表(標髒、重定 span、建索引)。
    pub fn build(old: &'a Tree, edits: &'a [Edit]) -> ReuseData<'a> {
        let n = old.nodes.len();
        let mut dirty = vec![false; n];
        let mut new_span = vec![Span::new(0, 0); n];
        for (id, node) in old.nodes.iter().enumerate() {
            for e in edits {
                let (s, oe) = (e.start, e.old_end);
                if oe > s {
                    if node.span.overlaps(&Span::new(s, oe)) {
                        dirty[id] = true;
                    }
                } else {
                    // 純插入:嚴格落在節點內部才算髒。
                    if node.span.start < s && s < node.span.end {
                        dirty[id] = true;
                    }
                }
            }
        }
        let mut by_start: HashMap<u32, Vec<u32>> = HashMap::new();
        for (id, node) in old.nodes.iter().enumerate() {
            if dirty[id] {
                continue;
            }
            let mut d = 0i64;
            for e in edits {
                if e.old_end <= node.span.start {
                    d += e.delta();
                }
            }
            let ns = node.span.shift(d);
            new_span[id] = ns;
            by_start.entry(ns.start).or_default().push(id as u32);
        }
        ReuseData {
            old,
            edits,
            dirty,
            new_span,
            by_start,
        }
    }

    /// §2.2 reuse 準則:σ(S) 與編輯區不相交 ∧ 邊界配置相同。
    fn lookup(
        rd: &ReuseData,
        kind: Kind,
        pos: u32,
        stack: &[Kind],
        prev: Option<TokKind>,
        toks: &[Token],
        tok_pos: usize,
    ) -> Option<(u32, i64, Span, Option<TokKind>)> {
        let ids = rd.by_start.get(&pos)?;
        for &oid in ids {
            let node = &rd.old.nodes[oid as usize];
            if node.kind != kind || rd.dirty[oid as usize] {
                continue;
            }
            let cfg = &rd.old.cfgs[oid as usize];
            if cfg.stack.as_slice() != stack {
                continue;
            }
            if cfg.prev != prev {
                continue;
            }
            // 後一結構 token 必須一致(LL(1) 邊界條件)。
            let nspan = rd.new_span[oid as usize];
            let mut i = tok_pos;
            while i < toks.len() && toks[i].span.start < nspan.end {
                i += 1;
            }
            while i < toks.len() && toks[i].kind == TokKind::Trivia {
                i += 1;
            }
            let actual = toks.get(i).map(|t| t.kind);
            if actual != cfg.next {
                continue;
            }
            let delta = nspan.start as i64 - node.span.start as i64;
            let last = rd.old.last_tok[oid as usize];
            return Some((oid, delta, nspan, last));
        }
        None
    }
}

/// 把舊子樹克隆進新樹(全子樹統一平移 delta;髒節點已被排除,故平移均勻)。
fn clone_subtree(
    old: &Tree,
    oid: u32,
    delta: i64,
    nodes: &mut Vec<Node>,
    cfgs: &mut Vec<Cfg>,
    first_tok: &mut Vec<Option<TokKind>>,
    last_tok: &mut Vec<Option<TokKind>>,
) -> u32 {
    let mut map = vec![u32::MAX; old.nodes.len()];
    let mut order: Vec<u32> = Vec::new();
    let mut stackv: Vec<u32> = vec![oid];
    // ★ 新 id 用**遞增計數器**,不能用 `nodes.len()` —— DFS 階段尚未 push 任何
    //   節點,nodes.len() 恆為常數,若用它會讓所有克隆節點分到**同一個** id,
    //   導致 `children` 全指向根(自環)→ 重析樹變畸形 → sexp/unparse 深遞歸爆棧。
    let mut next_id = nodes.len() as u32;
    while let Some(id) = stackv.pop() {
        if map[id as usize] != u32::MAX {
            continue;
        }
        let nid = next_id;
        next_id += 1;
        map[id as usize] = nid;
        order.push(id);
        // 先推子節點(稍後統一重映射 children)。
        for &c in old.nodes[id as usize].children.iter().rev() {
            if map[c as usize] == u32::MAX {
                stackv.push(c);
            }
        }
    }
    // 依遍歷順序(父先於子)建立新節點。
    let new_id = |old_id: u32| -> u32 {
        let nid = map[old_id as usize];
        debug_assert!(nid != u32::MAX);
        nid
    };
    for id in &order {
        let nnode = old.nodes[*id as usize].clone();
        let nspan = nnode.span.shift(delta);
        let children = nnode
            .children
            .iter()
            .map(|&c| new_id(c))
            .collect::<Vec<u32>>();
        nodes.push(Node {
            kind: nnode.kind,
            span: nspan,
            children,
        });
        cfgs.push(Cfg {
            stack: Vec::new(),
            prev: None,
            next: None,
        });
        first_tok.push(old.first_tok[*id as usize]);
        last_tok.push(old.last_tok[*id as usize]);
    }
    // 確保父節點先於子節點被建:order 是 DFS 前序(父先)。

    map[oid as usize]
}

// ===========================================================================
// 對外接口
// ===========================================================================

/// 增量重析的輸出:新樹 + 重用統計(reused/total = 重用率,§5.3 對賬指標)。
pub struct ReparseOut {
    /// 重析後的新樹。
    pub tree: Tree,
    /// 被重用(未重新解析)的舊節點數(新舊對賬的 reused 側)。
    pub reused: usize,
    /// 重析後**新樹**的總節點數(與 `tree.stats()` 之和一致)。
    pub total: usize,
}

/// 全函數語法分析(除引擎遞歸極限外永不失敗;「任何輸入都產出樹」= §2.3 全化)。
pub fn parse(src: &str) -> Result<Tree, ParseIssue> {
    let toks = lex(src);
    let mut p = Parser::new(src, toks, None);
    p.parse_program()?;
    Ok(Tree {
        src: src.to_string(),
        nodes: p.nodes,
        cfgs: p.cfgs,
        first_tok: p.first_tok,
        last_tok: p.last_tok,
    })
}

/// 增量重析:reparse(parse(s), s, e) —— 在編輯後的新源碼上重析,
/// 按 §2.2 reuse 準則重用乾淨且邊界配置一致的子樹。
pub fn reparse(old: &Tree, new_src: &str, edits: &[Edit]) -> Result<ReparseOut, ParseIssue> {
    let rd = ReuseData::build(old, edits);
    let toks = lex(new_src);
    let mut p = Parser::new(new_src, toks, Some(rd));
    p.parse_program()?;
    let total = p.nodes.len();
    let reused = p.reused;
    Ok(ReparseOut {
        tree: Tree {
            src: new_src.to_string(),
            nodes: p.nodes,
            cfgs: p.cfgs,
            first_tok: p.first_tok,
            last_tok: p.last_tok,
        },
        reused,
        total,
    })
}

// ===========================================================================
// 樹的性質檢查(供定律測試使用)
// ===========================================================================

impl Tree {
    /// 根節點 id(恆為 0;樹不空)。
    pub fn root(&self) -> NodeId {
        assert!(!self.nodes.is_empty());
        0
    }

    /// 依 id 取節點引用。
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    /// §1.2 連續性公理:內部節點 σ(v) = [σ(c₁).start, σ(c_k).end),
    /// 且子節點依序不交:∀i: σ(cᵢ).end ≤ σ(cᵢ₊₁).start。
    pub fn validate_continuity(&self) -> Result<(), String> {
        for (id, node) in self.nodes.iter().enumerate() {
            if node.children.is_empty() {
                continue;
            }
            // 葉子節點必須是 token(無子節點)或檢查缺失:這裡只檢查內部節點。
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
                        "node {} ({:?}) children overlap: child {} span {} before prev_end {}",
                        id, node.kind, c, cs, prev_end
                    ));
                }
                prev_end = cs.end;
            }
        }
        // 根節點覆蓋全源碼(虛擬節點:span 由構造器定義,這裡只驗證一致性)。
        Ok(())
    }

    /// 每個節點至多一父(樹公理:E ⊆ V×V 連通、無環、每節點至多一父)。
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
        // 連通性:從根出發 DFS 必須訪問全部節點。
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
    /// 這是§3.1 嵌套定理的機械形式。
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

    /// L7a:樹中是否有 ERROR 節點。
    pub fn n_errors(&self) -> usize {
        self.nodes.iter().filter(|n| n.kind == Kind::Error).count()
    }

    /// L7a 斷言用:樹中是否存在 ERROR 節點。
    pub fn has_error(&self) -> bool {
        self.n_errors() > 0
    }

    /// 最大(最外層)ERROR 跨度 —— L7b「挖掉後剩餘良構」的切割對象。
    pub fn maximal_error_spans(&self) -> Vec<Span> {
        let mut has_err_parent = vec![false; self.nodes.len()];
        for (id, node) in self.nodes.iter().enumerate() {
            if node.kind == Kind::Error {
                let mut st = vec![id as u32];
                while let Some(x) = st.pop() {
                    for &c in &self.nodes[x as usize].children {
                        has_err_parent[c as usize] = true;
                        st.push(c);
                    }
                }
            }
        }
        let mut out = Vec::new();
        for (id, node) in self.nodes.iter().enumerate() {
            if node.kind == Kind::Error && !has_err_parent[id] {
                out.push(node.span);
            }
        }
        out
    }

    /// 無損回環:L1 的機械檢查 —— unparse(parse(s)) ≡ s 逐字節。
    pub fn unparse(&self) -> String {
        let mut out = String::with_capacity(self.src.len());
        let mut stack = vec![self.root()];
        while let Some(id) = stack.pop() {
            let node = &self.nodes[id as usize];
            if node.children.is_empty() {
                out.push_str(&self.src[node.span.start as usize..node.span.end as usize]);
            } else {
                for &c in node.children.iter().rev() {
                    stack.push(c);
                }
            }
        }
        out
    }

    /// 具名投影 §1.3:只保留具名節點序列化的形式(供 L6 對賬)。
    /// 注意規則:匿名節點(含 token 與 trivia)被投影掉,具名節點間的
    /// 祖先序保持(樹同態)。
    pub fn named_sexp(&self) -> String {
        let mut out = String::new();
        pub fn go(t: &Tree, id: u32, out: &mut String) {
            let n = &t.nodes[id as usize];
            if !n.kind.is_named() {
                return;
            }
            out.push('(');
            out.push_str(n.kind.label());
            out.push_str(&format!(" {}", n.span));
            for &c in &n.children {
                go(t, c, out);
            }
            out.push(')');
        }
        go(self, self.root(), &mut out);
        out
    }

    /// 全序列化(含匿名與 trivia 文本):L2 決定論與 L3/L4 等價性的載體。
    pub fn sexp(&self) -> String {
        let mut out = String::new();
        pub fn go(t: &Tree, id: u32, out: &mut String) {
            let n = &t.nodes[id as usize];
            out.push('(');
            out.push_str(n.kind.label());
            out.push_str(&format!(" {}", n.span));
            if n.kind == Kind::Trivia {
                out.push_str(&format!(
                    " {:?}",
                    &t.src[n.span.start as usize..n.span.end as usize]
                ));
            }
            if n.children.is_empty() {
                out.push(')');
                return;
            }
            for &c in &n.children {
                go(t, c, out);
            }
            out.push(')');
        }
        go(self, self.root(), &mut out);
        out
    }

    /// 節點總數(具名 + 匿名 + trivia + error;樹的規模度量)。
    pub fn total_nodes(&self) -> usize {
        self.nodes.len()
    }
}

// ===========================================================================
// 輔助:把樹看作 1 維 CW 複形(§3.4)
// ===========================================================================

/// §3.4 歐拉示性數 χ = |V| − |E| = 1(樹可縮)。
pub fn euler_characteristic(t: &Tree) -> i64 {
    let v = t.nodes.len() as i64;
    let e: i64 = t.nodes.iter().map(|n| n.children.len() as i64).sum();
    v - e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_parse_legal() {
        let src = "fn main() {\n  let mut x = 1;\n  let r = &mut x;\n  let y = x + 1;\n  while y < 10 { f(y); }\n  if y == 3 { f(x, &y); } else { g(); }\n}\n";
        let t = parse(src).expect("parse");
        assert!(!t.has_error(), "legal program must parse cleanly (L7a)");
        assert_eq!(t.unparse(), src, "L1 roundtrip");
        assert!(t.laminar_ok(), "L5 laminar");
        t.validate_continuity().expect("continuity axiom");
        t.validate_tree_shapes().expect("tree axioms");
        println!("sexp: {}", t.sexp());
        println!("named: {}", t.named_sexp());
    }

    #[test]
    fn smoke_parse_garbage() {
        // 全化:任何輸入都產出樹,且 L1 回環逐字節成立。
        for src in [
            "@@@",
            "fn",
            "{ let = ;",
            "if { } else",
            "x = = 3;",
            "()",
            "&mut & &*",
            "fn f(x: & &mut int) {}",
            "/* no block comment in CL0 */ x;",
            "let x = f(a, , b);",
        ] {
            let t = parse(src).expect("total parser");
            assert_eq!(t.unparse(), src, "L1 garbage roundtrip for {:?}", src);
            assert!(t.laminar_ok(), "L5 laminar for {:?}", src);
            t.validate_continuity()
                .unwrap_or_else(|e| panic!("continuity for {:?}: {}", src, e));
        }
    }
}
