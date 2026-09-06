# 檢核 & 除錯 & 減債 & 去重 · 審計報告

> 日期:2026-09-07(香港時區)
> 對象:`cl0r0` v0.1.1 · main @ `3590375`
> 審計目標:檢核專案程式碼、除錯、減低技術債、去除重複。
> 審計方法:全量建置 → 全量測試 → clippy `-D warnings` → rustfmt → rustdoc `-D warnings` →
> `fuzz` 屬性測試 → `oracle_gate`(rustc 判決對帳 + 三軌 parity)→ 釋出建置(LTO=fat)。
> **審計前與審計後,上述所有門檻皆綠,47/47 具名測試 + 6 條外律 O 系測試不變。**

---

## 〇、總覽(決策摘要)

| 類別 | 項目 | 狀態 |
|---|---|---|
| 正確性 | `ast::conflicts()` 的檔案明與實作不一致(含一個**被淘汰的比對分支**) | ✅ 已修(改寫為對稱比對 + 修正檔案明) |
| 正確性 | `rep::l8_check()` 兩條全然相同的分支 | ✅ 已修(合併為單一判斷) |
| 減債 | 四個「一次性補丁」腳本已無人引用(共 ~1,963 行死碼) | ✅ 已刪(`tools/patch_*) |
| 減債 | 死代碼 / 警告抑制殘留(`TRANSLIT`、`total_red`、`sp`、`Ctx::Lhs/Borrowed`、`ReuseData.edits`、未用匯入) | ✅ 已刪 |
| 去重 | `recover_wrap` / `absorb_to` 的 15 行括號追蹤迴圈**完全重複** | ✅ 已抽取為 `consume_to_sync` |
| 去重(未動) | `ast.rs` ↔ `model.rs` 的語義抽取幾乎同構;`parse.rs`/`r0.rs`/`tree.rs` 的樹性質方法重複 | ⚠️ 建議,附理由(見 §四) |

**淨變動**:6 個 `.rs` 檔修改(+44 / -87 行),4 個孤立腳本刪除。測試/靜態/基準/rustdoc/oracle 全綠。

---

## 一、正確性審計(找到並處理的)

### 1.1 `ast::conflicts()` — 檔案明與實作不一致 + 一個「失效比對分支」(已修)

這是**唯一的實際缺陷**,但性質很微妙,值得說清楚:

- **檔案明**(舊)聲稱:`&mut` 與「讀/移/借用/解引用」衝突。
- **測試判據表**(`tests/laws.rs::test_law_semantic_conflict_matrix`)明定:
  `(BorrowMut, Deref) = false`,註解明說「解引用是借用鏈的內部使用,非相容性衝突」。
- **實作**則用「枚舉值降序排序」的技巧:
  ```rust
  if b as u8 > a as u8 { std::mem::swap(&mut a, &mut b); }
  matches!((a, b), ... | (BorrowMut, Deref) | ...)
  ```
  因為 `Deref` 是枚舉**最大**值(5),任何含 `Deref` 的配對排序後 `Deref` 一定落在 `a`(較大端),
  而舊匹配列只有 `(BorrowMut, Deref)`(即「Deref 在較小端」),**永遠不可能命中**。
  實測:`conflicts(BorrowMut, Deref) == false`,與測試判據一致,但與檔案明矛盾。

**結論**:實作「碰巧」符合測試判據(因為該列是死碼),但這個「值域排序」手法極脆:
只要日後有人調整 `EvKind` 的排列、或把某個衝突對寫成「大端在前」,就會**靜默改寫相容性語義**,
而不會產生任何編譯/測試錯誤。同時檔案明(先前的)也誤導。

**修法**:改寫為**對稱全寫**的衝突對集合(語義與測試判據完全一致),並把檔案明改正,
明確註記「`Deref` 不構成相容性衝突」。這同時消除對枚舉值域的隱性依賴:
```rust
pub fn conflicts(k1: EvKind, k2: EvKind) -> bool {
    use EvKind::*;
    matches!(
        (k1, k2),
        (BorrowMut, BorrowMut)
            | (BorrowMut, BorrowSh) | (BorrowSh, BorrowMut)
            | (BorrowMut, Read) | (Read, BorrowMut)
            | (BorrowMut, Move) | (Move, BorrowMut)
            | (BorrowSh, Move) | (Move, BorrowSh)
    )
}
```
> 語義與原測試判據逐對相等;`cargo test --all` 之後仍全綠(含 `test_law_semantic_conflict_matrix`)。

### 1.2 `rep::l8_check()` — 兩條全然相同的分支(已修)

```rust
if policy == Policy::Guarded {
    if !AState::strictly_decreases(s2.measure(), s.measure()) {
        return Some(...);
    }
} else if !AState::strictly_decreases(s2.measure(), s.measure()) {
    return Some(...);
}
```
兩個分支的**判斷條件完全一致**,`policy` 分支在此沒有資訊量(Guarded 的「遞減」由 `applicable` 保證,
Raw 的「遞減判定」是同一條件)。這是一種「同一邏輯兩處寫」的債:日後改判據時極易只改其一。

**修法**:合併為單一判斷,並註明「Guarded / Raw 在『是否嚴格遞減』上共用同一判據」。

### 1.3 其餘可疑點經**實測排除**(記錄以免重查)

- `conflicts` 中 `Deref` 與任何事件都不衝突 → **非 bug**,是設計(見 1.1 判據表)。已寫入檔案明。
- `ast::intervals` Referent 軌的`借鏈 span` 配對(`l.span == ev.span`)→ 已兩處一致(`ast`/`model`
  都以「源識別字 span」取),非死代碼。
- `oracle.rs` 的 `line_start_table` / `lc_to_byte` 對非 ASCII 用「字節列」→ 已於檔案明「如實申報」為
  已知限制,非缺陷(僅 ASCII 語料入 base)。
- `rep.rs::Menu::applicable(CommutativeTrim)` 的 `cut` 啟發式 → 依設計(規範 cut = 最早衝突起點),有
  `Newman` 通道機械驗證,非隱藏 bug。

---

## 二、減債(已執行的清理)

| # | 位置 | 問題 | 處理 |
|---|---|---|---|
| 1 | `tools/patch_docs_ast.py` (175 行) | 一次性「注入 doc comment」腳本;`ast.rs` 已直接內建該內容,腳本殘留 | 刪除 |
| 2 | `tools/patch_docs_rep.py` (172 行) | 同上(針對 `rep.rs`) | 刪除 |
| 3 | `tools/patch_r0_parse.py` (1465 行) | 內嵌整段 `r0_parse` 原始碼;該碼已**直接**位於 `src/r0.rs`(2034 行起)→ **1400+ 行重複**,且腳本再跑會破壞現檔 | 刪除 |
| 4 | `tools/patch_r0_tests.py` (151 行) | 同上(針對測試) | 刪除 |
| 5 | `src/lex.rs` | `const TRANSLIT` + `let _ = TRANSLIT;`(純死碼,僅為占位) | 刪除 |
| 6 | `src/bin/cl0r0.rs` | `total_red`(計算從不輸出)+ 末尾 `let _ = (total_red, Rule::R1Shorten(0,1), Kind::Root, Span::new(0,0));` 的**警告抑制 hack** + 相應未用匯入 `Kind`/`Rule`/`Span` | 刪除 |
| 7 | `src/bin/fuzz.rs` | `#[allow(dead_code)] fn sp`(死函數)+ 其唯一使用者的 `Span` 匯入 | 刪除 |
| 8 | `src/ast.rs` | `collect_decls` 的 `param_scope: bool` 參數(從未使用,`let _ = param_scope;`) | 刪除參數 |
| 9 | `src/ast.rs` | `Ctx` 的 `Lhs` / `Borrowed` 變體(無人使用;`model.rs` 側早已只剩 `Value`/`CallArg`)+ 整個 enum 的 `#[allow(dead_code)]` | 刪除 |
| 10 | `src/ast.rs` | `intervals` 迴圈的 `let _ = i;` + 重複的 `RedEdge` doc-comment | 刪除 |
| 11 | `src/parse.rs` | `ReuseData.edits` 欄位(在建構後從未讀;僅 `#[allow(dead_code)]` 支撐) | 刪除欄位 |
| 12 | `src/parse.rs` | `link()` 中先算 `(cstart, cend)` 再 `let _ = (cstart, cend);` 的無效計算 | 刪除 |

> 審計後 `src/` 與 `tests/` 皆 **0 處 `#[allow(dead_code)]`**,clippy `-D warnings` 仍 0 警告。

---

## 三、去重(已抽取的共用邏輯)

### `parse.rs`:抽取 `consume_to_sync`

`recover_wrap`(開一個 ERROR 節點再吞殘骸)與 `absorb_to`(吞殘骸進既有錯誤節點)的
「括號/大括號深度追蹤 + 同步邊界停止」迴圈**逐字元一致**。原因:兩者共用同一「吞殘骸」語義。

**修法**:抽出一私有方法 `consume_to_sync(&mut self, mode)`;`recover_wrap` 與 `absorb_to` 各自呼叫。
消除 ~15 行重複,並把「壞構造殘骸只落進同一錯誤區(L7b 極大性)」的註解集中在一處。
`test_law_L7*` / `smoke_parse_garbage` / `fuzz` 之後仍全綠。

---

## 四、仍存在的重複 / 債(未改,附建議)

> 以下為**建議性**項目。因本專案的每個函數都綁定一條律(L1–L9 / M / T2 / O 系)
> 與規格段落(SPEC-TRACE),這類「結構性」重構風險較高,我**保留不動**並說明理由;
> 若要動,宜先取得 SPEC-TRACE 對帳與 R³ 凍結資產的尊重。

| # | 位置 | 重複/債 | 建議 |
|---|---|---|---|
| A | `ast.rs`(CL0 抽取) vs `model.rs`(R₀ 抽取) | `collect_decls`/`EventCollector`/`walk_let`/`walk_unary`/`lookup`/`name_of`/`DeclSite` 幾乎同構,僅`Tree`/`R0Tree` 與表達式粒度不同 | 若要合併,引入「共享樹」trait(節點/子節點/種類三件)把語義抽取參數化;**風險**:兩個載體的事件語義目前有刻意差異(CL0 的 `Block` 語句位置 / R₀ 的扁平 `Expr`),宜先以差異測試鎖定 |
| B | `parse.rs` / `r0.rs` / `tree.rs` | `validate_continuity`/`validate_tree_shapes`/`laminar_ok`/`n_errors`/`has_error`/`named_sexp`/`unparse`/`total_nodes` 在 `Tree` 與 `R0Tree` 上重複 | 抽 `TreeLike` trait(def:`nodes`/`node(kind)`/`src`/`span`),把幾何檢查泛型化;可直接去 ~200 行 |
| C | `tools/bench_gate.py` ↔ `tools/oracle_gate.py` | 都各自實現「best-of-n + null 標尺漂移校正 + 基線 `--update`」統計紀律 | 抽共用 `gate_lib.py`(null 對照 / 判決定義);**低風險**,但兩工具目前解耦亦可接受 |
| D | `release-v0.1.1/` | 內含舊版 rustdoc HTML(上百檔)+ 編譯產物 `cl0r0` | 若能接受,移至釋出 tag 附帶 / git-lfs;或整目錄改為「釋出資產」不收進主源碼樹 |
| E | `src/r0.rs` 自帶迷你 JSON 以外的「自製解析器」族 | `r0_lex`/`r0_parse` 與 CL0 `lex`/`parse` 是兩套獨立的 DFA + 遞歸下降 | 屬**刻意**雙載體(九律在 CL0 上驗證、R₀ 作實用面),不建議合併;只需確保 `unsupported` 面與 `R0_EBNF` 同步(數據驅動,見 PIVOT §八) |

---

## 五、審計後驗證(全部綠)

| 門檻 | 結果 |
|---|---|
| `cargo build --all-targets` | ✅ |
| `cargo build --release`(LTO=fat, codegen-units=1) | ✅ |
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ 0 |
| `cargo clippy --all-targets --features oracle -- -D warnings` | ✅ 0 |
| `cargo test --all` | ✅ 47/47(15 單元 + 32 `laws`) |
| `cargo test --features oracle` | ✅ 53/53(15 單元 + 32 `laws` + 6 `oracle_laws`) |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | ✅ |
| `cargo run --bin fuzz`(種子 `0xC1020240001`) | ✅ 總失敗數 0 |
| `tools/oracle_gate.py corpus/curated corpus/BASELINE.json --parity` | ✅ 39 案例、BUG 候選 0、兩趟全等、基線對帳一致、parity 註冊表 24/24 |

---

## 六、給後續迭代的一句建議

本次審計最值得記住的一點:**這是一個「九律 / 神話名稱都對得上號」的高規格實驗碼庫,
單獨看每一段都「有據可查」,真正的風險在於「跨載體的結構性重複」與「依賴枚舉值域/隱性排序的脆技巧」**。
前者宜以共享 trait 收斂(建議項 A/B),後者宜一律改成「對稱全寫」的顯式判據(本次 `conflicts` 即範例)。
在 P4-P7 以 rustc 為權威指標的差分開發下,這類「語義判據」會是被差分反覆錘打的熱點 ——
**判據要寫得「一眼即對、無排序依賴」,才經得起版本矩陣的錘。**
