//! cl0r0 —— 雙載體(CL0 定律載體 + R₀ 實用載體)的機械自証代碼庫。
//!
//! 六層基礎 → 九條定律(L1–L9)→ 雙載體規格(§7):
//!   * `span`  : σ(v) = [a,b) 半開區間(§1.2)
//!   * `lex`   : CL0 詞法器 —— DFA,平鋪全源碼,trivia 保留(§5.1)
//!   * `parse` : 表面語法樹 + 增量重析(配置快照重用)+ ERROR 全化(§1–§2.3)
//!   * `tree`  : 樹的性質檢查(連續性公理、laminar、CW 復形)
//!   * `sexp`  : 序列化(決定論 / 等價性的載體)
//!   * `edit`  : 編輯單體(§2.1:位移函數、複合、結合律)
//!   * `ast`   : 語義面 —— liveness 三軌(lexical / NLL / referent,§3.2)、
//!     衝突圖(區間圖 ⊂ 弦圖 ⊂ 完美圖,§3.3)
//!   * `rep`   : 修法菜單重寫系統(§4:L8 遞減測度、L9 合流)
//!   * `gen`   : 合法程式生成器 + 髒輸入生成器(屬性測試的輸入宇宙)
//!   * `r0`    : R₀ Rust 子集 —— 附錄 B 覆蓋面契約 + 正則詞法 + unsupported 申報
//!   * `l9newman` : 機械的 Newman 通道(終止 + 局部合流 ⇒ 合流 ⇒ 唯一正規形)
//!   * `shrink` : 反例最小化(ddmin)—— fuzz 失敗時縮到最小反例,防回歸

#![warn(missing_docs)]

pub mod ast;
pub mod edit;
pub mod gen;
pub mod l9newman;
pub mod lex;
pub mod parse;
pub mod r0;
pub mod rep;
pub mod shrink;
pub mod span;
pub mod tree;
