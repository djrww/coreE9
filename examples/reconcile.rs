//! P0-c 對帳基準(Rust 側):輸出「標準事實」,供 `tools/rocq_reconcile.py`
//! 生成 Rocq 檢查檔 —— Rocq 端用 vm_compute 重新計算,由 kernel 以 eq_refl
//! 驗證兩邊一致。資料(樣本狀態)兩邊字面重複定義(見 ROCQ-TRACE §三),
//! 函數語義則是各自實現、互相驗證。
//!
//! 用法:cargo run --example reconcile  →  只印 `key=value` 行。

use cl0r0::ast::Interval;
use cl0r0::rep::{apply, enumerate_states, AState, Ev, Menu, Policy, Rule, K};

fn main() {
    // ── 狀態枚舉計數(小空間)──
    for (n, m) in [(1usize, 3u32), (2, 3), (2, 4), (3, 3), (3, 4)] {
        println!("count({},{})={}", n, m, enumerate_states(n, m).len());
    }

    // ── 標準樣本(兩邊字面一致;ID 0..3 = 位置索引,D5)──
    let canon = AState {
        evs: vec![
            Ev {
                id: 0,
                storage: 0,
                kind: K::Mut,
                it: Interval { start: 0, end: 4 },
            },
            Ev {
                id: 1,
                storage: 0,
                kind: K::Sh,
                it: Interval { start: 1, end: 3 },
            },
            Ev {
                id: 2,
                storage: 0,
                kind: K::Sh,
                it: Interval { start: 2, end: 5 },
            },
            Ev {
                id: 3,
                storage: 1,
                kind: K::Mut,
                it: Interval { start: 0, end: 2 },
            },
        ],
        runtime: vec![(1, 2)],
        log: vec![],
    };

    println!("red_edges={:?}", canon.red_edges());
    println!("red_count={}", canon.red_edges().len());
    println!("measure=({},{})", canon.measure().0, canon.measure().1);
    println!("nf={}", canon.is_normal_form());
    println!(
        "appli(ct,g)={}",
        Menu::CommutativeTrim
            .applicable(&canon, Policy::Guarded)
            .len()
    );
    println!(
        "appli(ct,r)={}",
        Menu::CommutativeTrim.applicable(&canon, Policy::Raw).len()
    );
    println!(
        "appli(naive,r)={}",
        Menu::Naive.applicable(&canon, Policy::Raw).len()
    );

    // ── 規則應用(apply 的語義對帳)──
    match apply(&canon, Rule::R1Shorten(0, 2)) {
        Some(s2) => {
            println!("r1_red={}", s2.red_edges().len());
            println!("r1_iv=({},{})", s2.evs[0].it.start, s2.evs[0].it.end);
        }
        None => println!("r1_err=1"),
    }
    match apply(&canon, Rule::R2Split(1, 2)) {
        Some(s2) => println!(
            "r2_storage={:?}",
            s2.evs.iter().map(|e| e.storage).collect::<Vec<_>>()
        ),
        None => println!("r2_err=1"),
    }
    match apply(&canon, Rule::R3Swap(0, 2)) {
        Some(s2) => println!(
            "r3_iv=({},{},{},{})",
            s2.evs[0].it.start, s2.evs[0].it.end, s2.evs[2].it.start, s2.evs[2].it.end
        ),
        None => println!("r3_err=1"),
    }
    match apply(&canon, Rule::R4Runtime(0, 1)) {
        Some(s2) => {
            println!("r4_red={}", s2.red_edges().len());
            println!("r4_runtime={:?}", s2.runtime);
        }
        None => println!("r4_err=1"),
    }
}
