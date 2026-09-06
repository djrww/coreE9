# 代碼檢核批次:除錯 + 減債 + 去重(DEBT-REVIEW,2026-09-07)

> 範圍:全 src + tools 檢核(非 parity 驅動;probe 實證後修復,回歸測試入庫)。
> 紀律不變:律先於碼、機械自証、如實申報。所有修復皆有具名回歸測試或既有律矩陣覆蓋。
> 驗證鏈(rustc 1.98.1,2026-09-07 本沙箱):fmt-clean ∧ clippy 0 warning(含 oracle
> feature)∧ `cargo test --all` 全綠(32 律 + 21 單元,含 6 個新回歸)∧ O 系 6/6 ∧
> oracle gate 39/39(BUG 0、parity 24/24 註冊、兩趟全等)∧ rustdoc -D warnings ∧
> 三 bin(cl0r0 / fuzz / l9newman)實跑輸出不變(除已申報的打印調整)。

---

## 一、修復的缺陷(7 項,全部 probe 實證)

| # | 位置 | 現象 | 根因 | 修復 | 回歸 |
|---|---|---|---|---|---|
| B1 | `r0::parse_params` | `fn f(,) {}` / `fn f(, a: int) {}` / `fn f(a: int,) {}` 皆解析成功(0 ERROR) | 舊迴圈頂部 `Comma => bump` 臂對**導前逗號**無防線;尾隨逗號在 EBNF(`params = param { "," param }`)之外卻被接受 | 重寫為 EBNF 精確形:首 token 非 `)` 即須是 param;逗號後必須是 Ident | `r0_params_strict_ebnf` |
| B2 | `r0::parse_struct_item` | `struct S { a: int, , }` / `struct S { , a: int }` 解析成功 | 同 B1:Comma 臂不收前衛 | `seen_field` 前衛 + 逗號後只許 Ident(下一 field)或 RBrace(尾隨,EBNF `[","]` 合法) | `r0_struct_fields_strict_ebnf` |
| B3 | `r0::unsupported` | raw string 內的 `trait`/`impl`/`use`/`'a` 被誤報為越界構造(如 `let s = r#"trait"#;` → 3 項誤報) | 關鍵字/生命週期掃描是**裸字節掃描**,不感知 RawString token;舊註釋宣稱「raw string 已整 token 化故無誤報」——對 lexer 真、對這個 byte-scan 假 | 掃描改為 token 感知:先取全部 RawString span,命中區間內不申報 | `r0_unsupported_skips_raw_string` |
| B4 | `r0::unsupported_item` | 每解析一個 unsupported 項**洩漏一個堆字符串**(20 萬次解析 VmRSS +9.4MB) | per-call `Box::leak` 構造 note | note 改取共享靜態表 `excluded_kw_note`(有限集;與 `unsupported` 掃描共用,同一構造同一標注) | probe 實測 +9.4MB → +32KB;`r0_parse_unsupported_nodes` 持續覆蓋 |
| B5 | `r0::R0Parser::peek2_kind` + `lalr1_clean` | 帶空格的 `Vec < int` 判為 Error 節點(無空格版是 Unsupported);`lalr1_clean` 對帶空格版漏判(Ok) | `peek2` 只取 `pos+1`(trivia 擋路);lalr1 的 windows 含 trivia token | `peek2` 跳過 trivia 取下一結構 token;lalr1 改結構 token 序列 windows | `r0_spaced_generic_is_unsupported_not_error` |
| B6 | `r0_lex` `&mut` | `&mut` 恰在 EOF 時拆成 `&` + `mut`;`&mutx` 被合併成 `&mut` + `x`(rustc 分詞是 `&` + `mutx`) | 邊界 `i + 3 < len` 漏 EOF;無 `mut` 整詞邊界檢查 | `i + 3 <= len` + 後隨非 ident 字符(與 rustc 分詞一致) | `r0_lex_ampmut_word_boundary` |
| B7 | `r0::parse_fn_item` / `parse_let` / 舊 `parse_params` | 深 `&` 鏈型別(param 型別 / let 型別 / 回傳型別)越過 `R0_RECURSION_LIMIT` 時被吞成 `Ok(tree with Error)` 而非如實 `Err(Depth)` | `if let Err(Syntax) = …` 不匹配 `Depth`,後者被後續 item_err/continue 吸收 | 三處改 `match`,Depth 直接上報(與 R₀ 自述合同「唯一例外以 Err(Depth) 如實報告」一致) | `r0_depth_reported_not_swallowed` |

**影響面**:B1/B2/B7 改變的是「EBNF 之外的輸入」的節點分類 —— corpus/curated 39 案例全數
EBNF 內(實查無 trailing comma / 帶空格泛型),parity 基線 38/39 in-scope、24/24 註冊、
0 BUG 全部不變(實跑 oracle gate 綠)。

## 二、減債(死代碼 / 冗餘 / API 隱患)

| 位置 | 債 | 處置 |
|---|---|---|
| `r0::unsupported` | 9 元組佔位死迴圈(`for w in […] { let _ = w; }`)+ 空 match 臂(Ident) | 刪除 |
| `r0::bump(kind)` | 參數 `kind` 被 `let _ = kind` 忽略 —— API 說謊,且 `parse_type` 的 Amp 臂曾傳錯值(AmpMut token 傳 `R0Kind::Amp`) | 參數移除,葉子種類一律取自 token;48 處呼叫點更新;運算符迴圈 5 處 `let k = …` 死計算一併刪 |
| `ast::intervals` | 返回 `(Vec<Vec<Interval>>, Vec<Event>)`,第二分量是 `facts.events` 的全量克隆(每軌一次) | 改回單值;5 處呼叫點改取 `facts.events` |
| `ast::intervals` | 迴圈尾 `let _ = i;`(未用變量壓制) | 刪除 |
| `rep::l8_check` | if/else-if 兩臂**完全相同**的死重複(Guarded/Raw) | 合為單判據(語義不變) |
| `lex::TRANSLIT` | 死常量 + `let _ = TRANSLIT;`(「保留標記」) | 刪除(keyword_of 的 match 是唯一事實源) |
| `parse::Tree::sexp/named_sexp` | 方法內巢狀 `pub fn go`(冗餘 pub ×2) | 降為 `fn go` |
| `bin/cl0r0` | 尾端 `let _ = (total_red, Rule::R1Shorten(0,1), Kind::Root, Span::new(0,0))` 壓制死變量 + 3 個為它而存的 import | 刪除;`total_red` 改入打印(「三軌紅邊共 N 條」) |
| `bin/fuzz` | `#[allow(dead_code)] fn sp()` + 為它而存的 Span import | 刪除 |
| `model::tests` | `let y_ev = …; let _ = y_ev;`(未用計算) | 刪除 |
| `tools/patch_*.py` ×4 | 一次性遷移腳本(共 1953 行),CI/文件零引用,變更已入碼、git 史可溯 | 刪除 |

## 三、去重(結構層)

1. **雙載體樹檢驗面合一(主要)**:`Tree`(CL0)與 `R0Tree`(R₀)各有一份
   `validate_continuity` / `validate_tree_shapes` / `laminar_ok` / `unparse` 的相同邏輯
   —— 同條公理兩份實現,是 HARD-ITEMS #2「鏡像漂移」稅在樹檢驗層的面。
   抽為 `tree::NodeView` trait + `check_continuity` / `check_tree_axioms` /
   `check_laminar` / `unparse_all` 單一實作;兩載體方法改為一行的委託(公開 API 不變,
   47 律矩陣 + L1 回環(含 id 序 = 源碼序的等價性)全綠驗證)。
2. **作用域查找合一**:CL0 `ast::lookup` 與 R₀ `model::lookup` 逐字相同 →
   `ast::lookup_binding`(pub),model 側刪除本地副本。
3. **排除關鍵字標注合一**:`unsupported`(掃描器)與 `unsupported_item`(解析器)
   各有一份 16 項 match 表(措辭略有漂移,如 `dyn` 走 fallback)→ 共享
   `excluded_kw_note`(兼修 B4 洩漏)。

**保留不去重(如實申報)**:`ast::extract` 與 `model::extract_r0` 的兩遍收集
(decls → events)結構同構但操作不同樹形/語句集,統一需通用樹訪問器 —— 屬
P4 路線下「雙載體」設計本身的代價(PIVOT §〇-3 已裁決以 rustc 替代手寫鏡像來
消此稅,而非重寫載體);此處不強並,留待 P4-2+ 視差分熱點裁決。

## 四、殘債(本次未動,如實掛號)

1. **`release-v0.1.1/`(14MB,repo 內)**:v0.1.1 的發布產物(二進制 + docs HTML)。
   repo 無 git tag —— 此目錄是該版本唯一的工件載體,故**未刪**。建議:打
   `v0.1.1` tag 後把產物移 GitHub Releases,repo 留 RELEASE.md 即可。
2. **`rust-toolchain.toml` 只釘 channel**:精確釘版 + stable×2 + nightly 矩陣是
   PIVOT §四裁決的 P4-2+ 工程項(基線漂移偵測現由 corpus/BASELINE.json 的
   rustc --version 承擔)。
3. **`Ctx::Lhs` / `Ctx::Borrowed`(ast.rs)**:保留語義槽(有 `#[allow(dead_code)]`
   與註釋),R₀ 賦值面才使用;非死債。
4. **oracle bin 手寫 JSON 輸出**(`jstr` + format! 拼串):零依賴哲學下的自帶方案,
   與 mini_json(解析側)對稱;若 P4-2 起 JSON 面擴張,再考慮統一。
5. **bench 基線**:本次 dedup 略改熱路徑(unparse id 序、共享檢驗),bench gate
   ±25% 容差內理論無感;CI 首跑若紅,按既有紀律 `--update` 重刷(不手調容差)。

## 五、驗證記錄(2026-09-07,rustc 1.98.1)

- `cargo fmt --all --check`:clean(基線原已 fmt-clean,diff 內格式變動皆屬新碼)。
- `cargo clippy --all-targets --all-features`:0 warning。
- `cargo test --all`:32 律 + 21 單元全綠(新增 6 回歸:`r0_params_strict_ebnf`、
  `r0_struct_fields_strict_ebnf`、`r0_unsupported_skips_raw_string`、
  `r0_spaced_generic_is_unsupported_not_error`、`r0_lex_ampmut_word_boundary`、
  `r0_depth_reported_not_swallowed`)。
- `cargo test --features oracle`:O 系 6/6(含 P4-1 主律 `oracle_parity_borrow_matrix`)。
- `tools/oracle_gate.py corpus/curated corpus/BASELINE.json --parity`:
  39/39,BUG 0,兩趟全等,parity 24/24 註冊、0 stale —— 與修復前基線一致。
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`:clean。
- bin 實跑:`cl0r0`(九律演示,新增三軌紅邊總計打印)、`fuzz`(種子 0xC1020240001,
  總失敗 0)、`l9newman`(CT 全域驗證通過;Naive 如實報告 WCR 違反 —— 行為不變)。
