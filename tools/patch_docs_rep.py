# -*- coding: utf-8 -*-
# 補 rep.rs 與 l9newman.rs 的 missing_docs(逐項校驗 count==1)
pairs_rep = []

pairs_rep.append((
r"""pub enum K {
    /// 可變訪問(write / &mut 引用本體)
    Mut,
    /// 共享訪問(read / & 引用本體)
    Sh,
}""",
r"""/// 訪問種類(重寫系統狀態的原子標記:可變 / 共享)。
pub enum K {
    /// 可變訪問(write / `&mut` 引用本體)。
    Mut,
    /// 共享訪問(read / `&` 引用本體)。
    Sh,
}"""))

pairs_rep.append(("impl K {\n    pub fn label(self) -> &'static str {",
                  "impl K {\n    /// 訪問種類的顯示標簽。\n    pub fn label(self) -> &'static str {"))

pairs_rep.append((
r"""pub struct Ev {
    pub id: u32,
    pub storage: u32,
    pub kind: K,
    pub it: Interval,
}""",
r"""/// 抽象事件:一個訪問的區間配置(重寫系統的最小對象)。
pub struct Ev {
    /// 事件 id(穩定;AState::new 自動補齊)。
    pub id: u32,
    /// 所屬存儲/綁定(storage 分組 ⇒ 同一綁定內的衝突)。
    pub storage: u32,
    /// 訪問種類。
    pub kind: K,
    /// 活躍區間(半開)。
    pub it: Interval,
}"""))

pairs_rep.append((
r"""pub struct AState {
    pub evs: Vec<Ev>,
    /// 已標記為運行期借用的邊(從 E_red 移出;事實層記 runtime-borrow 標記)。
    pub runtime: Vec<(u32, u32)>,""",
r"""/// 抽象狀態:事件的多集 + 運行期借用邊 + 日誌(§4.1 的狀態空間元素)。
pub struct AState {
    /// 事件配置。
    pub evs: Vec<Ev>,
    /// 已標記為運行期借用的邊(從 E_red 移出;事實層記 runtime-borrow 標記)。
    pub runtime: Vec<(u32, u32)>,"""))

pairs_rep.append(("impl AState {\n    pub fn new(evs: Vec<Ev>) -> AState {",
                  "impl AState {\n    /// 從事件配置構造狀態(自動補齊事件 id;日誌為空)。\n    pub fn new(evs: Vec<Ev>) -> AState {"))

pairs_rep.append(("    pub fn is_normal_form(&self) -> bool {",
                  "    /// 正規形判定:無紅邊(幾何收斂 §3.5 的終點)。\n    pub fn is_normal_form(&self) -> bool {"))

pairs_rep.append((
r"""pub enum Rule {
    R1Shorten(u32, u32), // (事件 id, 新右端點)
    R2Split(u32, u32),   // (事件 id, 切點)
    R3Swap(u32, u32),    // (事件 a, 事件 b)—— 交換區間(重排語句)
    R4Runtime(u32, u32), // (事件 a, 事件 b)—— 標記為運行期借用
}""",
r"""/// 菜單規則(§4.1):狀態上可施的原子修法。
pub enum Rule {
    /// 縮短:把事件的右端點左移(事件 id, 新右端點)。
    R1Shorten(u32, u32),
    /// 分裂:在切點把一個事件分裂為兩個(事件 id, 切點)。
    R2Split(u32, u32),
    /// 交換兩個事件的區間(重排語句)(事件 a, 事件 b)。
    R3Swap(u32, u32),
    /// 標記為運行期借用(事件 a, 事件 b)。
    R4Runtime(u32, u32),
}"""))

pairs_rep.append(("impl Rule {\n    pub fn label(&self) -> String {",
                  "impl Rule {\n    /// 規則的顯示標簽(診斷輸出用)。\n    pub fn label(&self) -> String {"))

pairs_rep.append((
r"""pub enum Policy {
    /// guard 通過 ⟺ μ 嚴格遞減(帶側條件的菜單 = 報告的封閉菜單紀律)。
    Guarded,
    /// 無側條件(機械展示:為什麼側條件是定律的必要載體)。
    Raw,
}""",
r"""/// 施用策略:是否要求 μ 嚴格遞減(§4.2 側條件)。
pub enum Policy {
    /// guard 通過 ⟺ μ 嚴格遞減(帶側條件的菜單 = 報告的封閉菜單紀律)。
    Guarded,
    /// 無側條件(機械展示:為什麼側條件是定律的必要載體)。
    Raw,
}"""))

pairs_rep.append(("impl Menu {\n    pub fn label(&self) -> &'static str {",
                  "impl Menu {\n    /// 菜單的顯示標簽。\n    pub fn label(&self) -> &'static str {"))

path = 'src/rep.rs'
s = open(path, encoding='utf-8').read()
for old, new in pairs_rep:
    assert s.count(old) == 1, (old[:50], s.count(old))
    s = s.replace(old, new)
open(path, 'w', encoding='utf-8').write(s)
print('ok rep.rs', len(pairs_rep))

# ---------------- l9newman.rs ----------------
pairs_l9 = []

pairs_l9.append((
r"""#[derive(Clone, Debug)]
pub struct NewmanReport {
    pub menu: Menu,
    pub policy: Policy,
    pub states: usize,
    pub l8_violations: Vec<(AState, AState, Rule)>,
    pub critical_pairs: usize,
    pub non_joinable: Vec<(AState, Rule, Rule, AState, AState)>,
    pub unique_nf_states: usize,
    pub multi_nf: Vec<(AState, Vec<AState>)>,
    pub conclusion: &'static str,
}""",
r"""/// Newman 通道的機械報告(§4.2–4.3 的驗證輸出;所有字段皆可機械復算)。
#[derive(Clone, Debug)]
pub struct NewmanReport {
    /// 被驗證的菜單。
    pub menu: Menu,
    /// 施用策略。
    pub policy: Policy,
    /// 窮舉的狀態數。
    pub states: usize,
    /// L8 遞減違反(源狀態, 目標狀態, 規則)。
    pub l8_violations: Vec<(AState, AState, Rule)>,
    /// 已檢查的臨界對數。
    pub critical_pairs: usize,
    /// 不可回合的臨界對(源, r1, r2, 分支 a, 分支 b)。
    pub non_joinable: Vec<(AState, Rule, Rule, AState, AState)>,
    /// 唯一正規形的狀態數。
    pub unique_nf_states: usize,
    /// 多正規形狀態(源, 全部正規形)—— WCR 反例的載體。
    pub multi_nf: Vec<(AState, Vec<AState>)>,
    /// 機器給出的結論(converges / WCR 違反)。
    pub conclusion: &'static str,
}"""))

pairs_l9.append((
r"""pub fn newman_check(
    menu: Menu,
    policy: Policy,
    n_events: usize,
    max_coord: u32,
    depth: usize,
) -> NewmanReport {""",
r"""/// 機械 Newman 檢查:對菜單 × 政策做窮舉狀態空間上的
/// L8(測度遞減)+ 臨界對可合流(§4.3)雙重驗證,輸出報告。
/// 前提(報告 §4.2):μ 良基 ⇒ SN;SN ∧ WCR ⇒ CR ⇒ 唯一正規形。
pub fn newman_check(
    menu: Menu,
    policy: Policy,
    n_events: usize,
    max_coord: u32,
    depth: usize,
) -> NewmanReport {"""))

path = 'src/l9newman.rs'
s = open(path, encoding='utf-8').read()
for old, new in pairs_l9:
    assert s.count(old) == 1, (old[:50], s.count(old))
    s = s.replace(old, new)
open(path, 'w', encoding='utf-8').write(s)
print('ok l9newman.rs', len(pairs_l9))
