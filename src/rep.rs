//! §4 重寫系統 —— 修法菜單(repair menu)的終止與合流。
//!
//! ARS (A, →):A = 「活躍區間配置」(幾何事實層的狀態),→ = 菜單規則單步。
//! 規則直接作用於**事實層的區間**(對應報告:judge 的 L5 路徑把錯誤現場歸約
//! 為 .cl 事實;修法輸出是「區間手術 + 對應源碼建議」)。
//!
//! 測度 μ(P) = (|E_red(P)|, |Err_rustc(P)|)(字典序;本演示中 rustc 面為
//! oracle 範疇,記 0 —— 判定權不轉移,§6.3)。
//!
//! **L8(遞減律)**:菜單每條規則的每個合法施用都嚴格遞減 μ。
//! 機械形式:每個規則帶 guard,guard 通過 ⟺ μ 嚴格遞減 —— 由
//! `apply_guarded` 檢查;另保留 `Policy::Raw`(無 guard)以機械找出
//! 「無 guard 菜單違反 L8」的反例(誠實申報:側條件不是裝飾,是定律的載體)。
//!
//! **L9(合流唯一)**:終止 + 局部合流 ⇒ 唯一正規形(Newman 引理)。
//! `CommutativeTrim` 菜單(R1 規範修剪)被機械驗證:所有臨界對可回合(相鄰
//! 規則交換律成立:修剪只縮短端點,端點只取決於他人 start,而 start 不變);
//! `NaiveMenu`(任意 cut 的縮短 / 分裂 / 交換 / 運行期標記)則被機械找出
//! 不可回合的臨界對 —— 正是報告 §4.3 說的「同一錯誤兩次修復兩種結果」。

use crate::ast::Interval;

// ===========================================================================
// 狀態:區間配置(事實層)
// ===========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum K {
    /// 可變訪問(write / &mut 引用本體)
    Mut,
    /// 共享訪問(read / & 引用本體)
    Sh,
}

impl K {
    pub fn label(self) -> &'static str {
        match self {
            K::Mut => "mut",
            K::Sh => "sh",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ev {
    pub id: u32,
    pub storage: u32,
    pub kind: K,
    pub it: Interval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AState {
    pub evs: Vec<Ev>,
    /// 已標記為運行期借用的邊(從 E_red 移出;事實層記 runtime-borrow 標記)。
    pub runtime: Vec<(u32, u32)>,
    /// 操作日誌(供演示;不參與相等性)。
    pub log: Vec<String>,
}

impl AState {
    pub fn new(evs: Vec<Ev>) -> AState {
        // 自動補 id
        let mut s = AState {
            evs,
            runtime: Vec::new(),
            log: Vec::new(),
        };
        for (i, e) in s.evs.iter_mut().enumerate() {
            if e.id == u32::MAX {
                e.id = i as u32;
            }
        }
        s
    }

    /// §3.3 相容性違反:mut ↔ mut 與 mut ↔ sh(兩個 mut 中的「本體」互斥)。
    pub fn pair_conflicts(a: K, b: K) -> bool {
        a == K::Mut || b == K::Mut
    }

    /// 紅邊集合(相容性違反 ∧ 區間相交 ∧ 未標記 runtime)。
    pub fn red_edges(&self) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for i in 0..self.evs.len() {
            for j in (i + 1)..self.evs.len() {
                let (a, b) = (&self.evs[i], &self.evs[j]);
                if a.storage != b.storage {
                    continue;
                }
                if !AState::pair_conflicts(a.kind, b.kind) {
                    continue;
                }
                if !a.it.overlaps(&b.it) {
                    continue;
                }
                let (x, y) = (a.id.min(b.id), a.id.max(b.id));
                if self.runtime.contains(&(x, y)) {
                    continue;
                }
                out.push((x, y));
            }
        }
        out
    }

    /// μ = (|E_red|, |Err_rustc|):本演示 Err_rustc := 0(oracle 範疇)。
    pub fn measure(&self) -> (usize, usize) {
        (self.red_edges().len(), 0)
    }

    /// 字典序嚴格遞減(§4.2 良基測度)。
    pub fn strictly_decreases(a: (usize, usize), b: (usize, usize)) -> bool {
        a.0 < b.0 || (a.0 == b.0 && a.1 < b.1)
    }

    pub fn is_normal_form(&self) -> bool {
        self.red_edges().is_empty()
    }
}

// ===========================================================================
// 菜單規則
// ===========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    R1Shorten(u32, u32), // (事件 id, 新右端點)
    R2Split(u32, u32),   // (事件 id, 切點)
    R3Swap(u32, u32),    // (事件 a, 事件 b)—— 交換區間(重排語句)
    R4Runtime(u32, u32), // (事件 a, 事件 b)—— 標記為運行期借用
}

impl Rule {
    pub fn label(&self) -> String {
        match self {
            Rule::R1Shorten(i, c) => format!("R1 shorten[{} → {}]", i, c),
            Rule::R2Split(i, m) => format!("R2 split[{} @ {}]", i, m),
            Rule::R3Swap(a, b) => format!("R3 swap({},{})", a, b),
            Rule::R4Runtime(a, b) => format!("R4 runtime({},{})", a, b),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// guard 通過 ⟺ μ 嚴格遞減(帶側條件的菜單 = 報告的封閉菜單紀律)。
    Guarded,
    /// 無側條件(機械展示:為什麼側條件是定律的必要載體)。
    Raw,
}

/// 菜單 M = 規範修剪家族(R1,自由事件;cut 規範化為「最早衝突事件起點」)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    /// 規範修剪菜單:只含 R1 且 cut 規範化;可被機械證明局部合流。
    CommutativeTrim,
    /// 樸素菜單:R1(任意 cut)、R2、R3、R4 —— 機械找出 L9 反例。
    Naive,
}

impl Menu {
    pub fn label(&self) -> &'static str {
        match self {
            Menu::CommutativeTrim => "CommutativeTrim(規範修剪)",
            Menu::Naive => "Naive(樸素:任意 cut / split / swap / runtime)",
        }
    }

    /// 枚舉狀態 s 上的所有適用規則(策略無關:全部單步)。
    pub fn applicable(&self, s: &AState, policy: Policy) -> Vec<Rule> {
        let mut out = Vec::new();
        let reds: Vec<(u32, u32)> = s.red_edges();
        match self {
            Menu::CommutativeTrim => {
                // 對每個事件:若存在「起點更晚」的衝突事件,規範 cut = 最早者起點。
                for a in &s.evs {
                    let mut cut: Option<u32> = None;
                    for j in 0..s.evs.len() {
                        let b = &s.evs[j];
                        if b.storage != a.storage || b.id == a.id {
                            continue;
                        }
                        if !AState::pair_conflicts(a.kind, b.kind) {
                            continue;
                        }
                        // 只修剪「起點更晚」的衝突(修剪較早事件才有效可能)
                        if b.it.start <= a.it.start {
                            continue;
                        }
                        if a.it.overlaps(&b.it) {
                            cut = Some(match cut {
                                Some(c) => c.min(b.it.start),
                                None => b.it.start,
                            });
                        }
                    }
                    if let Some(c) = cut {
                        out.push(Rule::R1Shorten(a.id, c));
                    }
                }
            }
            Menu::Naive => {
                // R1:任意 cut(嚴格在區間內,且 cut 後 μ 遞減(Guarded)或任意(Raw))
                for a in &s.evs {
                    for cut in (a.it.start + 1)..a.it.end {
                        out.push(Rule::R1Shorten(a.id, cut));
                    }
                }
                // R2:split —— 在切點 m 把 storage 的「之後」活動遷到新 storage(複製語義)
                for a in &s.evs {
                    for m in (a.it.start + 1)..a.it.end {
                        out.push(Rule::R2Split(a.id, m));
                    }
                }
                // R3:交換兩個區間(重排) —— 對所有對
                for i in 0..s.evs.len() {
                    for j in (i + 1)..s.evs.len() {
                        out.push(Rule::R3Swap(s.evs[i].id, s.evs[j].id));
                    }
                }
                // R4:標記紅邊為運行期借用
                for &(x, y) in &reds {
                    out.push(Rule::R4Runtime(x, y));
                }
            }
        }
        // Guarded 政策:只保留 μ 嚴格遞減的施用。
        if policy == Policy::Guarded {
            let m0 = s.measure();
            out.retain(|r| {
                if let Some(s2) = apply(s, *r) {
                    AState::strictly_decreases(s2.measure(), m0)
                } else {
                    false
                }
            });
        }
        out
    }
}

/// 單步重寫:返回 None 表示規則不適用。
pub fn apply(s: &AState, r: Rule) -> Option<AState> {
    let mut s2 = s.clone();
    match r {
        Rule::R1Shorten(id, cut) => {
            let ev = s2.evs.iter_mut().find(|e| e.id == id)?;
            if cut <= ev.it.start || cut >= ev.it.end {
                return None;
            }
            ev.it.end = cut;
            // 端點已縮短 ⇒ 交集只減不增(見 §4.2 討論)
            s2.log.push(r.label());
            Some(s2)
        }
        Rule::R2Split(id, m) => {
            let ev = s2.evs.iter_mut().find(|e| e.id == id)?;
            if m <= ev.it.start || m >= ev.it.end {
                return None;
            }
            let old_storage = ev.storage;
            let new_storage = s2.evs.iter().map(|e| e.storage).max().unwrap_or(0) + 1;
            // 複製語義:切點之後(m 之後)的活動遷往新 storage(獨立分配)。
            for e in s2.evs.iter_mut() {
                if e.storage == old_storage && e.it.start >= m {
                    e.storage = new_storage;
                }
            }
            s2.log.push(format!(
                "{} (storage {} ↗ {})",
                r.label(),
                old_storage,
                new_storage
            ));
            Some(s2)
        }
        Rule::R3Swap(a, b) => {
            let ia = s2.evs.iter().position(|e| e.id == a)?;
            let ib = s2.evs.iter().position(|e| e.id == b)?;
            if ia == ib {
                return None;
            }
            let (va, vb) = (s2.evs[ia].it, s2.evs[ib].it);
            s2.evs[ia].it = vb;
            s2.evs[ib].it = va;
            s2.log.push(r.label());
            Some(s2)
        }
        Rule::R4Runtime(x, y) => {
            let (x, y) = (x.min(y), x.max(y));
            if !s.red_edges().contains(&(x, y)) {
                return None;
            }
            s2.runtime.push((x, y));
            s2.log.push(r.label());
            Some(s2)
        }
    }
}

/// 一個正規化步驟(策略:取第一個適用規則;回傳 None ⇒ 已是正規形)。
pub fn step(s: &AState, menu: Menu, policy: Policy) -> Option<(AState, Rule)> {
    let rules = menu.applicable(s, policy);
    let r = *rules.first()?;
    let s2 = apply(s, r)?;
    Some((s2, r))
}

/// 反覆規約(帶步數上限;終止由 L8 測度保證)。
pub fn normalize(mut s: AState, menu: Menu, policy: Policy) -> (AState, usize) {
    let mut steps = 0usize;
    loop {
        if steps > 10000 {
            break; // 防禦性上限(理論上由良基測度保證不可達)
        }
        match step(&s, menu, policy) {
            Some((s2, _)) => {
                s = s2;
                steps += 1;
            }
            None => break,
        }
    }
    (s, steps)
}

// ===========================================================================
// L8 / L9 的機械檢查
// ===========================================================================

/// L8:隨機/窮舉檢查 —— 菜單每步必須嚴格遞減 μ。
/// 返回違反 L8 的第一個反例(若有)。
pub fn l8_check(s: &AState, menu: Menu, policy: Policy) -> Option<(AState, AState, Rule)> {
    for r in menu.applicable(s, policy) {
        if let Some(s2) = apply(s, r) {
            if policy == Policy::Guarded {
                // Guarded 由 applicable 保證;這裡獨立複驗。
                if !AState::strictly_decreases(s2.measure(), s.measure()) {
                    return Some((s.clone(), s2, r));
                }
            } else if !AState::strictly_decreases(s2.measure(), s.measure()) {
                // Raw:非嚴格遞減即違反
                return Some((s.clone(), s2, r));
            }
        }
    }
    None
}

/// 小狀態窮舉宇宙:1 個 storage、n 個事件、坐標 0..=max。
pub fn enumerate_states(n_events: usize, max_coord: u32) -> Vec<AState> {
    // 每個事件:(kind ∈ {Mut, Sh}) × (start < end ≤ max_coord)
    use std::collections::BTreeSet;
    let mut per_event: Vec<Vec<Ev>> = Vec::new();
    for _ in 0..n_events {
        let mut evs = Vec::new();
        for kind in [K::Mut, K::Sh] {
            for start in 0..max_coord {
                for end in (start + 1)..=max_coord {
                    evs.push(Ev {
                        id: u32::MAX,
                        storage: 0,
                        kind,
                        it: Interval { start, end },
                    });
                }
            }
        }
        per_event.push(evs);
    }
    let mut out = BTreeSet::new();
    let mut acc: Vec<Ev> = Vec::new();
    fn rec(
        per: &[Vec<Ev>],
        depth: usize,
        acc: &mut Vec<Ev>,
        out: &mut BTreeSet<Vec<(K, Interval)>>,
    ) {
        if depth == per.len() {
            out.insert(acc.iter().map(|e| (e.kind, e.it)).collect());
            return;
        }
        for e in &per[depth] {
            acc.push(*e);
            rec(per, depth + 1, acc, out);
            acc.pop();
        }
    }
    rec(&per_event, 0, &mut acc, &mut out);
    out.iter()
        .map(|combo| {
            AState::new(
                combo
                    .iter()
                    .enumerate()
                    .map(|(i, &(kind, it))| Ev {
                        id: i as u32,
                        storage: 0,
                        kind,
                        it,
                    })
                    .collect(),
            )
        })
        .collect()
}
