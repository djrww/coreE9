//! model —— 受驗對象(PIVOT-RUSTC-ORACLE §四 ModelOracle;P4-1)。
//!
//! 把 R₀ 表面樹接線到語義面:`extract_r0`(R₀ 樹 → `ast::Facts` 事實層)
//! → 三軌 liveness 區間 → 紅邊判決。與 rustc 判決(`oracle::Verdict`)做
//! **accept/reject 層**的 parity 對帳(碼級 parity 是 P4-3;見 ORACLE-TRACE §四)。
//!
//! 三軌分工(parity 實測將不斷驗證這組分工,見 ORACLE-TRACE §二):
//!   * Lexical —— 幾何保守下界:綁定活滿整個作用域;**只准過報,不准漏報**
//!     (gate 對本軌執行 blanket 規則:over 允許、under 即紅);
//!   * Nll     —— 值軌:killer = 後續 Decl / Move(寫);
//!   * Referent —— 借軌:借用活性由「引用綁定的最後使用」決定(借鏈)。
//!
//! 已知模型邊界(如實申報;分歧入 `corpus/PARITY-REGISTRY.json` 附出處):
//!   1. 無型別面 ⇒ Copy 盲(call-arg ident 一律記 Move;i32 拷貝語義不分辨)— S1 掛號;
//!   2. let 初始化的移動語義未建模(`let m2 = m;` 記 Read)— S1 掛號;
//!   3. 作用域逃逸借用(E0597)超出作用域機制 — S2 掛號;
//!   4. while/if 回邊活性以 source 線性序近似 — S2 掛號;
//!   5. call-arg 借用(`g(&mut x)`)無 let 綁定 ⇒ 無借鏈,Nll/Referent 對其
//!      活性保守(至作用域末 / 至下一寫);
//!   6. 不可變性(E0384)是型別面檢查,非幾何語義 —— 範圍外,如實申報。
//!
//! 紀律:ERROR 區域與 Unsupported 節點**不產事實**(與 CL0 `ast::extract`
//! 同一誠實面);此類案例 parity 標記 out-of-scope,不參與對帳。

use crate::ast::{self, Binding, BorrowLink, EvKind, Facts, Track};
use crate::oracle::mini_json;
use crate::r0::{r0_parse, R0Kind, R0Tree};
use crate::span::Span;

/// 參與 parity 門檻的軌道(Lexical 走 blanket 規則,見模組文檔)。
pub const GATE_TRACKS: [&str; 2] = ["nll", "referent"];

/// 借用衝突類錯誤碼:幾何語義**定義上**覆蓋的現場(雙借用 / 借用期寫 / 借用期用)。
/// Lexical blanket 規則只對這組碼執行「永不漏報」;範圍外家族
/// (E0382 移動 / E0384 不可變性 / E0505 let-init 移動 / E0597 逃逸)
/// 的 under 是已申報的模型邊界,不屬幾何承諾。
pub const BORROW_CONFLICT_CODES: [&str; 4] = ["E0499", "E0502", "E0503", "E0506"];

// ===========================================================================
// 第一遍:聲明收集(作用域棧;與 CL0 ast::collect_decls 同構)
// ===========================================================================

/// 聲明位置表:LetStmt / Param 節點 id → 綁定索引。
/// 事件遍行用(第二遍)——屆時以本表直接定位綁定,不靠名稱查找。
type DeclSite = std::collections::HashMap<u32, usize>;

fn name_of(t: &R0Tree, id: u32) -> String {
    let sp = t.node(id).span;
    t.src[sp.start as usize..sp.end as usize].to_string()
}

fn nontrivia_children(t: &R0Tree, id: u32) -> Vec<u32> {
    t.node(id)
        .children
        .iter()
        .copied()
        .filter(|&c| t.node(c).kind != R0Kind::Trivia)
        .collect()
}

fn first_ident_child(t: &R0Tree, id: u32) -> Option<u32> {
    t.node(id)
        .children
        .iter()
        .copied()
        .find(|&c| t.node(c).kind == R0Kind::Ident)
}

/// 遞歸收集聲明:FnItem / Block / 控制流子塊 / let 初始化與語句中的塊表達式。
fn collect_decls(
    t: &R0Tree,
    node: u32,
    facts: &mut Facts,
    scopes: &mut Vec<Vec<usize>>,
    decl_site: &mut DeclSite,
) {
    match t.node(node).kind {
        R0Kind::FnItem => {
            let Some(body) = t
                .node(node)
                .children
                .iter()
                .copied()
                .find(|&c| t.node(c).kind == R0Kind::Block)
            else {
                return; // 半截 fn:如實跳過
            };
            scopes.push(Vec::new());
            for &c in &t.node(node).children {
                if t.node(c).kind == R0Kind::Param {
                    let Some(nid) = first_ident_child(t, c) else {
                        continue;
                    };
                    let b = facts.bindings.len();
                    facts.bindings.push(Binding {
                        name: name_of(t, nid),
                        span: t.node(nid).span,
                        mutable: false, // R₀ 參數無 mut(附錄 B)
                        is_param: true,
                        scope: t.node(body).span,
                    });
                    decl_site.insert(c, b);
                    scopes.last_mut().unwrap().push(b);
                }
            }
            collect_decls(t, body, facts, scopes, decl_site);
            scopes.pop();
        }
        R0Kind::Block => {
            scopes.push(Vec::new());
            for c in t.node(node).children.clone() {
                match t.node(c).kind {
                    R0Kind::LetStmt => {
                        let Some(nid) = first_ident_child(t, c) else {
                            continue; // 半截 let:如實跳過
                        };
                        let mutable = t
                            .node(c)
                            .children
                            .iter()
                            .any(|&k| t.node(k).kind == R0Kind::MutKw);
                        let b = facts.bindings.len();
                        facts.bindings.push(Binding {
                            name: name_of(t, nid),
                            span: t.node(nid).span,
                            mutable,
                            is_param: false,
                            scope: t.node(node).span,
                        });
                        decl_site.insert(c, b);
                        scopes.last_mut().unwrap().push(b);
                    }
                    R0Kind::IfStmt | R0Kind::WhileStmt | R0Kind::LoopStmt => {
                        collect_decls(t, c, facts, scopes, decl_site);
                    }
                    R0Kind::ExprStmt | R0Kind::ReturnStmt => {
                        scan_nested_blocks(t, c, facts, scopes, decl_site);
                    }
                    _ => {}
                }
            }
            scopes.pop();
        }
        R0Kind::IfStmt | R0Kind::WhileStmt | R0Kind::LoopStmt => {
            for c in t.node(node).children.clone() {
                let k = t.node(c).kind;
                if k == R0Kind::Block || k == R0Kind::IfStmt {
                    collect_decls(t, c, facts, scopes, decl_site);
                }
            }
        }
        _ => {
            scan_nested_blocks(t, node, facts, scopes, decl_site);
        }
    }
}

/// 子樹內的塊表達式(`let r = { … };` / `{ … };` 語句)也有自己的作用域。
fn scan_nested_blocks(
    t: &R0Tree,
    node: u32,
    facts: &mut Facts,
    scopes: &mut Vec<Vec<usize>>,
    decl_site: &mut DeclSite,
) {
    for c in t.node(node).children.clone() {
        match t.node(c).kind {
            R0Kind::Block => collect_decls(t, c, facts, scopes, decl_site),
            R0Kind::Expr
            | R0Kind::UnaryExpr
            | R0Kind::LetStmt
            | R0Kind::ExprStmt
            | R0Kind::ReturnStmt
            | R0Kind::IfStmt
            | R0Kind::WhileStmt
            | R0Kind::LoopStmt => {
                scan_nested_blocks(t, c, facts, scopes, decl_site);
            }
            _ => {}
        }
    }
}

// ===========================================================================
// 第二遍:事件收集(與 CL0 EventCollector 同構;R₀ 的扁平 Expr 特化)
// ===========================================================================

#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    /// 普通 valuation:ident → Read。
    Value,
    /// 呼叫實參位置:單 ident → Move(Copy 盲,模組文檔邊界 1)。
    CallArg,
}

struct EventCollector<'a> {
    t: &'a R0Tree,
    facts: &'a mut Facts,
    scopes: &'a mut Vec<Vec<usize>>,
    decl_site: &'a DeclSite,
}

impl<'a> EventCollector<'a> {
    fn emit(&mut self, name: &str, span: Span, kind: EvKind) {
        if let Some(b) = crate::ast::lookup_binding(self.scopes, self.facts, name) {
            self.facts.events.push(crate::ast::Event {
                binding: b,
                kind,
                span,
            });
        }
    }

    fn walk_node(&mut self, node: u32) {
        match self.t.node(node).kind {
            R0Kind::FnItem => {
                let Some(body) = self
                    .t
                    .node(node)
                    .children
                    .iter()
                    .copied()
                    .find(|&c| self.t.node(c).kind == R0Kind::Block)
                else {
                    return;
                };
                // 參數作用域(fn body 層);事件遍行期間真正維護作用域棧
                self.scopes.push(Vec::new());
                for &c in &self.t.node(node).children {
                    if self.t.node(c).kind == R0Kind::Param {
                        if let Some(&b) = self.decl_site.get(&c) {
                            self.scopes.last_mut().unwrap().push(b);
                        }
                    }
                }
                self.walk_node(body);
                self.scopes.pop();
            }
            R0Kind::Block => {
                self.scopes.push(Vec::new());
                for c in self.t.node(node).children.clone() {
                    if self.t.node(c).kind == R0Kind::LetStmt {
                        self.walk_let(c); // init 走完才註冊綁定(遮蔽語義,見 walk_let)
                    } else {
                        self.walk_node(c);
                    }
                }
                self.scopes.pop();
            }
            R0Kind::ExprStmt => {
                let exprs: Vec<u32> = self
                    .t
                    .node(node)
                    .children
                    .iter()
                    .copied()
                    .filter(|&c| self.t.node(c).kind == R0Kind::Expr)
                    .collect();
                for e in exprs {
                    self.walk_stmt_expr(e);
                }
            }
            R0Kind::IfStmt | R0Kind::WhileStmt => {
                for c in self.t.node(node).children.clone() {
                    match self.t.node(c).kind {
                        R0Kind::Expr => self.walk_expr(c, Ctx::Value),
                        R0Kind::Block | R0Kind::IfStmt => self.walk_node(c),
                        _ => {}
                    }
                }
            }
            R0Kind::LoopStmt => {
                for c in self.t.node(node).children.clone() {
                    if self.t.node(c).kind == R0Kind::Block {
                        self.walk_node(c);
                    }
                }
            }
            R0Kind::ReturnStmt => {
                for c in self.t.node(node).children.clone() {
                    if self.t.node(c).kind == R0Kind::Expr {
                        self.walk_expr(c, Ctx::Value);
                    }
                }
            }
            R0Kind::Error | R0Kind::Unsupported => {} // 事實面對排除區如實靜默
            _ => {}
        }
    }

    /// `let [mut] IDENT [= expr];`:Decl + (若 init 是 `&x`/`&mut x`)借用事件與借鏈。
    fn walk_let(&mut self, node: u32) {
        let kids = nontrivia_children(self.t, node);
        let Some(nid) = kids
            .iter()
            .copied()
            .find(|&c| self.t.node(c).kind == R0Kind::Ident)
        else {
            return;
        };
        let Some(&b) = self.decl_site.get(&node) else {
            return; // 半截 let:如實跳過
        };
        let span = self.t.node(nid).span;
        // Decl 事件直接以 decl_site 定位(此時新綁定尚未入作用域,不靠名稱查找)
        self.facts.events.push(crate::ast::Event {
            binding: b,
            kind: EvKind::Decl,
            span,
        });
        // init:Eq 之後的 Expr 子節點
        let eq_pos = kids.iter().position(|&c| self.t.node(c).kind == R0Kind::Eq);
        let init = eq_pos.and_then(|p| {
            kids[p + 1..]
                .iter()
                .copied()
                .find(|&c| self.t.node(c).kind == R0Kind::Expr)
        });
        if let Some(init) = init {
            if let Some((src, kind, src_span)) = expr_borrow_shape(self.t, init) {
                // 借鏈:`let r = &x;` —— Referent 軌的活性錨點
                self.emit(&src, src_span, kind);
                if let Some(sb) = crate::ast::lookup_binding(self.scopes, self.facts, &src) {
                    self.facts.links.push(BorrowLink {
                        ref_binding: b,
                        src_binding: sb,
                        kind,
                        // ⚠ span 取「源 ident 的 span」(= 借用事件的 span):
                        // ast::intervals 的 Referent 軌以 `l.span == ev.span` 配對借鏈;
                        // CL0 的構造用了綁定名 span,配對永不命中(死代碼,CL0 事件
                        // 真空下未被曝光 —— 見 ORACLE-TRACE 發現 #2)。模型側修在本處。
                        span: src_span,
                    });
                }
            } else {
                self.walk_expr(init, Ctx::Value);
            }
        }
        // 遮蔽語義(與 Rust 一致):RHS 先行 —— `let x = x + 1;` 的 x 指外層;
        // 走完 init 才把新綁定註冊進作用域。
        self.scopes.last_mut().unwrap().push(b);
    }

    /// 語句級表達式:賦值(`x = e` / `*p = e`)與一般表達式。
    fn walk_stmt_expr(&mut self, node: u32) {
        let kids = nontrivia_children(self.t, node);
        if let Some(eq) = kids.iter().position(|&c| self.t.node(c).kind == R0Kind::Eq) {
            let lhs = &kids[..eq];
            // LHS 分類:單 ident → 寫(Move 語義);`*p` → 寫穿(Deref)。
            if lhs.len() == 1 && self.t.node(lhs[0]).kind == R0Kind::Ident {
                let name = name_of(self.t, lhs[0]);
                let sp = self.t.node(lhs[0]).span;
                self.emit(&name, sp, EvKind::Move);
            } else if lhs.len() == 1 && self.t.node(lhs[0]).kind == R0Kind::UnaryExpr {
                self.walk_unary(lhs[0]); // `*p = …`:p 上記 Deref(寫穿)
            } else {
                self.walk_flat(lhs, Ctx::Value);
            }
            self.walk_flat(&kids[eq + 1..], Ctx::Value);
        } else {
            self.walk_flat(&kids, Ctx::Value);
        }
    }

    /// 完整 Expr 節點(呼叫實參 / 括號內容):單 ident 實參記 Move(Copy 盲,邊界 1)。
    fn walk_expr(&mut self, node: u32, ctx: Ctx) {
        let kids = nontrivia_children(self.t, node);
        if ctx == Ctx::CallArg && kids.len() == 1 && self.t.node(kids[0]).kind == R0Kind::Ident {
            let name = name_of(self.t, kids[0]);
            let sp = self.t.node(kids[0]).span;
            self.emit(&name, sp, EvKind::Move);
            return;
        }
        self.walk_flat(&kids, Ctx::Value);
    }

    /// 扁平子節點序列的分類器:呼叫 / 字段 / 借用 / 解引用 / 讀。
    fn walk_flat(&mut self, kids: &[u32], ctx: Ctx) {
        let mut i = 0;
        while i < kids.len() {
            let c = kids[i];
            match self.t.node(c).kind {
                R0Kind::UnaryExpr => {
                    self.walk_unary(c);
                    i += 1;
                }
                R0Kind::Ident => {
                    // 呼叫偵測:ident 緊隨 LParen(層深配對)→ 區塊式消費實參
                    let next = kids.get(i + 1).copied();
                    if next.is_some_and(|n| self.t.node(n).kind == R0Kind::LParen) {
                        let mut depth = 0usize;
                        let mut j = i + 1;
                        while j < kids.len() {
                            match self.t.node(kids[j]).kind {
                                R0Kind::LParen => depth += 1,
                                R0Kind::RParen => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            j += 1;
                        }
                        // (i+1..j) 內的 Expr 子節點 = 實參;ident 為 callee 不計
                        for k in kids[i + 1..j.min(kids.len())].iter().copied() {
                            if self.t.node(k).kind == R0Kind::Expr {
                                self.walk_expr(k, Ctx::CallArg);
                            } else if self.t.node(k).kind == R0Kind::UnaryExpr {
                                self.walk_unary(k); // `g(&mut x)`
                            }
                        }
                        i = j + 1;
                        continue;
                    }
                    // 字段名(`.f`)不計使用;字段基座(`x.f` 的 x)按讀取計
                    let prev_is_dot = i > 0 && self.t.node(kids[i - 1]).kind == R0Kind::Dot;
                    if prev_is_dot {
                        i += 1;
                        continue;
                    }
                    let name = name_of(self.t, c);
                    let sp = self.t.node(c).span;
                    let kind = match ctx {
                        Ctx::CallArg => EvKind::Move,
                        _ => EvKind::Read,
                    };
                    self.emit(&name, sp, kind);
                    i += 1;
                }
                R0Kind::Expr => {
                    self.walk_expr(c, Ctx::Value); // 括號表達式
                    i += 1;
                }
                R0Kind::Block => {
                    self.walk_node(c); // 塊表達式
                    i += 1;
                }
                R0Kind::LBrack => {
                    // 索引:`x[i]` —— 區間內 ident 按讀取
                    let mut depth = 0usize;
                    let mut j = i;
                    while j < kids.len() {
                        match self.t.node(kids[j]).kind {
                            R0Kind::LBrack => depth += 1,
                            R0Kind::RBrack => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    let inner: Vec<u32> = kids[i + 1..j.min(kids.len())].to_vec();
                    self.walk_flat(&inner, Ctx::Value);
                    i = j + 1;
                }
                _ => {
                    i += 1; // 運算子 / 字面量 / 界符:不產事件
                }
            }
        }
    }

    /// 一元前綴:`&x` / `&mut x`(借用)/ `*p`(解引用)/ `!e`(透傳)。
    fn walk_unary(&mut self, node: u32) {
        let kids = nontrivia_children(self.t, node);
        let op = kids.first().map(|&c| self.t.node(c).kind);
        let inner = kids.iter().copied().find(|&c| {
            matches!(
                self.t.node(c).kind,
                R0Kind::Ident | R0Kind::Expr | R0Kind::UnaryExpr
            )
        });
        match (op, inner) {
            (Some(R0Kind::Amp | R0Kind::AmpMut), Some(inner))
                if self.t.node(inner).kind == R0Kind::Ident =>
            {
                let name = name_of(self.t, inner);
                let sp = self.t.node(inner).span;
                let kind = if op == Some(R0Kind::AmpMut) {
                    EvKind::BorrowMut
                } else {
                    EvKind::BorrowSh
                };
                self.emit(&name, sp, kind);
            }
            (Some(R0Kind::Star), Some(inner)) if self.t.node(inner).kind == R0Kind::Ident => {
                let name = name_of(self.t, inner);
                let sp = self.t.node(inner).span;
                self.emit(&name, sp, EvKind::Deref);
            }
            (_, Some(inner)) => match self.t.node(inner).kind {
                R0Kind::Expr => self.walk_expr(inner, Ctx::Value),
                R0Kind::UnaryExpr => self.walk_unary(inner),
                _ => {}
            },
            _ => {}
        }
    }
}

/// `expr` 是否整體是 `&x` / `&mut x`(借鏈偵測;Expr → UnaryExpr → [Amp, Ident])。
fn expr_borrow_shape(t: &R0Tree, expr: u32) -> Option<(String, EvKind, Span)> {
    let kids = nontrivia_children(t, expr);
    if kids.len() != 1 || t.node(kids[0]).kind != R0Kind::UnaryExpr {
        return None;
    }
    let uk = nontrivia_children(t, kids[0]);
    let op = uk.first().map(|&c| t.node(c).kind)?;
    let ident = uk
        .iter()
        .copied()
        .find(|&c| t.node(c).kind == R0Kind::Ident)?;
    let kind = match op {
        R0Kind::Amp => EvKind::BorrowSh,
        R0Kind::AmpMut => EvKind::BorrowMut,
        _ => return None,
    };
    Some((name_of(t, ident), kind, t.node(ident).span))
}

/// R₀ 樹 → 事實層(對外入口;兩遍:聲明 → 事件)。
pub fn extract_r0(t: &R0Tree) -> Facts {
    let mut facts = Facts {
        bindings: Vec::new(),
        events: Vec::new(),
        links: Vec::new(),
        has_error_regions: t.has_error(),
    };
    let root = t.root();
    let mut scopes: Vec<Vec<usize>> = vec![Vec::new()];
    let mut decl_site: DeclSite = DeclSite::new();
    for c in t.node(root).children.clone() {
        match t.node(c).kind {
            R0Kind::FnItem | R0Kind::StructItem | R0Kind::Error => {
                collect_decls(t, c, &mut facts, &mut scopes, &mut decl_site);
            }
            _ => {}
        }
    }
    for c in t.node(root).children.clone() {
        if t.node(c).kind == R0Kind::FnItem {
            let mut e = EventCollector {
                t,
                facts: &mut facts,
                scopes: &mut scopes,
                decl_site: &decl_site,
            };
            e.walk_node(c);
        }
    }
    facts
}

// ===========================================================================
// 模型判決(ModelReport)與 parity 對帳
// ===========================================================================

/// 單軌判決:是否拒絕(紅邊非空)+ 佐證。
#[derive(Clone, Debug)]
pub struct TrackVerdict {
    /// 軌道標籤("lexical" / "nll" / "referent")。
    pub track: &'static str,
    /// 模型在該軌下是否拒絕(存在紅邊)。
    pub reject: bool,
    /// 紅邊數。
    pub red_edges: usize,
    /// 首條紅邊跨度(診斷/對帳表顯示用)。
    pub first_red: Option<Span>,
}

/// 模型判決:三軌 + 範圍聲明。
#[derive(Clone, Debug)]
pub struct ModelReport {
    /// 是否在模型範圍內(無 Error / Unsupported 節點)。
    pub in_scope: bool,
    /// 範圍外原因(in_scope = false 時)。
    pub scope_note: Option<String>,
    /// 三軌判決。
    pub tracks: Vec<TrackVerdict>,
}

/// 對單份源碼跑模型:R₀ 解析 → 範圍檢查 → 事實層 → 三軌紅邊。
pub fn model_check(src: &str) -> ModelReport {
    let track_of = |label: &str| match label {
        "lexical" => Track::Lexical,
        "referent" => Track::Referent,
        _ => Track::Nll,
    };
    let mk = |facts: &Facts, label: &'static str| {
        let red = ast::red_edges(facts, track_of(label));
        TrackVerdict {
            track: label,
            reject: !red.is_empty(),
            red_edges: red.len(),
            first_red: red.first().map(|e| e.span),
        }
    };
    let tree = match r0_parse(src) {
        Ok(t) => t,
        Err(_) => {
            return ModelReport {
                in_scope: false,
                scope_note: Some("r0_parse 失敗(全化下不應發生)".to_string()),
                tracks: Vec::new(),
            }
        }
    };
    if tree.has_error() {
        return ModelReport {
            in_scope: false,
            scope_note: Some("error 節點(語法錯誤區)".to_string()),
            tracks: Vec::new(),
        };
    }
    let unsup = tree.unsupported_spans();
    if !unsup.is_empty() {
        return ModelReport {
            in_scope: false,
            scope_note: Some(format!("unsupported 節點 ×{}", unsup.len())),
            tracks: Vec::new(),
        };
    }
    let facts = extract_r0(&tree);
    ModelReport {
        in_scope: true,
        scope_note: None,
        tracks: vec![
            mk(&facts, "lexical"),
            mk(&facts, "nll"),
            mk(&facts, "referent"),
        ],
    }
}

/// 單案例 parity 結果。
#[derive(Clone, Debug)]
pub struct ParityCase {
    /// 語料檔名。
    pub file: String,
    /// rustc 是否接受。
    pub rustc_accept: bool,
    /// rustc 錯誤碼(拒絕時)。
    pub rustc_codes: Vec<String>,
    /// 模型是否在範圍內。
    pub in_scope: bool,
    /// 範圍外原因。
    pub scope_note: Option<String>,
    /// 三軌判決。
    pub tracks: Vec<TrackVerdict>,
    /// 分歧清單(gate 軌道:track + 方向 over/under)。
    pub mismatches: Vec<(&'static str, &'static str)>,
}

/// parity 註冊表項:已歸檔的已知分歧(附出處)。
#[derive(Clone, Debug)]
pub struct RegistryEntry {
    /// 語料檔名。
    pub file: String,
    /// 軌道(nll / referent)。
    pub track: String,
    /// 方向:over = 模型過報 / under = 模型漏報。
    pub direction: String,
    /// 歸因(必填;MODEL-DIFF 出處)。
    pub why: String,
}

/// parity 註冊表。
#[derive(Clone, Debug, Default)]
pub struct ParityRegistry {
    /// 已歸檔分歧(gate 軌道)。
    pub entries: Vec<RegistryEntry>,
    /// blanket 規則的軌道(lexical:僅允許 over)。
    pub blanket_track: Option<String>,
}

impl ParityRegistry {
    /// 從 JSON 文本解析註冊表(格式見 corpus/PARITY-REGISTRY.json)。
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v = mini_json::parse(text).map_err(|e| format!("註冊表 JSON 解析失敗:{e}"))?;
        let mut reg = ParityRegistry::default();
        if let Some(entries) = v.get("entries").and_then(mini_json::Json::as_arr) {
            for e in entries {
                let file = e
                    .get("file")
                    .and_then(mini_json::Json::as_str)
                    .ok_or("entry 缺 file")?
                    .to_string();
                let track = e
                    .get("track")
                    .and_then(mini_json::Json::as_str)
                    .ok_or("entry 缺 track")?
                    .to_string();
                let direction = e
                    .get("direction")
                    .and_then(mini_json::Json::as_str)
                    .ok_or("entry 缺 direction")?
                    .to_string();
                let why = e
                    .get("why")
                    .and_then(mini_json::Json::as_str)
                    .ok_or("entry 缺 why(歸因是強制的)")?
                    .to_string();
                reg.entries.push(RegistryEntry {
                    file,
                    track,
                    direction,
                    why,
                });
            }
        }
        reg.blanket_track = v
            .get("blanket")
            .and_then(|b| b.get("track"))
            .and_then(mini_json::Json::as_str)
            .map(str::to_string);
        Ok(reg)
    }
}

/// 計算單案例的 gate 軌道分歧。
fn mismatches_of(c: &ParityCase) -> Vec<(&'static str, &'static str)> {
    if !c.in_scope {
        return Vec::new();
    }
    let mut out = Vec::new();
    for t in GATE_TRACKS {
        if let Some(tv) = c.tracks.iter().find(|tv| tv.track == t) {
            if tv.reject == c.rustc_accept {
                let dir = if tv.reject && c.rustc_accept {
                    "over"
                } else {
                    "under"
                };
                out.push((t, dir));
            }
        }
    }
    out
}

/// 對帳驗證:回傳違規清單(空 = 全部合规)。
/// 規則:gate 軌道的每個分歧必須精確註冊(檔+軌+向);註冊項必須仍在發生
/// (防 stale);blanket 軌道(lexical)只准 over、under 即紅。
pub fn parity_violations(cases: &[ParityCase], reg: &ParityRegistry) -> Vec<String> {
    let mut out = Vec::new();
    let mut realized: Vec<bool> = vec![false; reg.entries.len()];
    for c in cases {
        if !c.in_scope {
            continue;
        }
        for (t, dir) in &c.mismatches {
            let hit = reg
                .entries
                .iter()
                .enumerate()
                .find(|(_, e)| e.file == c.file && e.track == *t && e.direction == *dir);
            match hit {
                Some((i, _)) => realized[i] = true,
                None => out.push(format!(
                    "未註冊分歧(BUG 候選):{} 軌 {} 方向 {} — 歸因後入 PARITY-REGISTRY",
                    c.file, t, dir
                )),
            }
        }
        // blanket 規則:lexical 只准 over —— 且僅對借用衝突類碼承諾「永不漏報」
        //(範圍外家族的 under 是已申報的模型邊界,見 BORROW_CONFLICT_CODES 文檔)
        if let Some(lx) = c.tracks.iter().find(|tv| tv.track == "lexical") {
            let is_borrow_conflict = c
                .rustc_codes
                .iter()
                .any(|x| BORROW_CONFLICT_CODES.contains(&x.as_str()));
            if !c.rustc_accept && !lx.reject && is_borrow_conflict {
                out.push(format!(
                    "lexical 軌漏報(不可接受):{} — rustc 報借用衝突({:?})而幾何保守下界竟接受",
                    c.file, c.rustc_codes
                ));
            }
        }
    }
    for (i, e) in reg.entries.iter().enumerate() {
        if !realized[i] {
            out.push(format!(
                "註冊項已失效(stale):{} 軌 {} 方向 {} — 分歧不再發生,請更新註冊表",
                e.file, e.track, e.direction
            ));
        }
    }
    out
}

/// 跑整個語料目錄的 parity(rustc × 模型三軌)。
pub fn parity_dir(
    dir: &str,
    oracle: &dyn crate::oracle::Oracle,
) -> Result<Vec<ParityCase>, String> {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("語料目錄不可讀({dir}):{e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let file = f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let src = std::fs::read_to_string(&f).map_err(|e| format!("{file}:{e}"))?;
        let rep = oracle
            .check(&src)
            .map_err(|e| format!("{file}:oracle 錯誤:{e}"))?;
        let m = model_check(&src);
        let mut pc = ParityCase {
            file,
            rustc_accept: rep.verdict.is_accept(),
            rustc_codes: rep.verdict.codes(),
            in_scope: m.in_scope,
            scope_note: m.scope_note,
            tracks: m.tracks,
            mismatches: Vec::new(),
        };
        pc.mismatches = mismatches_of(&pc);
        out.push(pc);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_r0_events_and_links() {
        let src = "fn g(p: &mut i32) { *p = *p + 1; }\nfn f() {\n    let mut x = 5;\n    let r = &x;\n    x = 6;\n    let z = *r;\n}\n";
        let tree = r0_parse(src).unwrap();
        let f = extract_r0(&tree);
        // 綁定:g 的參數 p、f 的 x、r、z
        assert_eq!(f.bindings.len(), 4);
        assert_eq!(f.bindings[1].name, "x");
        assert!(f.bindings[1].mutable);
        assert_eq!(f.bindings[0].name, "p");
        assert!(f.bindings[0].is_param);
        // 事件:x 上 Decl / BorrowSh / Move(賦值寫);r 上 Decl / Deref
        let kinds = |b: usize| {
            let mut v: Vec<EvKind> = f
                .events
                .iter()
                .filter(|e| e.binding == b)
                .map(|e| e.kind)
                .collect();
            v.sort_by_key(|k| format!("{k:?}"));
            v
        };
        assert_eq!(f.bindings.iter().position(|b| b.name == "x").unwrap(), 1);
        assert_eq!(kinds(1), vec![EvKind::BorrowSh, EvKind::Decl, EvKind::Move]);
        let ri = f.bindings.iter().position(|b| b.name == "r").unwrap();
        assert_eq!(kinds(ri), vec![EvKind::Decl, EvKind::Deref]);
        // 借鏈:r ← &x(Referent 軌的活性錨點)
        assert_eq!(f.links.len(), 1);
        assert_eq!(f.links[0].ref_binding, ri);
        assert_eq!(f.links[0].src_binding, 1);
        assert_eq!(f.links[0].kind, EvKind::BorrowSh);
    }

    #[test]
    fn extract_r0_call_args_and_deref_write() {
        let src = "fn g(p: &mut i32) {\n    *p = 1;\n}\nfn f() {\n    let mut x = 5;\n    g(&mut x);\n    let a = x + 1;\n}\n";
        let tree = r0_parse(src).unwrap();
        let f = extract_r0(&tree);
        let xi = f.bindings.iter().position(|b| b.name == "x").unwrap();
        let mut xs: Vec<EvKind> = f
            .events
            .iter()
            .filter(|e| e.binding == xi)
            .map(|e| e.kind)
            .collect();
        xs.sort_by_key(|k| format!("{k:?}"));
        // 呼叫實參 `&mut x` → BorrowMut;讀取 → Read
        assert_eq!(xs, vec![EvKind::BorrowMut, EvKind::Decl, EvKind::Read]);
        // `*p = 1` → p 上 Deref(寫穿)
        let pi = f.bindings.iter().position(|b| b.name == "p").unwrap();
        assert!(f
            .events
            .iter()
            .any(|e| e.binding == pi && e.kind == EvKind::Deref));
    }

    #[test]
    fn model_check_scope_gating() {
        // 語法錯誤 → out of scope(不參與 parity)
        let m = model_check("fn f( {\n    let x = 1;\n}\n");
        assert!(!m.in_scope);
        assert!(m.scope_note.is_some());
        // 乾淨程式 → 三軌全綠
        let m = model_check("fn f() {\n    let x = 5;\n    let y = x + 1;\n}\n");
        assert!(m.in_scope);
        assert_eq!(m.tracks.len(), 3);
        assert!(m.tracks.iter().all(|t| !t.reject));
        // E0506 現場:Referent 拒絕(rustc 同拒);Nll 漏報(已註冊分歧)
        let m = model_check(
            "fn f() {\n    let mut x = 5;\n    let r = &x;\n    x = 6;\n    let z = *r;\n}\n",
        );
        assert!(m.in_scope);
        let by = |l: &str| m.tracks.iter().find(|t| t.track == l).unwrap().reject;
        assert!(by("referent"));
        assert!(!by("nll"));
    }

    #[test]
    fn extract_r0_shadowing_and_nested_scope() {
        let src = "fn f() {\n    let x = 5;\n    {\n        let x = 6;\n        let y = x + 1;\n    }\n    let z = x + 2;\n}\n";
        let tree = r0_parse(src).unwrap();
        let f = extract_r0(&tree);
        // 兩個 x 綁定(不同 index);內層 y 事件綁到內層 x,外層 z 綁到外層 x
        let xs: Vec<usize> = f
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.name == "x")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(xs.len(), 2);
        assert_ne!(f.bindings[xs[0]].scope, f.bindings[xs[1]].scope);
        // y 的宣告 + 讀取 x(內層)
        let read_inner = f
            .events
            .iter()
            .any(|e| e.binding == xs[1] && e.kind == EvKind::Read);
        let read_outer = f
            .events
            .iter()
            .any(|e| e.binding == xs[0] && e.kind == EvKind::Read);
        assert!(read_inner, "內層 x 應被讀取");
        assert!(read_outer, "外層 x 應被讀取");
    }
}
