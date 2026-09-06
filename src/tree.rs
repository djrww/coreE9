//! 樹的工具:樹操作放在 `Tree` 的方法與此處的輔助函數。
//! (無損回環、具名投影、CW 複形的其它檢驗在這裡。)
//!
//! **雙載體共用檢驗面**:`NodeView` + `check_*` 是 CL0 `Tree` 與 R₀ `R0Tree`
//! 的同一套樹性質檢查(連續性公理 / 樹公理 / L5 laminar / 無損回環)的單一
//! 實作 —— 同一条公理只有一份實現,消掉「雙語言兩份定義」的鏡像漂移面
//! (HARD-ITEMS #2 在樹檢驗層的縮減)。

use crate::parse::{Kind, Tree};
use crate::r0::R0Tree;
use crate::span::Span;

/// 節點表視角:兩載體樹(CST 節點 + 半開 span + 子節點表)的共用介面。
pub trait NodeView {
    /// 節點總數(id = 下標)。
    fn node_count(&self) -> usize;
    /// 節點 span。
    fn span_of(&self, id: u32) -> Span;
    /// 節點的直接子節點(依源碼順序)。
    fn children_of(&self, id: u32) -> &[u32];
}

impl NodeView for Tree {
    fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn span_of(&self, id: u32) -> Span {
        self.nodes[id as usize].span
    }
    fn children_of(&self, id: u32) -> &[u32] {
        &self.nodes[id as usize].children
    }
}

impl NodeView for R0Tree {
    fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn span_of(&self, id: u32) -> Span {
        self.nodes[id as usize].span
    }
    fn children_of(&self, id: u32) -> &[u32] {
        &self.nodes[id as usize].children
    }
}

/// §1.2 連續性公理:內部節點 σ(v) = [σ(c₁).start, σ(c_k).end),
/// 且子節點依序不交:∀i: σ(cᵢ).end ≤ σ(cᵢ₊₁).start。
pub fn check_continuity(v: &dyn NodeView) -> Result<(), String> {
    let n = v.node_count();
    for id in 0..n as u32 {
        let children = v.children_of(id);
        if children.is_empty() {
            continue;
        }
        let first = v.span_of(children[0]);
        let last = v.span_of(*children.last().unwrap());
        if v.span_of(id) != Span::new(first.start, last.end) {
            return Err(format!(
                "node {} span {} != children union [{}, {})",
                id,
                v.span_of(id),
                first.start,
                last.end
            ));
        }
        let mut prev_end = first.start;
        for &c in children {
            let cs = v.span_of(c);
            if cs.start < prev_end {
                return Err(format!(
                    "node {} children overlap: child {} span {} before prev_end {}",
                    id, c, cs, prev_end
                ));
            }
            prev_end = cs.end;
        }
    }
    Ok(())
}

/// 樹公理:每節點至多一父、自根連通(E ⊆ V×V 連通、無環、每節點至多一父)。
pub fn check_tree_axioms(v: &dyn NodeView) -> Result<(), String> {
    let n = v.node_count();
    let mut parent_of = vec![u32::MAX; n];
    for id in 0..n as u32 {
        for &c in v.children_of(id) {
            if parent_of[c as usize] != u32::MAX {
                return Err(format!("node {} has two parents", c));
            }
            parent_of[c as usize] = id;
        }
    }
    // 連通性:從根(0)出發 DFS 必須訪問全部節點。
    let mut seen = vec![false; n];
    let mut stack = vec![0u32];
    while let Some(id) = stack.pop() {
        if seen[id as usize] {
            continue;
        }
        seen[id as usize] = true;
        for &c in v.children_of(id) {
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

/// L5 檢查:任意兩節點 span 要嘛嵌套、要嘛不交(laminar 族;§3.1 定理)。
pub fn check_laminar(v: &dyn NodeView) -> bool {
    let n = v.node_count();
    for i in 0..n as u32 {
        let a = v.span_of(i);
        for j in (i + 1)..n as u32 {
            let b = v.span_of(j);
            if a.overlaps(&b) && !a.contains(&b) && !b.contains(&a) {
                return false;
            }
        }
    }
    true
}

/// 無損回環(L1):葉子文本依源碼順序(= id 前序)拼接 ≡ 源碼,逐字節。
pub fn unparse_all(v: &dyn NodeView, src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for id in 0..v.node_count() as u32 {
        if v.children_of(id).is_empty() {
            let sp = v.span_of(id);
            out.push_str(&src[sp.start as usize..sp.end as usize]);
        }
    }
    out
}

impl Tree {
    /// §1.3 具名節點樹的節點列表(具名投影的節點集)。
    pub fn named_node_ids(&self) -> Vec<u32> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind.is_named())
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// 統計:具名節點 / 匿名節點 / 錯誤節點 / trivia 數。
    pub fn stats(&self) -> (usize, usize, usize, usize) {
        let mut named = 0;
        let mut anon = 0;
        let mut err = 0;
        let mut trivia = 0;
        for n in &self.nodes {
            match n.kind {
                Kind::Error => err += 1,
                Kind::Trivia => trivia += 1,
                k if k.is_named() => named += 1,
                _ => anon += 1,
            }
        }
        (named, anon, err, trivia)
    }
}

/// L7b 結構極大化的機械化檢查(§2.3):
/// 反覆移除「最大錯誤跨度」直到不動點;殘餘只允許位於切縫 / EOF 的空錯誤
/// (缺失內容的如實記號)。返回 `(bad, rounds)`:
/// `bad` = 違反結構極大性的錯誤節點數;`rounds` = 淨化輪數(≥8 視為不終止)。
/// 供 fuzz 縮小謂詞與回歸重播複用;測試矩陣中的原始斷言見 `tests/laws.rs`。
pub fn l7b_evaluate(src: &str) -> (usize, usize) {
    let mut cur = src.to_string();
    let mut seams: Vec<u32> = Vec::new();
    let mut rounds = 0usize;
    while rounds < 8 {
        let rec = crate::parse::parse(&cur).unwrap();
        let spans = rec.maximal_error_spans();
        if spans.is_empty() {
            break;
        }
        let mut cut = String::new();
        let mut last = 0u32;
        for sp in &spans {
            cut.push_str(&cur[last as usize..sp.start as usize]);
            seams.push(sp.start);
            seams.push(sp.end);
            last = sp.end;
        }
        cut.push_str(&cur[last as usize..]);
        if cut == cur {
            break; // 只剩空跨度:無進展
        }
        cur = cut;
        rounds += 1;
    }
    let fin = crate::parse::parse(&cur).unwrap();
    let mut bad = 0usize;
    for n in &fin.nodes {
        if n.kind != Kind::Error {
            continue;
        }
        if !n.span.is_empty()
            || !(n.span.start as usize == cur.len() || seams.contains(&n.span.start))
        {
            bad += 1;
        }
    }
    (bad, rounds)
}
