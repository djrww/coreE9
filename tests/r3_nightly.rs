//! R3 前測的夜間回歸保護(N5;健檢 §P3-13)。
//!
//! ## 為什麼獨立成檔,而且全部 `#[ignore]`
//!
//! `examples/r3_probe.rs` / `r3_swap.rs` / `r3_wf.rs` 的三個宇宙級證人,過去
//! 只被 `clippy --all-targets` 保證「**能編**」,沒有任何自動執行 ——
//! `docs/EXTERNAL-XCHECK.md` 與 `docs/ROCQ-TRACE.md` 的 R3 前測表全靠人手重跑。
//! 本檔把其中**尚未被 CI 保護**的性質下沉成具名測試。
//!
//! ## 誠實的範圍界定:哪些其實**已經**被保護(不要重複蓋)
//!
//! 盤點後發現,`examples/` 的多數性質早已由 `tests/laws.rs` 守住,本檔**不**重複:
//!
//! | 性質 | 出處 | 現況 |
//! |---|---|---|
//! | P1 start 不變性 | `r3_probe` | ✅ `test_law_L9b_parallel_moves_exact_swap` 已斷言 |
//! | P2 臨界對可回合 | `r3_probe` | ✅ `test_law_L9_scaled_space_joinable`(M5 後跑 4×6 = 623,616 狀態) |
//! | P3 Guarded ≡ Raw | `r3_probe` | ⚠️ 該測試只比 `.len()`,**集合相等**未證 —— 本檔補上 |
//! | 多正規形 = 0 | `r3_probe` | ✅ `test_law_L9_scaled_space_joinable` 已斷言 |
//! | 精確交換 | `r3_swap` | ✅ `test_law_L9b_parallel_moves_exact_swap` 已斷言 |
//!
//! ## 真正沒被保護的:`r3_wf` 第二節的「倒掛宇宙」
//!
//! `rep::enumerate_states` 的生成式是 `for end in (start+1)..=max`(Rocq 側
//! `ends_for` 同形),**由構造排除**倒掛區間 `[start, end)` 且 `end ≤ start`。
//! 因此所有以 `enumerate_states` 為宇宙的測試,對「R3 到底需不需要 `wf_state`
//! 前提」這個問題都是**套套邏輯** —— 這正是 `r3_wf.rs` 第一節自己印出的警告。
//!
//! 決定性的樣本是 `r3_wf` 第二節自行枚舉的**允許倒掛**宇宙。它的結論是反直覺的:
//! **即使允許倒掛,全部 peers 仍精確交換** ⇒ R3 可能不需要 wf 前提。
//! 這個結論撐著 `docs/ROCQ-TRACE.md` 的 R3 路線,卻從未被任何測試執行過。
//! 本檔把它釘住。
//!
//! 為何標 `#[ignore]`:規模屬夜間級(見各測試註解),放進 CI 的 50 具名測試
//! 會拖慢每次 PR。由 `.github/workflows/nightly.yml` 以
//! `cargo test --release -- --ignored` 執行。

use cl0r0::ast::Interval;
use cl0r0::rep::{apply, enumerate_states, AState, Ev, Menu, Policy, K};
use std::collections::BTreeSet;

/// 狀態的規範鍵(排序後無序比較):(id, start, end, kind)。
fn key(s: &AState) -> Vec<(u32, u32, u32, u32)> {
    let mut v: Vec<_> = s
        .evs
        .iter()
        .map(|e| {
            (
                e.id,
                e.it.start,
                e.it.end,
                if e.kind == K::Mut { 1 } else { 0 },
            )
        })
        .collect();
    v.sort();
    v
}

/// 倒掛區間(`start > end`)的個數。
fn inverted(s: &AState) -> usize {
    s.evs.iter().filter(|e| e.it.start > e.it.end).count()
}

/// 自行枚舉「允許倒掛」的小宇宙:每個事件的 (kind, start, end) 三者獨立。
///
/// 這是 `rep::enumerate_states` 做不到的 —— 它構造上就排除倒掛,所以任何以它
/// 為宇宙的測試都無法回答「R3 是否需要 wf 前提」。
fn enumerate_inverted(n: usize, m: u32) -> Vec<AState> {
    let mut per: Vec<Ev> = Vec::new();
    for kind in [K::Mut, K::Sh] {
        for start in 0..=m {
            for end in 0..=m {
                per.push(Ev {
                    id: u32::MAX,
                    storage: 0,
                    kind,
                    it: Interval { start, end },
                });
            }
        }
    }
    let mut out: Vec<Vec<Ev>> = vec![Vec::new()];
    for _ in 0..n {
        let mut next = Vec::with_capacity(out.len() * per.len());
        for pref in &out {
            for e in &per {
                let mut v = pref.clone();
                v.push(*e);
                next.push(v);
            }
        }
        out = next;
    }
    out.into_iter()
        .map(|evs| {
            let mut evs = evs;
            for (i, e) in evs.iter_mut().enumerate() {
                e.id = i as u32;
            }
            AState::new(evs)
        })
        .collect()
}

/// 逐狀態檢查「精確交換」:`s --ra--> a --rb--> c`、`s --rb--> b --ra--> c'`
/// ⇒ `c == c'`(平行步的嚴格交換,而非 merely joinable)。
///
/// 回傳 `(peers, trivial, exact, violations)`;「平凡」= 兩條規則給出同一後繼。
fn scan_exact_swap(states: &[AState]) -> (usize, usize, usize, usize) {
    let mut peers = 0usize;
    let mut trivial = 0usize;
    let mut exact = 0usize;
    let mut violations = 0usize;
    for s in states {
        let g = Menu::CommutativeTrim.applicable(s, Policy::Guarded);
        for x in 0..g.len() {
            for y in (x + 1)..g.len() {
                let (Some(a), Some(b)) = (apply(s, g[x]), apply(s, g[y])) else {
                    continue;
                };
                peers += 1;
                if key(&a) == key(&b) {
                    trivial += 1;
                    continue;
                }
                match (apply(&a, g[y]), apply(&b, g[x])) {
                    (Some(c), Some(c2)) if key(&c) == key(&c2) => exact += 1,
                    _ => violations += 1,
                }
            }
        }
    }
    (peers, trivial, exact, violations)
}

/// `r3_wf` 第二節:允許倒掛的宇宙上,精確交換是否仍全數成立。
///
/// 這是 `docs/ROCQ-TRACE.md` R3 路線「wf 前提可能不必要」的唯一經驗依據,
/// 過去沒有任何自動執行。計數以 `assert_eq!` 釘住:生成器靜默縮水會紅。
#[ignore = "夜間規模(32,768 狀態);cargo test --release -- --ignored"]
#[test]
fn test_r3_wf_inverted_universe_exact_swap() {
    let states = enumerate_inverted(3, 3);
    assert_eq!(
        states.len(),
        32_768,
        "倒掛宇宙規模(2 種 kind × 4 start × 4 end)³"
    );
    assert_eq!(
        states.iter().filter(|s| inverted(s) > 0).count(),
        24_768,
        "含倒掛區間的狀態數 —— 若為 0,本測試就退化成第一節的套套邏輯"
    );

    let (peers, trivial, exact, violations) = scan_exact_swap(&states);
    assert_eq!(
        (peers, trivial, exact),
        (387, 0, 387),
        "倒掛宇宙的 peers / 平凡 / 精確交換計數"
    );
    assert_eq!(
        violations, 0,
        "即使允許倒掛也全數精確交換 —— 這是 R3 無需 wf 前提的經驗依據,一旦出現違反,\
         Rocq 側必須顯式帶 wf_state 不變量(並修訂 ROCQ-TRACE)"
    );
}

/// `r3_wf` 第一節:標準宇宙**不含任何倒掛區間**,故它無法回答 wf 問題。
///
/// 把這個「套套邏輯」的結論本身釘住:未來若有人拿第一節的綠燈當成
/// 「R3 不需要 wf 前提」的證據,這條測試會提醒他第一節根本沒有倒掛樣本。
#[ignore = "夜間規模;cargo test --release -- --ignored"]
#[test]
fn test_r3_wf_standard_universe_has_no_inverted_samples() {
    let states = enumerate_states(3, 3);
    assert_eq!(states.len(), 1_728, "標準宇宙規模(構造上排除倒掛)");
    assert_eq!(
        states.iter().filter(|s| inverted(s) > 0).count(),
        0,
        "標準宇宙構造上不含倒掛 ⇒ 以它為宇宙的結論**不能**用來判斷 \
         R3 是否需要 wf 前提(見 r3_wf 第一節自己的警告)"
    );
    let (peers, _trivial, _exact, violations) = scan_exact_swap(&states);
    assert_eq!(peers, 216, "標準宇宙的 peers 計數");
    assert_eq!(violations, 0, "良構樣本上精確交換成立(已知,現由機器守住)");
}

/// P3 的**集合**相等版:Guarded 與 Raw 在 CT 菜單上給出同一組規則。
///
/// 為什麼要補:`test_law_L9b_parallel_moves_exact_swap` 只比 `.len()`,
/// 而「元素個數相同」不等於「集合相同」。`examples/r3_probe` 比的是集合,
/// 但它從未被自動執行。本測試在含倒掛的宇宙上比集合 —— 那是更嚴的樣本。
#[ignore = "夜間規模;cargo test --release -- --ignored"]
#[test]
fn test_r3_guarded_equals_raw_as_sets_on_inverted_universe() {
    let mut checked = 0usize;
    let mut diverge = Vec::new();
    for s in &enumerate_inverted(3, 3) {
        let g = Menu::CommutativeTrim
            .applicable(s, Policy::Guarded)
            .iter()
            .map(|r| format!("{r:?}"))
            .collect::<BTreeSet<_>>();
        let r = Menu::CommutativeTrim
            .applicable(s, Policy::Raw)
            .iter()
            .map(|r| format!("{r:?}"))
            .collect::<BTreeSet<_>>();
        checked += 1;
        if g != r && diverge.len() < 3 {
            diverge.push(format!(
                "s={:?} guarded={:?} raw={:?}",
                s.evs.iter().map(|e| (e.id, e.it)).collect::<Vec<_>>(),
                g,
                r
            ));
        }
    }
    assert_eq!(checked, 32_768, "比對覆蓋全部倒掛宇宙狀態");
    assert!(
        diverge.is_empty(),
        "CT 菜單上 Guarded 與 Raw 應為同一組規則(側條件在 CT 上冗餘);分歧:\n{}",
        diverge.join("\n")
    );
}
