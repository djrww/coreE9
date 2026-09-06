//! 樹的工具:樹操作放在 `Tree` 的方法與此處的輔助函數。
//! (無損回環、具名投影、CW 複形的其它檢驗在這裡。)
//!
//! 共享幾何特質 `TreeGeometry`:CL0 `Tree` 與 R₀ `R0Tree` 的「純 span/子節點
//! 幾何」性質檢查原先是兩份逐字元一致的方法(連續性公理、樹公理、L5 laminar、
//! L7a 錯誤面度量、規模)。此處抽成**單一**特質 default 方法,兩個載體只各提供
//! 極小的存取器 —— 邏輯一份,兩樹共用。

use crate::parse::{Kind, Tree};
use crate::span::Span;

// ===========================================================================
// 共享幾何特質(§1.2 / §2.3 / §3.1)
// ===========================================================================

/// 表面語法樹的幾何/律檢查特質(dedup 載體):CL0 `Tree` 與 R₀ `R0Tree` 共用。
///
/// 每個實作只需要提供 4 個「原子」存取器:
///
/// * `nodes_len`     —— 節點總數;
/// * `node_span`     —— 依 id 取半開跨度;
/// * `node_children` —— 依 id 取直接子節點表;
/// * `is_error`      —— 節點是否為 ERROR 類。
///
/// 具體的定律檢查(連續性公理、樹公理、L5 laminar 嵌套、L7a 錯誤面度量、
/// 節點規模)由本特質的 default 方法**一次**定義,兩樹共用同一份語義。
pub trait TreeGeometry {
    /// 節點總數(樹規模)。
    fn nodes_len(&self) -> usize;

    /// 依 id 取節點的半開跨度 σ(v) = [start, end)。
    fn node_span(&self, id: u32) -> Span;

    /// 依 id 取直接子節點表(依源碼順序)。
    fn node_children(&self, id: u32) -> &[u32];

    /// 節點是否為錯誤類(§2.3 全化的錯誤面;L7a 判據的原子)。
    fn is_error(&self, id: u32) -> bool;

    /// §1.2 連續性公理:內部節點 σ(v) = [σ(c₁).start, σ(c_k).end),
    /// 且子節點依序不交:∀i: σ(cᵢ).end ≤ σ(cᵢ₊₁).start。
    fn validate_continuity(&self) -> Result<(), String> {
        for id in 0..self.nodes_len() {
            let children = self.node_children(id as u32);
            if children.is_empty() {
                continue;
            }
            let first = self.node_span(children[0]);
            let last = self.node_span(*children.last().unwrap());
            let span = self.node_span(id as u32);
            if span != Span::new(first.start, last.end) {
                return Err(format!(
                    "node {} span {} != children union [{}, {})",
                    id, span, first.start, last.end
                ));
            }
            let mut prev_end = first.start;
            for &c in children {
                let cs = self.node_span(c);
                if cs.start < prev_end {
                    return Err(format!(
                        "node {} children overlap: child {} span {} < prev_end {}",
                        id, c, cs, prev_end
                    ));
                }
                prev_end = cs.end;
            }
        }
        Ok(())
    }

    /// 樹公理:E ⊆ V×V 連通、無環、每節點至多一父。
    fn validate_tree_shapes(&self) -> Result<(), String> {
        let n = self.nodes_len();
        let mut parent_of = vec![u32::MAX; n];
        for id in 0..n {
            for &c in self.node_children(id as u32) {
                if parent_of[c as usize] != u32::MAX {
                    return Err(format!("node {} has two parents", c));
                }
                parent_of[c as usize] = id as u32;
            }
        }
        // 連通性:從根(恆為 0)出發 DFS 必須訪問全部節點。
        let mut seen = vec![false; n];
        let mut stack = vec![0u32];
        while let Some(id) = stack.pop() {
            if seen[id as usize] {
                continue;
            }
            seen[id as usize] = true;
            for &c in self.node_children(id) {
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
    fn laminar_ok(&self) -> bool {
        let n = self.nodes_len();
        for i in 0..n {
            let a = self.node_span(i as u32);
            for j in (i + 1)..n {
                let b = self.node_span(j as u32);
                if a.overlaps(&b) && !a.contains(&b) && !b.contains(&a) {
                    return false;
                }
            }
        }
        true
    }

    /// ERROR 節點數(§2.3 全化的「錯誤面」度量)。
    fn n_errors(&self) -> usize {
        (0..self.nodes_len())
            .filter(|&id| self.is_error(id as u32))
            .count()
    }

    /// L7a:是否存在 ERROR 節點。
    fn has_error(&self) -> bool {
        self.n_errors() > 0
    }

    /// 節點總數(具名 + 匿名 + trivia + error;樹的規模度量)。
    fn total_nodes(&self) -> usize {
        self.nodes_len()
    }
}

// ===========================================================================
// CL0 `Tree` 的實作與 API 形影(委派到共享特質,呼叫點與對外介面不變)
// ===========================================================================

impl TreeGeometry for Tree {
    fn nodes_len(&self) -> usize {
        self.nodes.len()
    }
    fn node_span(&self, id: u32) -> Span {
        self.nodes[id as usize].span
    }
    fn node_children(&self, id: u32) -> &[u32] {
        &self.nodes[id as usize].children
    }
    fn is_error(&self, id: u32) -> bool {
        self.nodes[id as usize].kind == Kind::Error
    }
}

impl Tree {
    /// §1.2 連續性公理(共享幾何特質)。
    pub fn validate_continuity(&self) -> Result<(), String> {
        TreeGeometry::validate_continuity(self)
    }

    /// 樹公理:E ⊆ V×V 連通、無環、每節點至多一父(共享幾何特質)。
    pub fn validate_tree_shapes(&self) -> Result<(), String> {
        TreeGeometry::validate_tree_shapes(self)
    }

    /// L5 檢查(共享幾何特質)。
    pub fn laminar_ok(&self) -> bool {
        TreeGeometry::laminar_ok(self)
    }

    /// L7a:樹中是否有 ERROR 節點(共享幾何特質)。
    pub fn n_errors(&self) -> usize {
        TreeGeometry::n_errors(self)
    }

    /// L7a 斷言用:樹中是否存在 ERROR 節點(共享幾何特質)。
    pub fn has_error(&self) -> bool {
        TreeGeometry::has_error(self)
    }

    /// 節點總數(共享幾何特質)。
    pub fn total_nodes(&self) -> usize {
        TreeGeometry::total_nodes(self)
    }

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
