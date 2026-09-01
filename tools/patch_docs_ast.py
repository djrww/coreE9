# -*- coding: utf-8 -*-
# 為 ast.rs 補 missing_docs(逐項校驗 count==1,防靜默失敗)
pairs = []

pairs.append((
r"""pub enum EvKind {
    Decl,
    Read,
    Move,
    BorrowSh,
    BorrowMut,
    Deref,
}""",
r"""/// 事件種類:對某個綁定的訪問/操作(§3.2 事實層的原子)。
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
}"""))

pairs.append(("impl EvKind {\n    pub fn label(self) -> &'static str {",
              "impl EvKind {\n    /// 事件的顯示標簽(診斷輸出用)。\n    pub fn label(self) -> &'static str {"))

pairs.append((
r"""pub enum Track {
    Lexical,
    Nll,
    Referent,
}""",
r"""/// liveness 三軌(§3.2):同一棵樹、三種活躍區間語義。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Track {
    /// lexical:綁定存活到其作用域(block)末端。
    Lexical,
    /// nll:非詞法生命週期(killer = 覆蓋的聲明 / 移動)。
    Nll,
    /// referent:以借用鏈的**源綁定**為參照(refer 軌)。
    Referent,
}"""))

pairs.append(("impl Track {\n    pub fn label(self) -> &'static str {",
              "impl Track {\n    /// 軌道的顯示標簽。\n    pub fn label(self) -> &'static str {"))

pairs.append((
r"""pub struct Binding {
    pub name: String,
    pub span: Span,
    pub mutable: bool,
    pub is_param: bool,""",
r"""/// 一個綁定(變量聲明):名字、聲明跨度、可變性、是否形參、作用域。
pub struct Binding {
    /// 綁定名。
    pub name: String,
    /// 綁定聲明的跨度。
    pub span: Span,
    /// 是否 `mut`(可變綁定 ⇒ `&mut` 合法)。
    pub mutable: bool,
    /// 是否為函數形參(參數作用域 = 函數體)。
    pub is_param: bool,"""))

pairs.append((
r"""pub struct Event {
    pub binding: usize,
    pub kind: EvKind,
    pub span: Span,
}""",
r"""/// 一個訪問事件(事實層):哪個綁定、什麼操作、在哪。
#[derive(Clone, Debug)]
pub struct Event {
    /// 事件的綁定索引(facts.bindings 下標;referent 軌會重定向)。
    pub binding: usize,
    /// 事件種類。
    pub kind: EvKind,
    /// 事件在源碼中的跨度。
    pub span: Span,
}"""))

pairs.append((
r"""pub struct BorrowLink {
    pub ref_binding: usize,
    pub src_binding: usize,
    pub kind: EvKind, // BorrowSh | BorrowMut
    pub span: Span,
}""",
r"""/// 借用鏈 `let r = &x;`:refer 綁定 → 源綁定(§3.2 referent 軌的拓撲)。
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
}"""))

pairs.append((
r"""pub struct Facts {
    pub bindings: Vec<Binding>,
    pub events: Vec<Event>,
    pub links: Vec<BorrowLink>,
    pub has_error_regions: bool,
}""",
r"""/// 事實層輸出(§3.2):綁定表 + 事件表 + 借用鏈 + 錯誤標記。
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
}"""))

pairs.append((
r"""pub struct Interval {
    pub start: u32,
    pub end: u32,
}""",
r"""/// 活躍區間(半開)—— §3.3 衝突圖的頂點幾何。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Interval {
    /// 左端點(含)。
    pub start: u32,
    /// 右端點(不含)。
    pub end: u32,
}"""))

pairs.append(("impl Interval {\n    pub fn overlaps(&self, o: &Interval) -> bool {",
              "impl Interval {\n    /// 兩區間是否重疊(半開:相接不算重疊;T2 排序鍵的依據)。\n    pub fn overlaps(&self, o: &Interval) -> bool {"))

pairs.append((
r"""pub struct RedEdge {
    pub a: usize, // 事件索引(facts.events)
    pub b: usize,
    pub binding: usize,
    pub span: Span,
}""",
r"""/// 紅邊(§3.3 衝突邊):同一綁定、活躍區間相交、相容性被違反。
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
}"""))

pairs.append(("pub fn red_edges(facts: &Facts, track: Track) -> Vec<RedEdge> {",
              "/// 計算給定軌道下的紅邊集合(衝突圖;空圖 ⇒ 幾何收斂 §3.5)。\npub fn red_edges(facts: &Facts, track: Track) -> Vec<RedEdge> {"))

path = 'src/ast.rs'
s = open(path, encoding='utf-8').read()
for old, new in pairs:
    n = s.count(old)
    assert n == 1, (old[:60], n)
    s = s.replace(old, new)
open(path, 'w', encoding='utf-8').write(s)
print('ok ast.rs', len(pairs), 'patches')
