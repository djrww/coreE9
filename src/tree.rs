//! 樹的工具:樹操作放在 `Tree` 的方法與此處的輔助函數。
//! (無損回環、具名投影、CW 複形的其它檢驗在這裡。)

use crate::parse::{Kind, ParseIssue, Tree};

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
/// (缺失內容的如實記號)。返回 `Ok((bad, rounds))`:
/// `bad` = 違反結構極大性的錯誤節點數;`rounds` = 淨化輪數(≥8 視為不終止)。
/// 供 fuzz 縮小謂詞與回歸重播複用;測試矩陣中的原始斷言見 `tests/laws.rs`。
///
/// ## 為什麼回傳 `Result` 而不是直接 unwrap(健檢 P0-2)
/// `parse` 在遞迴深度 ≥ `RECURSION_LIMIT` 時回 `Err(ParseIssue::Depth)` ——
/// 這是**刻意**的「機器界如實申報」。舊版在這裡寫 `parse(..).unwrap()`,
/// 於是任何 ≥ 門檻的深嵌套輸入(實測:`fn f() {{` × 84)都會讓本函式
/// **panic**,連帶把 `fuzz` 行程整支 abort(而不是優雅回報 `LAW FAIL`)。
/// 這與 §2.3「全化、永不 panic」直接矛盾:機器界要被**傳出去**,不能被吞掉。
/// 回歸防線見 `tests/laws.rs::test_l7b_depth_reported_not_panicked`。
pub fn l7b_evaluate(src: &str) -> Result<(usize, usize), ParseIssue> {
    let mut cur = src.to_string();
    let mut seams: Vec<u32> = Vec::new();
    let mut rounds = 0usize;
    while rounds < 8 {
        let rec = crate::parse::parse(&cur)?;
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
    let fin = crate::parse::parse(&cur)?;
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
    Ok((bad, rounds))
}
