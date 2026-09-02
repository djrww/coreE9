//! 反例最小化(ddmin,確定性縮小)—— fuzz 失敗時的「把反例縮到最小」。
//!
//! 語境:L1/L2/L5/L7 等律以「輸入 → 布爾(律成立)」為形態受檢(見 `tests/laws.rs`
//! 與 `src/bin/fuzz.rs`)。當屬性失敗,我們要的是**最小反例**:
//! 最短的輸入(字符數),使同一屬性仍失敗。最小反例讓失敗可讀、可歸檔
//! (`tests/fixtures/`)、可防回歸。
//!
//! 算法:字符級 ddmin(Zeller):把當前串分成 n 塊,逐塊嘗試刪除;若刪後仍失敗,
//! 接受刪除並從 n=2 重來;否則 n 倍增。n ≥ len 時塊退化為單字符 ⇒
//! 不動點即「任何單字符刪除都不再保持失敗」—— 這是局部最小,且對
//! 「刪除單調」的屬性(本專案律檢查皆是:刪除不會讓合法程式重新合法化之外
//! 的性質失效)接近全局最小。
//!
//! 約定:**`prop(s) = true` 表示「s 仍是失敗輸入」**(失敗保持謂詞)。
//! 調用方必須保證 `prop(input) = true`(預條件,debug 斷言)。

/// 字符級 ddmin。返回與 `input` 等價失敗(就 `prop` 而言)的最短輸入。
pub fn shrink_to_minimal(input: &str, prop: &dyn Fn(&str) -> bool) -> String {
    let mut cur: Vec<char> = input.chars().collect();
    if cur.is_empty() {
        return String::new();
    }
    debug_assert!(
        prop(input),
        "shrink precondition: input must fail the property"
    );
    let mut n = 2usize;
    loop {
        let len = cur.len();
        let size = len.div_ceil(n).max(1);
        let mut changed = false;
        let mut i = 0usize;
        while i < len {
            let end = (i + size).min(len);
            let mut cand: Vec<char> = Vec::with_capacity(len - (end - i));
            cand.extend_from_slice(&cur[..i]);
            cand.extend_from_slice(&cur[end..]);
            let s: String = cand.iter().collect();
            if prop(&s) {
                cur = cand;
                changed = true;
                n = 2;
                break; // 接受刪除,回到 n=2 重新掃描
            }
            i = end;
        }
        if !changed {
            if size <= 1 {
                // 已逐字符試過所有刪除且無改進:不動點。
                break;
            }
            n *= 2;
        }
    }
    cur.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::shrink_to_minimal;

    #[test]
    fn shrink_finds_exact_minimal() {
        // 合成屬性:包含標記子串「BUGMARK」。干擾腳本無關字符全應被刪光,
        // 最小反例恰為標記本身。
        let prop = |s: &str| s.contains("BUGMARK");
        let input = "xx yy BUGMARK zz /* 註釋 */ tr\x41sh";
        let m = shrink_to_minimal(input, &prop);
        assert_eq!(m, "BUGMARK", "minimal counterexample must be the marker");
        assert!(prop(&m));
    }

    #[test]
    fn shrink_handles_unicode_boundaries() {
        // UTF-8 多字節字符邊界:字符級刪除不得製造非法 UTF-8。
        let prop = |s: &str| s.contains("BUG") || s.contains("イ") || s.chars().count() > 8;
        let input = "開始します BUG 測試 イ 中文 喵喵喵喵喵喵";
        let m = shrink_to_minimal(input, &prop);
        assert!(
            std::str::from_utf8(m.as_bytes()).is_ok(),
            "still valid UTF-8"
        );
        assert!(prop(&m), "failure preserved");
        // 由於「字符數 > 8」也是一條失敗路徑,最小可能比 BUG 更短也合法;
        // 無論哪條路徑,都必須是極小(不存在可再刪的保持失敗字符)。
        let mut removable = false;
        for i in 0..m.chars().count() {
            let mut cand: String = m.chars().take(i).collect();
            cand.extend(m.chars().skip(i + 1));
            if prop(&cand) {
                removable = true;
                break;
            }
        }
        assert!(!removable, "must be a local fixed point: {:?}", m);
    }

    #[test]
    fn shrink_monotone_l7_style() {
        // 模擬 L7/L1 形態的屬性:任意輸入 → parse 樹 + roundtrip 失敗必然保持。
        // 這裡以「包含半開區間記號且長度 ≥ 5」為合成失敗條件,驗證
        // 縮小結果對所有刪除均不再失敗(局部最小)。
        let prop = |s: &str| s.contains("(let") && s.chars().count() >= 5;
        let m = shrink_to_minimal("gg ((let)) hh", &prop);
        assert!(prop(&m));
        assert!(m.len() <= 6, "shrink must drop clutter: {:?}", m);
        for i in 0..m.chars().count() {
            let mut cand: String = m.chars().take(i).collect();
            cand.extend(m.chars().skip(i + 1));
            assert!(!prop(&cand), "removing {} must not keep failure", i);
        }
    }

    #[test]
    fn shrink_empty_and_trivial() {
        assert_eq!(shrink_to_minimal("", &|s: &str| s.is_empty()), "");
        let m = shrink_to_minimal("aaaa", &|s: &str| s.chars().count() >= 4);
        assert_eq!(m, "aaaa");
    }
}
