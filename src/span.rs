//! §1.2 — 位址映象 σ : V → ℤ×ℤ ,σ(v) = [a, b) 為源碼字節區間(半開)。
//! 半開區間是全部定律的幾何基石:連續性公理、L5 嵌套定理、編輯位移函數
//! 都建立在「[a,b)」之上。

/// 半開字節區間 [start, end) 。任何節點 / token 的 span 一律滿足 start <= end。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Span {
        debug_assert!(
            start <= end,
            "span must be half-open [start, end) with start <= end"
        );
        Span { start, end }
    }

    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// 區間包含(允許等式):σ(u) ⊆ σ(v)
    pub fn contains(&self, other: &Span) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// 區間重疊(部分或全部;接觸不算重疊,因為半開)。
    pub fn overlaps(&self, other: &Span) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn shift(&self, delta: i64) -> Span {
        Span {
            start: (self.start as i64 + delta) as u32,
            end: (self.end as i64 + delta) as u32,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {})", self.start, self.end)
    }
}
