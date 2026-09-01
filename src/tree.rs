//! 樹的工具:樹操作放在 `Tree` 的方法與此處的輔助函數。
//! (無損回環、具名投影、CW 複形的其它檢驗在這裡。)

use crate::parse::{Kind, Tree};

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
