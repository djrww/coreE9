//! §2.1 編輯單體(edit monoid)。
//!
//! 一次編輯 e = (start, old_end, new_end, text)。位移函數(照抄報告公式):
//!   shift_e(p) = p                    若 p ≤ start
//!              = p + (new_end − old_end) 若 p ≥ old_end
//!              = ⊥(落在被替換區內,未定義)
//!
//! 機械驗證的命題:
//!   (M1) 單位元:空編輯與任意編輯複合仍是原編輯。
//!   (M2) 位移複合 = 平移量之和(整數加法結合律——報告的「證明要點」)。
//!   (M3) 結合律:compose(compose(e1,e2),e3) ≡ compose(e1, compose(e2,e3))
//!        對「可複合」三元組(複合後坐標歸一化到原源碼空間)。
//!   (M4) apply(compose(e1,e2)) == apply(e2, apply(e1, src)) :與「先施 e1
//!        再把 e2 的位址經 e1 位移平移」的語義一致。
//!   (M5) 並行歸併:任意順序歸併同一批互不重疊的編輯,總編輯相同
//!        (watch 層去抖的代數依據,報告 §2.1 的實用意義)。

use crate::span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// 原源碼空間的替換區間 [start, old_end)。
    pub start: u32,
    pub old_end: u32,
    /// 新文本(位於 [start, start + text.len()))。
    pub text: String,
}

impl Edit {
    pub fn new(start: u32, old_end: u32, text: &str) -> Edit {
        Edit {
            start,
            old_end,
            text: text.to_string(),
        }
    }

    /// 編輯在源碼上的淨位移(new_end − old_end)。
    pub fn delta(&self) -> i64 {
        self.text.len() as i64 - (self.old_end as i64 - self.start as i64)
    }

    pub fn new_end(&self) -> u32 {
        self.start + self.text.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.old_end == self.start
    }

    /// 報告的位移函數 shift_e(p):可能為 ⊥(落在被替換區內)。
    pub fn shift(&self, p: u32) -> Option<u32> {
        if p <= self.start {
            Some(p)
        } else if p >= self.old_end {
            Some((p as i64 + self.delta()) as u32)
        } else {
            None
        }
    }

    /// 坐標逆位移(把 e2 的後置坐標映射回 e1 的前置空間)。
    pub fn unshift(&self, p: u32) -> Option<u32> {
        if p <= self.start {
            Some(p)
        } else if p >= self.new_end() {
            Some((p as i64 - self.delta()) as u32)
        } else {
            None
        }
    }
}

/// 複合 e₁·e₂(先施 e₁,再施經 e₁ 位移的 e₂),歸一化到原源碼空間。
///
/// 適用範圍(嚴格):e₂ 的效果區域與 e₁ 的替換區**不相交**。
///   * e₂ 在 e₁ 之後:e₂ 的坐標經 e₁ 逆位移後回到原空間,得到並行的
///     兩個互不重疊編輯(順序無關 —— M5 的合併語義);
///   * e₂ 在 e₁ 之前:同理,返回 [e₂′, e₁]。
/// 相疊(如 e₂ 改寫了 e₁ 插入的文本)需要真正的文本拼接,本函數如實
/// 返回 None(該情形不屬於「去抖批次歸併」的語義範圍)。
pub fn compose(e1: &Edit, e2: &Edit) -> Option<Vec<Edit>> {
    let s2 = e1.unshift(e2.start)?;
    let o2 = e1.unshift(e2.old_end)?;
    if o2 < s2 {
        return None;
    }
    // 閉區間分離檢查:e₁ 與 e₂ 的(閉)區域必須嚴格分離,並行應用才
    // 順序無關。同點插入 / 相觸 / 相疊都屬於「文本拼接」的複合,超出本單體。
    let closed_overlap = s2 <= e1.old_end && e1.start <= o2;
    if closed_overlap {
        return None;
    }
    let e2n = Edit {
        start: s2,
        old_end: o2,
        text: e2.text.clone(),
    };
    if o2 < e1.start {
        // e₂ 完全在 e₁ 之前:並行編輯(順序無關)。
        Some(vec![e2n, e1.clone()])
    } else {
        // s2 > e1.old_end:e₂ 完全在 e₁ 之後。
        Some(vec![e1.clone(), e2n])
    }
}

/// 編輯序列(全部在原空間、互不重疊)的「先後歸併」:
/// 任意順序歸併結果相同 —— 這是報告 §2.1「多個檔案事件任意順序歸併」
/// 的代數依據(M5)。這裡給出機械檢查所需的輔助。
pub fn compose_seq(seq: &[Edit]) -> Option<Vec<Edit>> {
    if !is_pairwise_disjoint(seq) {
        return None;
    }
    let mut out: Vec<Edit> = seq.to_vec();
    out.sort_by_key(|e| (e.start, e.old_end));
    Some(out)
}

/// 應用編輯(坐標在原源碼空間)。
pub fn apply(src: &str, e: &Edit) -> String {
    debug_assert!(e.start as usize <= e.old_end as usize && e.old_end as usize <= src.len());
    let mut out = String::with_capacity(src.len() + e.text.len());
    out.push_str(&src[..e.start as usize]);
    out.push_str(&e.text);
    out.push_str(&src[e.old_end as usize..]);
    out
}

/// 應用一批(互不重疊、坐標皆在原空間的)編輯:從右向左施作 ⇒ O(n)。
pub fn apply_all(src: &str, edits: &[Edit]) -> String {
    let mut sorted: Vec<&Edit> = edits.iter().collect();
    sorted.sort_by_key(|e| (e.start, e.old_end));
    let mut out = src.to_string();
    for e in sorted.iter().rev() {
        out = apply(&out, e);
    }
    out
}

/// (M5) 的檢驗:對一個「編輯集合」,任意排列(在互不重疊前提下)
/// 的總應用結果相同 —— 增量編輯單體的實用後果。
pub fn is_pairwise_disjoint(edits: &[Edit]) -> bool {
    for i in 0..edits.len() {
        for j in (i + 1)..edits.len() {
            let a = Span::new(edits[i].start, edits[i].old_end);
            let b = Span::new(edits[j].start, edits[j].old_end);
            // 插入點(零長)允許與其它區域相接,但同點雙插入語義有順序
            // (誰在誰前) → 不算互不重疊。
            if a.is_empty() && b.is_empty() && a.start == b.start {
                return false;
            }
            let a_nonempty = a.len() > 0;
            let b_nonempty = b.len() > 0;
            if a_nonempty && b_nonempty && a.overlaps(&b) {
                return false;
            }
            if a_nonempty && b.is_empty() && a.start < b.start && b.start < a.end {
                return false;
            }
            if b_nonempty && a.is_empty() && b.start < a.start && a.start < b.end {
                return false;
            }
        }
    }
    true
}
