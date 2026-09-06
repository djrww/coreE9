//! §3.2 語義區間:liveness 投影 + 衝突圖(區間圖 ⊂ 弦圖 ⊂ 完美圖)。
//!
//! 這裡把 CL0 的 CST 投影到「事實層」:
//!   * 每個 `let` / 參數綁定 → 一個 storage(帶可變性與作用域 span);
//!   * 每次對綁定名的使用 → 一個事件(讀 / 移動 / 共享借用 / 可變借用 / 解引用),
//!     帶源碼 span(§4.4 錨定保持:事實攜帶可回跳的位址);
//!   * 三軌 liveness(報告 §3.2):
//!       - Lexical:  [scope.start, scope.end)  塊範圍;
//!       - Nll:      [event.start, 下一個 killer 事件.start)  最後使用點;
//!       - Referent: 借用事件延續到「被借引用變量」的最後使用(borrow's liveness)。
//!   * 相容性違反表 + 區間相交 ⇒ 衝突圖 G = (V, E_red)。
//!     借用錯誤的幾何本質(§3.2):語義區間違反了語法區間天生享有的 laminarity。

use crate::parse::{Kind, Tree};
use crate::span::Span;

/// 事件種類:對某個綁定的訪問/操作(§3.2 事實層的原子)。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EvKind {
    /// 綁定聲明(定義點)。
    Decl,
    /// 讀取(僅使用)。
    Read,
    /// 移動(所有權轉移)。
    Move,
    /// 共享借用 `&x`。
    BorrowSh,
    /// 可變借用 `&mut x`。
    BorrowMut,
    /// 解引用 `*x`。
    Deref,
}

impl EvKind {
    /// 事件的顯示標簽(診斷輸出用)。
    pub fn label(self) -> &'static str {
        match self {
            EvKind::Decl => "decl",
            EvKind::Read => "read",
            EvKind::Move => "move",
            EvKind::BorrowSh => "&",
            EvKind::BorrowMut => "&mut",
            EvKind::Deref => "*",
        }
    }
}

/// liveness 三軌(§3.2):同一棵樹、三種活躍區間語義。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Track {
    /// lexical:綁定存活到其作用域(block)末端。
    Lexical,
    /// nll:非詞法生命週期(killer = 覆蓋的聲明 / 移動)。
    Nll,
    /// referent:以借用鏈的**源綁定**為參照(refer 軌)。
    Referent,
}

impl Track {
    /// 軌道的顯示標簽。
    pub fn label(self) -> &'static str {
        match self {
            Track::Lexical => "lexical",
            Track::Nll => "nll",
            Track::Referent => "referent",
        }
    }
}

#[derive(Clone, Debug)]
/// 一個綁定(變量聲明):名字、聲明跨度、可變性、是否形參、作用域。
pub struct Binding {
    /// 綁定名。
    pub name: String,
    /// 綁定聲明的跨度。
    pub span: Span,
    /// 是否 `mut`(可變綁定 ⇒ `&mut` 合法)。
    pub mutable: bool,
    /// 是否為函數形參(參數作用域 = 函數體)。
    pub is_param: bool,
    /// 綁定所在作用域(最內層 block 的跨度)—— lexical 軌的端點來源。
    pub scope: Span,
}

/// 一個訪問事件(事實層):哪個綁定、什麼操作、在哪。
#[derive(Clone, Debug)]
pub struct Event {
    /// 事件的綁定索引(facts.bindings 下標;referent 軌會重定向)。
    pub binding: usize,
    /// 事件種類。
    pub kind: EvKind,
    /// 事件在源碼中的跨度。
    pub span: Span,
}

/// `let r = &x;`(或 `&mut x`)—— 借用鏈:refer 綁定 → 源綁定。
/// 借用鏈 `let r = &x;`:refer 綁定 → 源綁定(§3.2 referent 軌的拓撲)。
#[derive(Clone, Debug)]
pub struct BorrowLink {
    /// 引用綁定(refer)的索引。
    pub ref_binding: usize,
    /// 被引用綁定(源)的索引。
    pub src_binding: usize,
    /// 借用種類(BorrowSh 或 BorrowMut)。
    pub kind: EvKind,
    /// 鏈的跨度(整條 let 語句)。
    pub span: Span,
}

/// 事實層輸出(§3.2):綁定表 + 事件表 + 借用鏈 + 錯誤標記。
#[derive(Clone, Debug)]
pub struct Facts {
    /// 全部綁定(按聲明順序)。
    pub bindings: Vec<Binding>,
    /// 全部訪問事件。
    pub events: Vec<Event>,
    /// 全部借用鏈。
    pub links: Vec<BorrowLink>,
    /// 樹中是否存在 ERROR 區域(有則 liveness 分析降級為保守)。
    pub has_error_regions: bool,
}

/// 相容性違反表(§3.3):哪些事件對在「活躍區間相交」時構成紅邊。
/// 規則(與 rustc 的近似,足夠演示幾何):
///   * `&mut` 與除「自身」外的一切併發訪問(讀/移/借用/解引用)衝突;
///   * `&` 共享借用與移動衝突(借出的值被移動);
///   * 其餘(讀-讀、讀-&、&-&)合法。
pub fn conflicts(k1: EvKind, k2: EvKind) -> bool {
    let mut a = k1;
    let mut b = k2;
    if b as u8 > a as u8 {
        std::mem::swap(&mut a, &mut b);
    }
    matches!(
        (a, b),
        (EvKind::BorrowMut, EvKind::BorrowMut)
            | (EvKind::BorrowMut, EvKind::BorrowSh)
            | (EvKind::BorrowMut, EvKind::Read)
            | (EvKind::BorrowMut, EvKind::Move)
            | (EvKind::BorrowMut, EvKind::Deref)
            | (EvKind::BorrowSh, EvKind::Move)
    )
}

/// 從 CST 抽取事實層(具名節點樹上的結構遞歸;ERROR 區域不產事實,如實申報)。
pub fn extract(t: &Tree) -> Facts {
    let mut facts = Facts {
        bindings: Vec::new(),
        events: Vec::new(),
        links: Vec::new(),
        has_error_regions: t.has_error(),
    };
    let root = t.root();
    let mut scopes: Vec<Vec<usize>> = vec![Vec::new()]; // 作用域棧:每層 block 的綁定
                                                        // Root 是語法外殼(span 覆蓋全文,kind 不屬語法面):以它的子節點
                                                        // (項 / 錯誤區)進入語義面。修復:此前直接以 Root 進入,合法程式的
                                                        // 事實層恆為空 —— 由 test_law_semantic_extract_breadth 抓出。
    for c in t.node(root).children.clone() {
        walk_item_scope(t, c, &mut facts, &mut scopes);
    }
    facts
}

/// 聲明位置表:LetStmt / Param 節點 id → 綁定索引。
/// P4-1a(2026-09-06):事件遍行以本表直接定位綁定 —— 此前遍行期間作用域棧
/// 為空(collect_decls 已全部彈出),名稱查找永遠落空,事件層真空
/// (ORACLE-TRACE §一 發現 #1)。
type DeclSite = std::collections::HashMap<u32, usize>;

/// 由內到外、由後到先查綁定(遮蔽語義)。雙載體共用
/// (CL0 `ast` 與 R₀ `model` 的作用域棧同一查找契約)。
pub fn lookup_binding<'a>(scopes: &'a [Vec<usize>], facts: &'a Facts, name: &str) -> Option<usize> {
    for scope in scopes.iter().rev() {
        for &b in scope.iter().rev() {
            if facts.bindings[b].name == name {
                return Some(b);
            }
        }
    }
    None
}

fn child_of_kind(t: &Tree, node: u32, kind: Kind) -> Option<u32> {
    t.node(node)
        .children
        .iter()
        .copied()
        .find(|&c| t.node(c).kind == kind)
}

/// 收集聲明(綁定)。遍歷 fn 項與所有塊。
fn collect_decls(
    t: &Tree,
    node: u32,
    facts: &mut Facts,
    scopes: &mut Vec<Vec<usize>>,
    param_scope: bool,
    decl_site: &mut DeclSite,
) {
    let n = t.node(node);
    match n.kind {
        Kind::FnItem => {
            // 半截 fn(無 body):如實跳過(§2.3 全化 —— 語義面不 panic)。
            let Some(body) = child_of_kind(t, node, Kind::Block) else {
                return;
            };
            scopes.push(Vec::new());
            for &c in &n.children {
                if t.node(c).kind == Kind::Param {
                    // 半截參數(無名稱):如實跳過。
                    let Some(name_node) = child_of_kind(t, c, Kind::Ident) else {
                        continue;
                    };
                    // 參數綁定在 fn body 作用域
                    let b = facts.bindings.len();
                    facts.bindings.push(Binding {
                        name: name_of(t, name_node),
                        span: t.node(name_node).span,
                        mutable: false,
                        is_param: true,
                        scope: t.node(body).span,
                    });
                    decl_site.insert(c, b);
                    scopes.last_mut().unwrap().push(b);
                }
            }
            collect_decls(t, body, facts, scopes, false, decl_site);
            scopes.pop();
        }
        Kind::Block => {
            scopes.push(Vec::new());
            for c in t.node(node).children.clone() {
                let cn = t.node(c);
                match cn.kind {
                    Kind::LetStmt => {
                        // 半截 let(無綁定名):如實跳過。
                        let Some(name_node) = child_of_kind(t, c, Kind::Ident) else {
                            continue;
                        };
                        let b = facts.bindings.len();
                        let mut mutable = false;
                        // let [mut]
                        for &cc in &t.node(c).children {
                            if t.node(cc).kind == Kind::MutKw {
                                mutable = true;
                            }
                        }
                        facts.bindings.push(Binding {
                            name: name_of(t, name_node),
                            span: t.node(name_node).span,
                            mutable,
                            is_param: false,
                            scope: t.node(node).span,
                        });
                        decl_site.insert(c, b);
                        scopes.last_mut().unwrap().push(b);
                        // rhs 中的嵌套塊(block-expr)仍要掃
                        scan_nested_blocks(t, c, facts, scopes, decl_site);
                    }
                    Kind::IfStmt | Kind::WhileStmt => {
                        // if / while 的子塊
                        for cc in t.node(c).children.clone() {
                            if matches!(t.node(cc).kind, Kind::Block | Kind::IfStmt) {
                                collect_decls(t, cc, facts, scopes, false, decl_site);
                            }
                        }
                    }
                    _ => {}
                }
            }
            scopes.pop();
        }
        _ => {}
    }
    let _ = param_scope;
}

fn scan_nested_blocks(
    t: &Tree,
    node: u32,
    facts: &mut Facts,
    scopes: &mut Vec<Vec<usize>>,
    decl_site: &mut DeclSite,
) {
    for c in t.node(node).children.clone() {
        if t.node(c).kind == Kind::Block {
            collect_decls(t, c, facts, scopes, false, decl_site);
        } else if t.node(c).kind == Kind::Expr {
            scan_nested_blocks(t, c, facts, scopes, decl_site);
        } else if t.node(c).kind == Kind::CallExpr {
            // 參數中的 block-expr
            for cc in t.node(c).children.clone() {
                if t.node(cc).kind == Kind::Expr || t.node(cc).kind == Kind::Block {
                    scan_nested_blocks(t, cc, facts, scopes, decl_site);
                }
            }
        }
    }
}

/// 事件抽取。ctx:事件對名稱的「使用分類」。
#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)] // Lhs: R₀ 才有賦值;CL0 無(保留語義槽)
enum Ctx {
    Value,    // 讀
    CallArg,  // 移動語義 `f(x)`
    Lhs,      // (R₀ 才有賦值;CL0 無 — 保留)
    Borrowed, // `&x` 內部(不另外記 read)
}

fn walk_item_scope(t: &Tree, node: u32, facts: &mut Facts, scopes: &mut Vec<Vec<usize>>) {
    // 第一遍:decls(順帶建立 decl_site:節點 → 綁定索引)
    let mut decl_site: DeclSite = DeclSite::new();
    collect_decls(t, node, facts, scopes, false, &mut decl_site);
    // 第二遍:events(需要兩遍:事件要綁定到已知 storage)。
    // P4-1a:遍行期間真正管理作用域棧(FnItem 參數層 / Block 層 / let 後註冊),
    // 並以 decl_site 定位宣告 —— 修復「事件層真空」(ORACLE-TRACE §一 發現 #1)。
    let mut e = EventCollector {
        t,
        facts,
        scopes,
        decl_site: &decl_site,
    };
    e.walk_node(node, Ctx::Value);
}

struct EventCollector<'a> {
    t: &'a Tree,
    facts: &'a mut Facts,
    scopes: &'a mut Vec<Vec<usize>>,
    decl_site: &'a DeclSite,
}

impl<'a> EventCollector<'a> {
    fn emit(&mut self, name: &str, span: Span, kind: EvKind) -> bool {
        if let Some(b) = lookup_binding(self.scopes, self.facts, name) {
            self.facts.events.push(Event {
                binding: b,
                kind,
                span,
            });
            true
        } else {
            false
        }
    }

    fn walk_node(&mut self, node: u32, ctx: Ctx) {
        let kind = self.t.node(node).kind;
        match kind {
            Kind::FnItem => {
                if let Some(body) = child_of_kind(self.t, node, Kind::Block) {
                    // 參數作用域(fn body 層)
                    self.scopes.push(Vec::new());
                    for c in self.t.node(node).children.clone() {
                        if self.t.node(c).kind == Kind::Param {
                            if let Some(&b) = self.decl_site.get(&c) {
                                self.scopes.last_mut().unwrap().push(b);
                            }
                        }
                    }
                    self.walk_node(body, Ctx::Value);
                    self.scopes.pop();
                }
            }
            Kind::Block => {
                self.scopes.push(Vec::new());
                for c in self.t.node(node).children.clone() {
                    if self.t.node(c).kind == Kind::LetStmt {
                        self.walk_let(c); // init 走完才註冊綁定(遮蔽語義,見 walk_let)
                    } else {
                        self.walk_node(c, Ctx::Value);
                    }
                }
                self.scopes.pop();
            }
            Kind::LetStmt => {
                // 語句位置的 let 由 Block 臂分派到 walk_let;此臂只服務
                // 非語句位置的殘餘路徑(如 ERROR 吸收區邊緣),保守透傳。
                self.walk_let(node);
            }
            Kind::IfStmt | Kind::WhileStmt => {
                for c in self.t.node(node).children.clone() {
                    match self.t.node(c).kind {
                        Kind::Expr => self.walk_node(c, Ctx::Value),
                        Kind::Block | Kind::IfStmt => self.walk_node(c, Ctx::Value),
                        _ => {}
                    }
                }
            }
            Kind::ExprStmt => {
                for c in self.t.node(node).children.clone() {
                    if self.t.node(c).kind == Kind::Expr {
                        self.walk_node(c, Ctx::Value);
                    }
                }
            }
            Kind::Expr => {
                for c in self.t.node(node).children.clone() {
                    match self.t.node(c).kind {
                        Kind::UnaryExpr | Kind::CallExpr | Kind::Block => {
                            self.walk_node(c, Ctx::Value)
                        }
                        Kind::Ident => {
                            let name = name_of(self.t, c);
                            let sp = self.t.node(c).span;
                            match ctx {
                                Ctx::CallArg => {
                                    self.emit(&name, sp, EvKind::Move);
                                }
                                _ => {
                                    self.emit(&name, sp, EvKind::Read);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Kind::UnaryExpr => {
                let mut is_borrow = false;
                let mut is_mut = false;
                let mut is_deref = false;
                let mut inner = None;
                for c in self.t.node(node).children.clone() {
                    match self.t.node(c).kind {
                        Kind::Amp => is_borrow = true,
                        Kind::MutKw => is_mut = true,
                        Kind::Star => is_deref = true,
                        Kind::Ident | Kind::Expr | Kind::CallExpr | Kind::Block => inner = Some(c),
                        _ => {}
                    }
                }
                if let Some(inner) = inner {
                    if is_borrow {
                        // `&x` / `&mut x`(inner 應是 ident):在源名稱上記借用事件
                        if self.t.node(inner).kind == Kind::Ident {
                            let name = name_of(self.t, inner);
                            let sp = self.t.node(inner).span;
                            self.emit(
                                &name,
                                sp,
                                if is_mut {
                                    EvKind::BorrowMut
                                } else {
                                    EvKind::BorrowSh
                                },
                            );
                            // 借用的主體不再當 read 計
                        } else {
                            self.walk_node(inner, Ctx::Value);
                        }
                    } else if is_deref {
                        // `*p`:在 p 上記解引用事件
                        if self.t.node(inner).kind == Kind::Ident {
                            let name = name_of(self.t, inner);
                            let sp = self.t.node(inner).span;
                            self.emit(&name, sp, EvKind::Deref);
                        } else {
                            self.walk_node(inner, Ctx::Value);
                        }
                    } else {
                        self.walk_node(inner, Ctx::Value);
                    }
                }
            }
            Kind::CallExpr => {
                // 被調用者(callee ident)不計;參數按移動語義
                let mut first = true;
                for c in self.t.node(node).children.clone() {
                    match self.t.node(c).kind {
                        Kind::Expr => {
                            // 參數表達式:直接 ident → Move;其它 → 讀取內部使用
                            if self.t.node(c).children.len() == 1 {
                                let ch = self.t.node(self.t.node(c).children[0]).kind;
                                if ch == Kind::Ident {
                                    let name = name_of(self.t, self.t.node(c).children[0]);
                                    let sp = self.t.node(c).span;
                                    self.emit(&name, sp, EvKind::Move);
                                    continue;
                                }
                            }
                            self.walk_node(c, Ctx::CallArg);
                        }
                        Kind::Ident if first => {
                            first = false; // callee
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// `let [mut] IDENT [= expr];` —— P4-1a 重寫:Decl 以 decl_site 定位;
    /// 借用初始化同時發**借用事件**(舊碼只建鏈不發事件 —— 真空下未曝光)
    /// 且鏈 span 取源 ident(修 ORACLE-TRACE §一 發現 #2:配對死代碼);
    /// init 走完才註冊綁定(遮蔽語義:`let x = x + 1;` 的 RHS 指外層)。
    fn walk_let(&mut self, node: u32) {
        let mut expr_node = None;
        let mut name_node = None;
        for c in self.t.node(node).children.clone() {
            match self.t.node(c).kind {
                Kind::Ident => {
                    if name_node.is_none() {
                        name_node = Some(c);
                    }
                }
                Kind::Expr => expr_node = Some(c),
                _ => {}
            }
        }
        let Some(nn) = name_node else {
            return; // 半截 let:如實跳過
        };
        let Some(&b) = self.decl_site.get(&node) else {
            return;
        };
        let span = self.t.node(nn).span;
        self.facts.events.push(Event {
            binding: b,
            kind: EvKind::Decl,
            span,
        });
        if let Some(en) = expr_node {
            if let Some((src, borrow_kind, src_span)) = self.walk_expr_for_borrow(en) {
                // 借用事件落在源綁定上;借鏈錨定同一 span(Referent 軌配對)
                self.emit(&src, src_span, borrow_kind);
                if let Some(sb) = lookup_binding(self.scopes, self.facts, &src) {
                    self.facts.links.push(BorrowLink {
                        ref_binding: b,
                        src_binding: sb,
                        kind: borrow_kind,
                        span: src_span,
                    });
                }
            } else {
                self.walk_node(en, Ctx::Value);
            }
        }
        self.scopes.last_mut().unwrap().push(b);
    }

    /// 若 expr 是 `&x` / `&mut x`,返回 (源名, 借用種類, 源 ident span)。
    fn walk_expr_for_borrow(&mut self, en: u32) -> Option<(String, EvKind, Span)> {
        // Expr → UnaryExpr → [Amp, (Mut), Ident]
        let mut unary = None;
        for c in self.t.node(en).children.clone() {
            if self.t.node(c).kind == Kind::UnaryExpr {
                unary = Some(c);
            }
        }
        let un = unary?;
        let mut kind: Option<EvKind> = None;
        let mut name = None;
        let mut src_span = None;
        for c in self.t.node(un).children.clone() {
            match self.t.node(c).kind {
                Kind::Amp => kind = Some(EvKind::BorrowSh),
                Kind::MutKw => kind = Some(EvKind::BorrowMut),
                Kind::Ident => {
                    name = Some(name_of(self.t, c));
                    src_span = Some(self.t.node(c).span);
                }
                _ => {}
            }
        }
        Some((name?, kind?, src_span?))
    }
}

/// 名稱文本:錨定取得(節點 span 直接切源碼 → §4.4 錨定保持)。
pub fn span_text(t: &Tree, sp: Span) -> String {
    t.src[sp.start as usize..sp.end as usize].to_string()
}

fn name_of(t: &Tree, node: u32) -> String {
    span_text(t, t.node(node).span)
}

// ===========================================================================
// 三軌 liveness 與衝突圖
// ===========================================================================

/// 活躍區間(半開)—— §3.3 衝突圖的頂點幾何。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Interval {
    /// 左端點(含)。
    pub start: u32,
    /// 右端點(不含)。
    pub end: u32,
}

impl Interval {
    /// 兩區間是否重疊(半開:相接不算重疊;T2 排序鍵的依據)。
    pub fn overlaps(&self, o: &Interval) -> bool {
        self.start < o.end && o.start < self.end
    }
}

/// 每個綁定在給定軌道下的活躍區間集合(索引與 `facts.events` 同序:
/// 每事件恰一區間;事件表直接取 `facts.events`,不再克隆)。
/// Nll 軌:killer = Span 更大的 Decl 與 Move(值被覆蓋 / 移動)。
pub fn intervals(facts: &Facts, track: Track) -> Vec<Vec<Interval>> {
    let n = facts.bindings.len();
    let mut out: Vec<Vec<Interval>> = vec![Vec::new(); n];
    for ev in facts.events.iter() {
        let b = ev.binding;
        let mut it = Interval {
            start: ev.span.start,
            end: facts.bindings[b].scope.end,
        };
        match track {
            Track::Lexical => {
                it.start = facts.bindings[b].scope.start;
                it.end = facts.bindings[b].scope.end;
            }
            Track::Nll => {
                // 下一個 killer(Decl/Move)的 start —— 借用與讀取不終止值生命週期
                let mut end = facts.bindings[b].scope.end;
                for other in &facts.events {
                    if other.binding == b
                        && other.span.start > ev.span.start
                        && matches!(other.kind, EvKind::Decl | EvKind::Move)
                    {
                        end = end.min(other.span.start);
                        break;
                    }
                }
                it.end = end.max(it.start);
            }
            Track::Referent => {
                if matches!(ev.kind, EvKind::BorrowSh | EvKind::BorrowMut) {
                    // 借用事件:活躍到「被借引用」的最後使用(borrow's liveness)。
                    // ⚠ 2026-09-06 修正(ORACLE-TRACE 發現 #3):終點初值原為
                    // `scope.end`,再 `.max(使用點)` —— 只能伸、不能縮,借用活性
                    // 因而恆延至作用域末(過度保守)。正確語義:最後使用點;
                    // 無使用 ⇒ 立即死亡(與 NLL「未用借用即死」一致)。
                    // 此前 CL0 事件層真空(extract 產 0 事件),該缺陷從未曝光。
                    if let Some(link) = facts
                        .links
                        .iter()
                        .find(|l| l.src_binding == b && l.span == ev.span)
                    {
                        let mut end = link.span.end;
                        for other in &facts.events {
                            if other.binding == link.ref_binding
                                && other.span.start >= link.span.start
                            {
                                end = end.max(other.span.end);
                            }
                        }
                        it.end = end.max(it.start);
                    }
                }
            }
        }
        out[b].push(it);
    }
    out
}

/// 紅邊集合(§3.3 衝突圖的邊):同一綁定、區間相交、相容性被違反。
/// 紅邊(§3.3 衝突邊):同一綁定、活躍區間相交、相容性被違反。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedEdge {
    /// 事件 a 的索引(facts.events)。
    pub a: usize,
    /// 事件 b 的索引(facts.events)。
    pub b: usize,
    /// 所屬綁定。
    pub binding: usize,
    /// 兩事件活躍區間的交疊跨度(修法菜單的定位依據)。
    pub span: Span,
}

/// 計算給定軌道下的紅邊集合(衝突圖;空圖 ⇒ 幾何收斂 §3.5)。
pub fn red_edges(facts: &Facts, track: Track) -> Vec<RedEdge> {
    let ivs = intervals(facts, track);
    let events = &facts.events;
    let mut out = Vec::new();
    for (b, _) in facts.bindings.iter().enumerate() {
        let mut evs: Vec<usize> = (0..events.len())
            .filter(|&i| events[i].binding == b)
            .collect();
        evs.sort_by_key(|&i| events[i].span.start);
        for x in 0..evs.len() {
            for y in (x + 1)..evs.len() {
                let i = evs[x];
                let j = evs[y];
                let (ei, ej) = (&events[i], &events[j]);
                if !conflicts(ei.kind, ej.kind) {
                    continue;
                }
                if ivs[b][x].overlaps(&ivs[b][y]) {
                    out.push(RedEdge {
                        a: i,
                        b: j,
                        binding: b,
                        span: Span::new(
                            ei.span.start.min(ej.span.start),
                            ei.span.end.max(ej.span.end),
                        ),
                    });
                }
            }
        }
    }
    out
}

/// 衝突圖的頂點 = 事件;邊 = 紅邊。返回 (頂點數, 邊數)。
pub fn conflict_graph_shape(facts: &Facts, track: Track) -> (usize, usize) {
    let edges = red_edges(facts, track);
    let mut verts = std::collections::BTreeSet::new();
    for e in &edges {
        verts.insert(e.a);
        verts.insert(e.b);
    }
    (verts.len(), edges.len())
}

// ===========================================================================
// §3.3 定理 T2 的實例驗證:區間圖 = 弦圖 = 完美圖(χ = ω)
// ===========================================================================

/// 掃描線求最大團 ω(O(n log n)):排序端點,掃描重疊計數。
pub fn max_clique(intervals: &[Interval]) -> usize {
    let mut ends: Vec<(u32, bool)> = Vec::new();
    for it in intervals {
        ends.push((it.start, false)); // 開始
        ends.push((it.end, true)); // 結束(半開:先結束後開始,不重疊)
    }
    // 半開區間:同一位置先結束、後開始(相接不算重疊)
    ends.sort_by_key(|&(p, is_end)| (p, !is_end));
    let mut cur = 0usize;
    let mut best = 0usize;
    for &(_, is_end) in &ends {
        if is_end {
            cur -= 1;
        } else {
            cur += 1;
            best = best.max(cur);
        }
    }
    best
}

/// 貪婪著色(按左端點):區間圖上貪婪著色恰用 ω 種顏色 ⇒ χ = ω。
pub fn greedy_chromatic(intervals: &[Interval]) -> usize {
    let mut sorted: Vec<&Interval> = intervals.iter().collect();
    sorted.sort_by_key(|it| it.start);
    let n = sorted.len();
    let mut colors = vec![usize::MAX; n];
    let mut maxc = 0usize;
    for i in 0..n {
        let mut used = std::collections::BTreeSet::new();
        for j in 0..i {
            if colors[j] != usize::MAX && sorted[i].overlaps(sorted[j]) {
                used.insert(colors[j]);
            }
        }
        let mut c = 0usize;
        while used.contains(&c) {
            c += 1;
        }
        colors[i] = c;
        maxc = maxc.max(c + 1);
    }
    maxc
}
