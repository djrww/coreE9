//! §4.3 機械的 Newman 通道:
//!   **SN ∧ WCR ⇒ CR ⇒ 唯一正規形**(Newman 引理,1942)。
//!
//! 這裡不是「引一句定理」,而是把前提逐一**機械驗證**:
//!   (T8) 終止性(SN):μ = (|E_red|, |Err_rustc|) 良基 + 每步嚴格遞減
//!        ⇒ 重寫序列不可能無限;
//!   (T9) 局部合流(WCR):對每個狀態與每對「臨界步」(s→a, s→b),
//!        機械搜索共同後繼(可回合性);
//!   (T10) 推論:唯一正規形 —— 對每個狀態,窮舉所有極大歸約序列,
//!        終點必為同一個狀態(唯一正規形)。
//!
//! 對 `CommutativeTrim` 菜單,三項全部機械通過;對 `Naive` 菜單,
//! 機械檢查**如實報告**其不可回合臨界對與多正規形(這正是報告 §4.3 的
//! 「同一錯誤、兩次修復、兩種結果」)。

use crate::rep::{apply, enumerate_states, l8_check, AState, Menu, Policy, Rule, K};
use std::collections::{BTreeSet, HashMap};

/// Newman 通道的機械報告(§4.2–4.3 的驗證輸出;所有字段皆可機械復算)。
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
    /// 並行分塊所使用的線程數(available_parallelism,上限 8)。
    pub threads: usize,
    /// 反例列表是否為報告目的截斷(計數字段 critical_pairs / states 永遠全量)。
    pub truncated: bool,
}

fn canon_key(s: &AState) -> StateKey {
    let mut v: Vec<(u32, u32, K, u32, u32)> = s
        .evs
        .iter()
        .map(|e| (e.id, e.storage, e.kind, e.it.start, e.it.end))
        .collect();
    v.sort();
    v
}

/// 狀態的規範鍵(重寫系統的狀態空間元素)。
pub type StateKey = Vec<(u32, u32, K, u32, u32)>;

/// 從 s 出發的 bf(深度 ≤ depth)狀態閉包。
fn closure(s: &AState, menu: Menu, policy: Policy, depth: usize) -> HashMap<StateKey, AState> {
    let mut seen: HashMap<_, AState> = HashMap::new();
    seen.insert(canon_key(s), s.clone());
    let mut frontier = vec![s.clone()];
    for _ in 0..depth {
        if frontier.is_empty() {
            break;
        }
        let mut next = Vec::new();
        for st in frontier {
            for r in menu.applicable(&st, policy) {
                if let Some(s2) = apply(&st, r) {
                    let k = canon_key(&s2);
                    if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(k) {
                        e.insert(s2.clone());
                        next.push(s2);
                    }
                }
            }
        }
        frontier = next;
    }
    seen
}

/// 可回合性:兩個狀態的閉包有交集(深度 depth)。
pub fn joinable(a: &AState, b: &AState, menu: Menu, policy: Policy, depth: usize) -> bool {
    let ca = closure(a, menu, policy, depth);
    let cb = closure(b, menu, policy, depth);
    ca.keys().any(|k| cb.contains_key(k))
}

/// 從 s 出發收集所有正規形(極大歸約序列的終點)。
pub fn normal_forms(s: &AState, menu: Menu, policy: Policy, depth: usize) -> Vec<StateKey> {
    let mut nfs: BTreeSet<StateKey> = BTreeSet::new();
    let mut stack = vec![(s.clone(), 0usize)];
    while let Some((st, d)) = stack.pop() {
        if d >= depth {
            nfs.insert(canon_key(&st));
            continue;
        }
        let rules = menu.applicable(&st, policy);
        if rules.is_empty() {
            nfs.insert(canon_key(&st));
            continue;
        }
        for r in rules {
            if let Some(s2) = apply(&st, r) {
                stack.push((s2, d + 1));
            }
        }
    }
    nfs.into_iter().collect()
}

/// 單一狀態的 Newman 檢查(純函數;供並行分塊調用,無共享可變狀態)。
struct ChunkResult {
    l8: Vec<(AState, AState, Rule)>,
    critical_pairs: usize,
    non_joinable: Vec<(AState, Rule, Rule, AState, AState)>,
    unique_ok: usize,
    multi_nf: Vec<(AState, Vec<AState>)>,
}

fn check_state(s: &AState, menu: Menu, policy: Policy, depth: usize) -> ChunkResult {
    let mut l8 = Vec::new();
    let mut critical_pairs = 0usize;
    let mut non_joinable = Vec::new();
    let mut unique_ok = 0usize;
    let mut multi_nf = Vec::new();
    if let Some(v) = l8_check(s, menu, policy) {
        l8.push(v);
    }
    let rules = menu.applicable(s, policy);
    let mut x = 0usize;
    while x < rules.len() {
        let mut y = x + 1;
        while y < rules.len() {
            critical_pairs += 1;
            if let (Some(a), Some(b)) = (apply(s, rules[x]), apply(s, rules[y])) {
                if !joinable(&a, &b, menu, policy, depth) {
                    non_joinable.push((s.clone(), rules[x], rules[y], a, b));
                }
            }
            y += 1;
        }
        x += 1;
    }
    let nfs = normal_forms(s, menu, policy, depth);
    if nfs.len() == 1 {
        unique_ok += 1;
    } else {
        multi_nf.push((
            s.clone(),
            nfs.iter()
                .map(|k| {
                    // 重新構造狀態僅用於報告
                    let evs = k
                        .iter()
                        .map(|&(id, storage, kind, st, en)| crate::rep::Ev {
                            id,
                            storage,
                            kind,
                            it: crate::ast::Interval { start: st, end: en },
                        })
                        .collect();
                    AState::new(evs)
                })
                .collect(),
        ));
    }
    ChunkResult {
        l8,
        critical_pairs,
        non_joinable,
        unique_ok,
        multi_nf,
    }
}

/// 機械 Newman 檢查:對菜單 × 政策做窮舉狀態空間上的
/// L8(測度遞減)+ 臨界對可合流(§4.3)雙重驗證,輸出報告。
/// 前提(報告 §4.2):μ 良基 ⇒ SN;SN ∧ WCR ⇒ CR ⇒ 唯一正規形。
///
/// **並行分塊**(P0 #4):狀態間檢查互不依賴,以 `std::thread::scope`
/// 分塊並行(零依賴;線程數 = available_parallelism,上限 8)。
/// 計數字段(critical_pairs / unique_nf_states / states)永遠全量;
/// 反例列表為報告可讀性截斷(每類上限 64),`truncated` 如實標記。
pub fn newman_check(
    menu: Menu,
    policy: Policy,
    n_events: usize,
    max_coord: u32,
    depth: usize,
) -> NewmanReport {
    let all = enumerate_states(n_events, max_coord);
    // 排除重複 start(演示域:start 嚴格遞增 —— 詳見 rep.rs 的註記)
    let states: Vec<AState> = all
        .into_iter()
        .filter(|s| {
            let mut starts: Vec<u32> = s.evs.iter().map(|e| e.it.start).collect();
            starts.sort();
            starts.windows(2).all(|w| w[0] != w[1])
        })
        .collect();

    let n = states.len();
    let threads = std::thread::available_parallelism()
        .map(|t| t.get())
        .unwrap_or(1)
        .clamp(1, 8)
        .min(n.max(1));
    let chunk = n.div_ceil(threads).max(1);

    let mut results: Vec<ChunkResult> = Vec::with_capacity(threads);
    std::thread::scope(|sc| {
        let handles: Vec<_> = states
            .chunks(chunk)
            .map(|part| {
                sc.spawn(move || {
                    part.iter()
                        .map(|s| check_state(s, menu, policy, depth))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            for r in h.join().expect("newman worker must not panic") {
                results.push(r);
            }
        }
    });

    let mut l8_violations = Vec::new();
    let mut critical_pairs = 0usize;
    let mut non_joinable = Vec::new();
    let mut unique_ok = 0usize;
    let mut multi_nf = Vec::new();
    let mut truncated = false;
    for r in results {
        critical_pairs += r.critical_pairs;
        unique_ok += r.unique_ok;
        for v in r.l8 {
            if l8_violations.len() < 64 {
                l8_violations.push(v);
            } else {
                truncated = true;
            }
        }
        for v in r.non_joinable {
            if non_joinable.len() < 64 {
                non_joinable.push(v);
            } else {
                truncated = true;
            }
        }
        for v in r.multi_nf {
            if multi_nf.len() < 64 {
                multi_nf.push(v);
            } else {
                truncated = true;
            }
        }
    }

    let conclusion = if l8_violations.is_empty() && non_joinable.is_empty() && multi_nf.is_empty() {
        "SN ∧ WCR ⇒ CR ⇒ 唯一正規形(機械驗證通過)"
    } else if !l8_violations.is_empty() {
        "L8 違反:存在不嚴格遞減 μ 的施用(側條件是定律的載體)"
    } else if !non_joinable.is_empty() {
        "WCR 違反:存在不可回合臨界對 ⇒ 不滿足 Newman 前提 ⇒ 正規形不唯一"
    } else {
        "多正規形:同一狀態存在不同極大歸約終點"
    };

    NewmanReport {
        menu,
        policy,
        states: states.len(),
        l8_violations,
        critical_pairs,
        non_joinable,
        unique_nf_states: unique_ok,
        multi_nf,
        conclusion,
        threads,
        truncated,
    }
}
