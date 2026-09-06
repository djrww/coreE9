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
//! 已知模型邊界(如實申報;分歧入 `corpus/PARITY-REGISTRY.json` 附出處;
//! P4-3 後剩餘):
//!   1. 作用域逃逸借用(E0597)超出作用域機制 —— 生命週期求解,範圍外;
//!   2. 不可變性(E0384)是型別面檢查,非幾何語義 —— 範圍外,如實申報;
//!   3. 字段賦值 `x.a = e` 記為基座的部分寫(place 精確,consumes=false);
//!      對**共享**借用的字段寫之冲突由「部分寫 × 借用」紅邊覆蓋,但
//!      place 前綴之外的跨字段活性不細分(過報允許、漏報不允許的紀律下,
//!      現行近似偏保守,如實申報);
//!   4. match 的分支控制流(若 R₀ 擴張至 match,見 P4-7)以源碼線性序近似。
//!
//! P4-3 已解決(2026-09-07;ORACLE-TRACE §P4-3 記錄實測):
//!   * S1 place 敏感度:事件攜帶字段路徑,x.f1/x.f2 不相交(Lexical 軌保持
//!     綁定粒度 = 幾何保守下界);
//!   * S1 型別面:Copy(int) call-arg/let-init → Read;非 Copy(引用)→
//!     Move + consumes(use-after-move 死用紅邊);
//!   * S1 臨時借用區域:call-arg 借用死於呼叫返回(dies_at);
//!   * S2 回邊活性:let 形式借用,且**引用綁定在迴圈內被使用**(條件逐迭代
//!     重求值)⇒ 借用活性延伸至迴圈出口;dies_at 臨時借用(呼叫實參)精確限於
//!     呼叫,不回邊延伸;引用未用/最後使用在迴圈前 ⇒ 不跨迴圈(迴圈內是對
//!     referent 的直接訪問,不延續本借用);
//!   * 區間語義:Read/Deref/Decl = 點事件[Nll/Referent];值生命週期(e0382)
//!     由 dead-use 紅邊承載,不靠區間延伸(否則「讀區間 × 已死借用」虛假衝突);
//!   * 軌道分工收斂:點寫(Move = 點活性)+ let 形式借用端點 = 引用最後使用
//!     (兩軌同律)—— 消滅 F-A/F-B/F-C/F-D/F-E/F-F 家族(註冊表 24 → 4);
//!     fuzz 2000 輪 gate over/under = (0,0)/(0,0)(2026-09-07,rustc 1.98.1)。
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
    btypes: &mut std::collections::HashMap<usize, TypeClass>,
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
                    btypes.insert(b, param_type(t, c));
                    scopes.last_mut().unwrap().push(b);
                }
            }
            collect_decls(t, body, facts, scopes, decl_site, btypes);
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
                        collect_decls(t, c, facts, scopes, decl_site, btypes);
                    }
                    R0Kind::ExprStmt | R0Kind::ReturnStmt => {
                        scan_nested_blocks(t, c, facts, scopes, decl_site, btypes);
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
                    collect_decls(t, c, facts, scopes, decl_site, btypes);
                }
            }
        }
        _ => {
            scan_nested_blocks(t, node, facts, scopes, decl_site, btypes);
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
    btypes: &mut std::collections::HashMap<usize, TypeClass>,
) {
    for c in t.node(node).children.clone() {
        match t.node(c).kind {
            R0Kind::Block => collect_decls(t, c, facts, scopes, decl_site, btypes),
            R0Kind::Expr
            | R0Kind::UnaryExpr
            | R0Kind::LetStmt
            | R0Kind::ExprStmt
            | R0Kind::ReturnStmt
            | R0Kind::IfStmt
            | R0Kind::WhileStmt
            | R0Kind::LoopStmt => {
                scan_nested_blocks(t, c, facts, scopes, decl_site, btypes);
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
    /// 呼叫實參位置:單 ident → 型別面決定(S1,P4-3):Copy → Read;
    /// 非 Copy → Move + consumes。
    CallArg,
}

/// 型別面分類(S1 型別面,P4-3):R₀ 的型別只有「Copy 值」與「引用」兩類
/// 語義相關性(未知保守處理:不設 consumes,事件-kind 保守)。
#[derive(Clone, Copy, PartialEq)]
enum TypeClass {
    /// int / i32 / bool 等 Copy 值。
    Int,
    /// `&T` / `&mut T`(非 Copy)。
    Ref,
    /// 未判定(呼叫結果 / 塊 / 未標註參數)。
    Unknown,
}

/// 形參型別面(從 TypeRef 節點):`&T`/`&mut T` → Ref;裸型名 → Int;
/// 陣列 `[…T]` → Unknown(R₀ 罕見,保守)。
fn param_type(t: &R0Tree, param: u32) -> TypeClass {
    let type_ref = t
        .node(param)
        .children
        .iter()
        .copied()
        .find(|&c| t.node(c).kind == R0Kind::TypeRef);
    let Some(tr) = type_ref else {
        return TypeClass::Unknown;
    };
    let kids = nontrivia_children(t, tr);
    if kids
        .iter()
        .any(|&c| matches!(t.node(c).kind, R0Kind::Amp | R0Kind::AmpMut))
    {
        TypeClass::Ref
    } else if kids.iter().any(|&c| t.node(c).kind == R0Kind::LBrack) {
        TypeClass::Unknown
    } else {
        TypeClass::Int
    }
}

/// 表達式型別面推斷(S1):字面量 → Int;`&x`/`&mut x` → Ref;`*r` → Int;
/// 單 ident → 該綁定的型別;呼叫/塊 → Unknown;算術/比較 → Int(R₀ 算術
/// 只作用於 int 類值 —— 若操作數是引用型,程式本身非合法 Rust,rustc 會拒)。
fn expr_type(
    t: &R0Tree,
    expr: u32,
    btypes: &std::collections::HashMap<usize, TypeClass>,
    facts: &Facts,
    scopes: &[Vec<usize>],
) -> TypeClass {
    let kids = nontrivia_children(t, expr);
    if kids.len() == 1 {
        match t.node(kids[0]).kind {
            R0Kind::Number | R0Kind::TrueKw | R0Kind::FalseKw => TypeClass::Int,
            R0Kind::Ident => {
                let name = name_of(t, kids[0]);
                crate::ast::lookup_binding(scopes, facts, &name)
                    .and_then(|b| btypes.get(&b).copied())
                    .unwrap_or(TypeClass::Unknown)
            }
            R0Kind::UnaryExpr => {
                let uk = nontrivia_children(t, kids[0]);
                let op = uk.first().map(|&c| t.node(c).kind);
                match op {
                    Some(R0Kind::Amp) | Some(R0Kind::AmpMut) => TypeClass::Ref,
                    Some(R0Kind::Star) => TypeClass::Int,
                    _ => TypeClass::Unknown,
                }
            }
            R0Kind::Expr => expr_type(t, kids[0], btypes, facts, scopes),
            R0Kind::Block => TypeClass::Unknown,
            _ => TypeClass::Unknown,
        }
    } else if kids.len() > 1 {
        // 呼叫?ident 緊隨 LParen
        if t.node(kids[0]).kind == R0Kind::Ident
            && kids
                .get(1)
                .is_some_and(|&c| t.node(c).kind == R0Kind::LParen)
        {
            TypeClass::Unknown
        } else {
            TypeClass::Int
        }
    } else {
        TypeClass::Unknown
    }
}

/// 字段鏈偵測:從 `kids[i]`(基座 ident)起,`Dot Ident` 對的序列 → place 路徑。
/// (不消費游標;主迴圈的既有 Dot/field-ident 跳躍邏輯不變。)
fn field_chain(t: &R0Tree, kids: &[u32], i: usize) -> Vec<String> {
    let mut place = Vec::new();
    let mut j = i + 1;
    while j + 1 < kids.len()
        && t.node(kids[j]).kind == R0Kind::Dot
        && t.node(kids[j + 1]).kind == R0Kind::Ident
    {
        place.push(name_of(t, kids[j + 1]));
        j += 2;
    }
    place
}

struct EventCollector<'a> {
    t: &'a R0Tree,
    facts: &'a mut Facts,
    scopes: &'a mut Vec<Vec<usize>>,
    decl_site: &'a DeclSite,
    /// 綁定索引 → 型別面分類(S1 型別面)。
    btypes: &'a mut std::collections::HashMap<usize, TypeClass>,
}

impl<'a> EventCollector<'a> {
    /// 預設事件(無 place / 無 dies_at / 無 consumes)。
    fn emit(&mut self, name: &str, span: Span, kind: EvKind) {
        self.emit_ext(name, span, kind, Vec::new(), None, false);
    }

    /// 完整事件(S1:place 路徑 + 臨時借用 dies_at + 型別面 consumes)。
    fn emit_ext(
        &mut self,
        name: &str,
        span: Span,
        kind: EvKind,
        place: Vec<String>,
        dies_at: Option<u32>,
        consumes: bool,
    ) {
        if let Some(b) = crate::ast::lookup_binding(self.scopes, self.facts, name) {
            self.facts.events.push(crate::ast::Event {
                binding: b,
                kind,
                span,
                place,
                dies_at,
                consumes,
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
                        R0Kind::Expr => self.walk_expr(c, Ctx::Value, None),
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
                        self.walk_expr(c, Ctx::Value, None);
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
            place: Vec::new(),
            dies_at: None,
            consumes: false,
        });
        // init:Eq 之後的 Expr 子節點
        let eq_pos = kids.iter().position(|&c| self.t.node(c).kind == R0Kind::Eq);
        let init = eq_pos.and_then(|p| {
            kids[p + 1..]
                .iter()
                .copied()
                .find(|&c| self.t.node(c).kind == R0Kind::Expr)
        });
        let mut bind_type = TypeClass::Unknown;
        if let Some(init) = init {
            if let Some((src, kind, src_span, place)) = expr_borrow_shape(self.t, init) {
                // 借鏈:`let r = &x;` / `let r = &mut x.a;` —— Referent 軌的活性錨點
                // (place 隨借用事件:S1,`&mut x.a` 與 `&mut x.b` 不相交)
                self.emit_ext(&src, src_span, kind, place, None, false);
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
                bind_type = TypeClass::Ref;
            } else if init_is_single_ident(self.t, init) {
                // S1 型別面(F-F):let-init 單 ident = 值拷貝或所有權轉移。
                // `let m2 = m;`(m: &mut)→ Move + consumes ⇒ m 死;
                // `let y = x;`(x: int)→ Read。
                let nid2 = nontrivia_children(self.t, init)[0];
                let name = name_of(self.t, nid2);
                let sp = self.t.node(nid2).span;
                let ty = expr_type(self.t, init, self.btypes, self.facts, self.scopes);
                match ty {
                    TypeClass::Ref => {
                        self.emit_ext(&name, sp, EvKind::Move, Vec::new(), None, true)
                    }
                    TypeClass::Int => self.emit(&name, sp, EvKind::Read),
                    TypeClass::Unknown => {
                        self.emit_ext(&name, sp, EvKind::Move, Vec::new(), None, false)
                    }
                }
            } else {
                self.walk_expr(init, Ctx::Value, None);
                bind_type = expr_type(self.t, init, self.btypes, self.facts, self.scopes);
            }
        }
        // 型別面記錄(供後續 `let m2 = m` / 實參 / 賦值的 consumes 判定)
        self.btypes.insert(b, bind_type);
        // 遮蔽語義(與 Rust 一致):RHS 先行 —— `let x = x + 1;` 的 x 指外層;
        // 走完 init 才把新綁定註冊進作用域。
        self.scopes.last_mut().unwrap().push(b);
    }

    /// 語句級表達式:賦值(`x = e` / `x.a = e` / `*p = e`)與一般表達式。
    fn walk_stmt_expr(&mut self, node: u32) {
        let kids = nontrivia_children(self.t, node);
        if let Some(eq) = kids.iter().position(|&c| self.t.node(c).kind == R0Kind::Eq) {
            let lhs = &kids[..eq];
            // LHS 分類(S1):單 ident → 寫(型別面:非 Copy 引用 ⇒ consumes);
            // 字段鏈 `x.a = …` → 寫基座的 place(部分寫不移動整體,consumes=false);
            // `*p` → 寫穿(Deref)。
            if lhs.len() == 1 && self.t.node(lhs[0]).kind == R0Kind::Ident {
                let name = name_of(self.t, lhs[0]);
                let sp = self.t.node(lhs[0]).span;
                let consumes = self.binding_type(&name) == Some(TypeClass::Ref);
                self.emit_ext(&name, sp, EvKind::Move, Vec::new(), None, consumes);
            } else if lhs.len() >= 3
                && lhs
                    .iter()
                    .step_by(2)
                    .all(|&c| self.t.node(c).kind == R0Kind::Ident)
                && lhs
                    .iter()
                    .skip(1)
                    .step_by(2)
                    .all(|&c| self.t.node(c).kind == R0Kind::Dot)
            {
                let base = lhs[0];
                let name = name_of(self.t, base);
                let sp = self.t.node(base).span;
                let place = field_chain(self.t, lhs, 0);
                self.emit_ext(&name, sp, EvKind::Move, place, None, false);
            } else if lhs.len() == 1 && self.t.node(lhs[0]).kind == R0Kind::UnaryExpr {
                self.walk_unary(lhs[0], None); // `*p = …`:p 上記 Deref(寫穿)
            } else {
                self.walk_flat(lhs, Ctx::Value, None);
            }
            self.walk_flat(&kids[eq + 1..], Ctx::Value, None);
        } else {
            self.walk_flat(&kids, Ctx::Value, None);
        }
    }

    /// 作用域內綁定的型別面(lookup 失敗 = None)。
    fn binding_type(&self, name: &str) -> Option<TypeClass> {
        let b = crate::ast::lookup_binding(self.scopes, self.facts, name)?;
        self.btypes.get(&b).copied()
    }

    /// 完整 Expr 節點(呼叫實參 / 括號內容):單 ident 實參按型別面
    /// (S1,F-C):Copy(int)→ Read;非 Copy(引用)→ Move + consumes;
    /// 未知 → Move(保守,不設 consumes)。
    fn walk_expr(&mut self, node: u32, ctx: Ctx, dies_at: Option<u32>) {
        let kids = nontrivia_children(self.t, node);
        if ctx == Ctx::CallArg && kids.len() == 1 && self.t.node(kids[0]).kind == R0Kind::Ident {
            let name = name_of(self.t, kids[0]);
            let sp = self.t.node(kids[0]).span;
            match self.binding_type(&name) {
                Some(TypeClass::Int) => self.emit(&name, sp, EvKind::Read),
                Some(TypeClass::Ref) => {
                    self.emit_ext(&name, sp, EvKind::Move, Vec::new(), None, true)
                }
                _ => self.emit_ext(&name, sp, EvKind::Move, Vec::new(), None, false),
            }
            return;
        }
        // `g(&mut x)`:實參是 Expr[UnaryExpr[…]] —— dies_at 貫穿至借用事件
        //(S1 臨時借用區域)。
        self.walk_flat(&kids, Ctx::Value, dies_at);
    }

    /// 扁平子節點序列的分類器:呼叫 / 字段 / 借用 / 解引用 / 讀。
    /// `dies_at`:call-arg 借用死點(僅經 `g(&mut x)` 路徑貫穿;其餘位置 None)。
    fn walk_flat(&mut self, kids: &[u32], ctx: Ctx, dies_at: Option<u32>) {
        let mut i = 0;
        while i < kids.len() {
            let c = kids[i];
            match self.t.node(c).kind {
                R0Kind::UnaryExpr => {
                    self.walk_unary(c, dies_at);
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
                        // (i+1..j) 內的 Expr 子節點 = 實參;ident 為 callee 不計。
                        // S1 臨時借用區域:F-B —— call-arg 借用死於呼叫返回
                        // (RParen 的 span.end),不再保守至作用域末。
                        let call_end = self.t.node(kids[j.min(kids.len())]).span.end;
                        for k in kids[i + 1..j.min(kids.len())].iter().copied() {
                            if self.t.node(k).kind == R0Kind::Expr {
                                self.walk_expr(k, Ctx::CallArg, Some(call_end));
                            } else if self.t.node(k).kind == R0Kind::UnaryExpr {
                                self.walk_unary(k, Some(call_end)); // `g(&mut x)`
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
                    // S1 place 敏感度:字段訪問基座 `x.f1` 的使用 = place [f1]
                    //(x.f1 與 x.f2 不相交;與整體 x 相交)。
                    let place = field_chain(self.t, kids, i);
                    let name = name_of(self.t, c);
                    let sp = self.t.node(c).span;
                    let kind = match ctx {
                        Ctx::CallArg => EvKind::Move,
                        _ => EvKind::Read,
                    };
                    self.emit_ext(&name, sp, kind, place, None, false);
                    i += 1;
                }
                R0Kind::Expr => {
                    self.walk_expr(c, Ctx::Value, None); // 括號表達式
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
                    self.walk_flat(&inner, Ctx::Value, None);
                    i = j + 1;
                }
                _ => {
                    i += 1; // 運算子 / 字面量 / 界符:不產事件
                }
            }
        }
    }

    /// 一元前綴:`&x` / `&mut x`(借用;place 可含字段鏈)/ `*p`(解引用)/ `!e`(透傳)。
    /// `dies_at`:呼叫實參位置的借用死於呼叫返回(S1 臨時借用區域);其餘位置 None。
    fn walk_unary(&mut self, node: u32, dies_at: Option<u32>) {
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
                // place:ident 之後的字段鏈(`&mut x.a` → [a])
                let pos = kids.iter().position(|&c| c == inner).unwrap();
                let place = field_chain(self.t, &kids[pos..], 0);
                let name = name_of(self.t, inner);
                let sp = self.t.node(inner).span;
                let kind = if op == Some(R0Kind::AmpMut) {
                    EvKind::BorrowMut
                } else {
                    EvKind::BorrowSh
                };
                self.emit_ext(&name, sp, kind, place, dies_at, false);
            }
            (Some(R0Kind::Star), Some(inner)) if self.t.node(inner).kind == R0Kind::Ident => {
                let name = name_of(self.t, inner);
                let sp = self.t.node(inner).span;
                self.emit(&name, sp, EvKind::Deref);
            }
            (_, Some(inner)) => match self.t.node(inner).kind {
                R0Kind::Expr => self.walk_expr(inner, Ctx::Value, None),
                R0Kind::UnaryExpr => self.walk_unary(inner, dies_at),
                _ => {}
            },
            _ => {}
        }
    }
}

/// `expr` 是否整體是單個 ident(供 let-init 型別面判定)。
fn init_is_single_ident(t: &R0Tree, expr: u32) -> bool {
    let kids = nontrivia_children(t, expr);
    kids.len() == 1 && t.node(kids[0]).kind == R0Kind::Ident
}

/// `expr` 是否整體是 `&x` / `&mut x`(借鏈偵測;Expr → UnaryExpr → [Amp, Ident])。
/// 回傳 (源名, 借用種類, 源 span, 字段鏈 place)。
fn expr_borrow_shape(t: &R0Tree, expr: u32) -> Option<(String, EvKind, Span, Vec<String>)> {
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
    let pos = uk.iter().position(|&c| c == ident)?;
    Some((
        name_of(t, ident),
        kind,
        t.node(ident).span,
        field_chain(t, &uk[pos..], 0),
    ))
}

/// R₀ 樹 → 事實層(對外入口;兩遍:聲明 → 事件)。
pub fn extract_r0(t: &R0Tree) -> Facts {
    let mut facts = Facts {
        bindings: Vec::new(),
        events: Vec::new(),
        links: Vec::new(),
        has_error_regions: t.has_error(),
        loops: collect_r0_loop_spans(t),
    };
    let root = t.root();
    let mut scopes: Vec<Vec<usize>> = vec![Vec::new()];
    let mut decl_site: DeclSite = DeclSite::new();
    let mut btypes: std::collections::HashMap<usize, TypeClass> = std::collections::HashMap::new();
    for c in t.node(root).children.clone() {
        match t.node(c).kind {
            R0Kind::FnItem | R0Kind::StructItem | R0Kind::Error => {
                collect_decls(t, c, &mut facts, &mut scopes, &mut decl_site, &mut btypes);
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
                btypes: &mut btypes,
            };
            e.walk_node(c);
        }
    }
    facts
}

/// 收集 R₀ 樹的迴圈語句 span(S2 回邊活性):WhileStmt / LoopStmt。
fn collect_r0_loop_spans(t: &R0Tree) -> Vec<Span> {
    let mut out = Vec::new();
    fn rec(t: &R0Tree, id: u32, out: &mut Vec<Span>) {
        if matches!(t.node(id).kind, R0Kind::WhileStmt | R0Kind::LoopStmt) {
            out.push(t.node(id).span);
        }
        for &c in &t.node(id).children {
            rec(t, c, out);
        }
    }
    rec(t, t.root(), &mut out);
    out
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

// ===========================================================================
// P4-2:生成式差分 —— fuzz agreement(by-construction 期望 × rustc × 三軌模型)
//
// 參照 RustSmith 的「生成時即知合法性」:`gen::gen_r0_semantic` 的樣本不是
// 「隨機文本問 rustc 怎麼說」,而是構造上已知期望判決。由此:
//   * 期望 ≠ rustc 判決 = BUG 候選(生成器構造推理錯誤,或真發現);
//   * 借用衝突碼下 Lexical 漏報 = 幾何保守下界律違反(逐樣本機械執法)。
// 判定權不轉移:gate 軌道(nll/referent)的 over/under 只記錄、不入庫,
// 歸因仍由 parity 註冊表單源承擔(ORACLE-TRACE §四)。
// ===========================================================================

/// 一次 fuzz agreement 失敗(律級)。
#[derive(Clone, Debug)]
pub struct FuzzFailure {
    /// 違反的律(精細標記,shrink 謂詞依此選取):
    /// `expectation-accept-rejected`(構造安全但 rustc 拒)/
    /// `expectation-reject-accepted`(構造衝突但 rustc 收)/
    /// `expectation-reject-wrong-code`(拒了,但碼不屬構造類)/
    /// `lower-bound`(借用衝突碼下 Lexical 漏報)/
    /// `out-of-scope`(樣本非純 R₀)/ `oracle-crash`(問不到判決)。
    pub law: String,
    /// 失敗樣本源碼。
    pub src: String,
    /// 說明。
    pub detail: String,
}

/// fuzz agreement 匯總(實測統計;ORACLE-TRACE 的 P4-2 段引用)。
#[derive(Clone, Debug)]
pub struct FuzzReport {
    /// 輪數。
    pub rounds: usize,
    /// 種子(可重現)。
    pub seed: u64,
    /// rustc 版本見證。
    pub rustc_version: String,
    /// by-construction 期望的分布。
    pub n_accept_expect: usize,
    /// 期望 = 借用衝突拒絕的樣本數。
    pub n_reject_borrow_expect: usize,
    /// 期望 = 移動拒絕的樣本數。
    pub n_reject_move_expect: usize,
    /// gate 軌道過報(MODEL-DIFF 候選;非失敗):`(nll, referent)`。
    pub gate_over: (usize, usize),
    /// gate 軌道漏報(同上):`(nll, referent)`。
    pub gate_under: (usize, usize),
    /// 律級失敗清單(空 = agreement 成立)。
    pub failures: Vec<FuzzFailure>,
}

/// 跑 N 輪生成式差分 agreement(外律 O-7 的引擎;bin/fuzz 併軌同用)。
pub fn fuzz_agreement(oracle: &dyn crate::oracle::Oracle, rounds: u64, seed: u64) -> FuzzReport {
    use crate::gen::{gen_r0_semantic, R0Expect, Rng};
    let mut rng = Rng::new(seed);
    let mut rep = FuzzReport {
        rounds: rounds as usize,
        seed,
        rustc_version: String::new(),
        n_accept_expect: 0,
        n_reject_borrow_expect: 0,
        n_reject_move_expect: 0,
        gate_over: (0, 0),
        gate_under: (0, 0),
        failures: Vec::new(),
    };
    for _ in 0..rounds {
        let s = gen_r0_semantic(&mut rng);
        match s.expect {
            R0Expect::Accept => rep.n_accept_expect += 1,
            R0Expect::RejectBorrow => rep.n_reject_borrow_expect += 1,
            R0Expect::RejectMove => rep.n_reject_move_expect += 1,
        }
        let orep = match oracle.check(&s.src) {
            Ok(r) => r,
            Err(e) => {
                rep.failures.push(FuzzFailure {
                    law: "oracle-crash".to_string(),
                    src: s.src.clone(),
                    detail: e.to_string(),
                });
                continue;
            }
        };
        rep.rustc_version = orep.rustc_version.clone();
        let m = model_check(&s.src);
        if !m.in_scope {
            rep.failures.push(FuzzFailure {
                law: "out-of-scope".to_string(),
                src: s.src.clone(),
                detail: format!("model 判範圍外:{:?}", m.scope_note),
            });
            continue;
        }
        let accept = orep.verdict.is_accept();
        let codes = orep.verdict.codes();
        let borrow_conflict = codes
            .iter()
            .any(|c| BORROW_CONFLICT_CODES.contains(&c.as_str()));
        // 律 1:by-construction 期望 × rustc 判決
        let ok1 = match s.expect {
            R0Expect::Accept => accept,
            R0Expect::RejectBorrow => !accept && borrow_conflict,
            R0Expect::RejectMove => !accept,
        };
        if !ok1 {
            let law = if s.expect == R0Expect::Accept {
                "expectation-accept-rejected"
            } else if accept {
                "expectation-reject-accepted"
            } else {
                "expectation-reject-wrong-code"
            };
            rep.failures.push(FuzzFailure {
                law: law.to_string(),
                src: s.src.clone(),
                detail: format!(
                    "expect={:?} rustc_accept={accept} codes={codes:?}",
                    s.expect
                ),
            });
        }
        // 律 2:幾何保守下界(逐樣本):借用衝突碼 ⇒ Lexical 必拒
        if !accept && borrow_conflict {
            let lx = m
                .tracks
                .iter()
                .find(|t| t.track == "lexical")
                .expect("lexical 軌");
            if !lx.reject {
                rep.failures.push(FuzzFailure {
                    law: "lower-bound".to_string(),
                    src: s.src.clone(),
                    detail: format!("rustc {codes:?} 而 Lexical 軌接受"),
                });
            }
        }
        // 統計:gate 軌道分歧(MODEL-DIFF 候選,非失敗)
        for label in ["nll", "referent"] {
            let tv = m
                .tracks
                .iter()
                .find(|t| t.track == label)
                .expect("軌道缺失");
            let is_nll = label == "nll";
            if tv.reject == accept {
                if tv.reject {
                    if is_nll {
                        rep.gate_over.0 += 1;
                    } else {
                        rep.gate_over.1 += 1;
                    }
                } else {
                    if is_nll {
                        rep.gate_under.0 += 1;
                    } else {
                        rep.gate_under.1 += 1;
                    }
                }
            }
        }
    }
    rep
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
        // E0506 現場:Referent 與 Nll 皆拒絕(rustc 同拒)—— P4-3 分工收斂
        //(let 形式借用端點 = 引用最後使用,兩軌同律;此前 Nll 漏報 = F-D,已消滅)
        let m = model_check(
            "fn f() {\n    let mut x = 5;\n    let r = &x;\n    x = 6;\n    let z = *r;\n}\n",
        );
        assert!(m.in_scope);
        let by = |l: &str| m.tracks.iter().find(|t| t.track == l).unwrap().reject;
        assert!(by("referent"));
        assert!(by("nll"));
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
