# 外律 O 系對帳表(ORACLE-TRACE)

> P4 路線的對帳中樞(對應 `docs/PIVOT-RUSTC-ORACLE.md`;地位等同 Rocq 時代的
> `ROCQ-TRACE.md`——工具換了,對帳紀律不換)。
> 本文一切數字為**實測**(rustc 1.98.1 (48a229cea 2026-09-01),2026-09-06,
> 本沙箱),不是文件宣稱;重現:`cargo test --features oracle` →
> `python3 tools/oracle_gate.py corpus/curated corpus/BASELINE.json --parity`。

---

## 〇、現狀(P4-0 + P4-1 已交付)

| 交付物 | 內容 | 狀態 |
|---|---|---|
| `src/oracle.rs`(feature = "oracle") | `trait Oracle` + `CliOracle`(Tier-A)+ `Verdict`/`RustcError`/`Expectation` + 自帶 mini JSON 解析器(零新依賴) | ✅ |
| `src/model.rs`(feature = "oracle") | **受驗對象**:`extract_r0`(R₀ 樹 → `ast::Facts`)+ `model_check`(三軌紅邊判決)+ `parity_dir`/`parity_violations`(對帳引擎)+ 註冊表解析 | ✅ |
| `tests/oracle_laws.rs` | 外律 O 系 ×7(見 SPEC-TRACE 〇′′),含 P4-1 主律 `oracle_parity_borrow_matrix` + P4-2 主律 `oracle_fuzz_agreement` | ✅ 7/7 |
| `corpus/curated/`(39 案例)+ `corpus/BASELINE.json` | 種子 8 → **39 案例**(24 accept / 14 reject / 1 uncoded;模板系統化生成,碼由 rustc bootstrap 實測命名);基線 39/39 | ✅ |
| `corpus/PARITY-REGISTRY.json` | 24 項已歸檔分歧,7 個家族,每項強制附出處 | ✅ 24/24 命中,無 stale |
| `tools/oracle_gate.py`(含 `--parity`) | 兩趟全等 + BUG=0 + 基線對帳 + parity(判定單一源於 Rust 側,腳本只中繼) | ✅ |
| CI `oracle` job | clippy(帶 feature)→ O 系 → gate --parity;rocq job 凍結(`continue-on-error`) | ✅ |
| core 零依賴 | 默認構建不編 oracle bin;`cargo test --all` 47 條全綠不受影響 | ✅ 實測 |
| **P4-1a(2026-09-06)** | CL0 `ast::extract` 事件真空修復(發現 #1)+ 借鏈構造修正(發現 #2 的 CL0 側)+ 兩具名測試補反真空斷言 | ✅ 47/47;parity 不變(24/24) |

---

## 一、P4-1 三大發現(parity 驅動曝光;oracle 路線的第一批回報)

> 以下三項均在 2026-09-06 由 parity 對帳實證發現,證據可重現。
> 共同根因:CL0 載體的語義面事件層從未被非空斷言覆蓋 —— 結構性測試
> (一致性/廣度)對「零事件」**空轉通過**。這正是 HARD-ITEMS #2「樣本點由
> 行為生成,不由名詞生成」要防的事,只是發生在自己身上。

### 發現 #1:CL0 語義面事件層真空(嚴重;✅ 已修 —— P4-1a,2026-09-06)

**現象**:`ast::extract`(CL0 載體)對一切程式產出 **bindings 有、events = 0、
links = 0**。三軌 liveness / 衝突圖 / 紅邊在 CL0 上是對空集合運算 ——
`test_law_semantic_facts_consistent` 與 `test_law_semantic_extract_breadth`
全部通過,因為它們**從不斷言事件非空**。

**根因**:`EventCollector` 事件遍行期間不管理作用域棧(遍行前 `collect_decls`
已把棧彈空),`emit` 的名稱查找永遠落空。

**實證**:`bindings=3 events=0 links=0` × 3 個 CL0 樣本(2026-09-06 探針)。

**處置(✅ P4-1a 已交付,2026-09-06)**:CL0 `ast.rs` 與 R₀ 側同構修復 ——
`decl_site` 聲明側表(collect_decls 建立)+ 事件遍行期間真正管理作用域棧
(FnItem 參數層 / Block 層 / let 走完 init 才註冊 = 遮蔽語義)。附帶修復:
借用初始化**同時發借用事件**(舊碼只建鏈不發事件)。反真空紀律:
`test_law_semantic_facts_consistent` 與 `test_law_semantic_extract_breadth`
補「事件非空」斷言(門檻按實測校準:gen_legal×200 → 119 有事件/248 事件/7 鏈,
門檻取六成邊際)—— 真空復發時必紅。修復後 47/47 全綠;T2 χ=ω 斷言首次
對非空區間數據運行並通過;parity 不變(24/24,R₀ 側本就正確)。

### 發現 #2:借鏈配對死代碼(✅ 模型側與 CL0 側均已修)

**現象**:`BorrowLink.span` 記的是**引用綁定名**(`r`)的 span,而
`ast::intervals` Referent 軌以 `l.span == ev.span` 配對借用事件 —— 借用事件
的 span 是**源 ident**(`x`)的。兩者永不相等 ⇒ 借鏈永遠配不中 ⇒
Referent 軌全量退化為「保守至作用域末」。

**處置**:`model.rs` 與 CL0 `walk_let`(P4-1a)構造借鏈時均改用源 ident span,
配對命中;Referent 軌自此對 let 形式借用真正生效。

### 發現 #3:Referent 軌終點下限錯誤(已修 `ast.rs`)

**現象**:`intervals()` Referent 軌終點初值取 `scope.end` 再 `.max(使用點)`
—— 只能伸、不能縮,借用活性恆延至作用域末(過度保守),「最後使用點」
語義從未生效。

**處置**:已修(`src/ast.rs`,2026-09-06):終點自借用點起步,取引用事件
end 之最大值;**無使用 ⇒ 立即死亡**(與 NLL「未用借用即死」一致)。
CL0 行為不變(事件真空);R₀ 模型即時受惠(parity 由 29 項分歧收斂至 24 項)。

---

## 二、判決抽取的實證契約(對 rustc 1.98.1 的實測結論)

這些是 `CliOracle` 賴以成立的**行為事實**,逐條有實測背書;若未來 rustc 版本
改變其中任何一條,gate 的「判決漂移」通道會先紅(§三),屆時更新本表並如實
記錄版本差異。

| # | 事實 | 實測證據 |
|---|---|---|
| F1 | `--error-format=json` 的診斷流走 **stderr**(stdout 留給產物,幾乎恆空) | 首版實作誤讀 stdout → 3 條 O 系測試即時紅;改 stderr 後全綠(2026-09-06) |
| F2 | 每行一個 JSON 對象;`level` ∈ {error, fatal, warning, note, help, failure-note};判決只取前兩者 | `oracle_smoke_e0502` 的輸入含 warning(`unused_variables`),未入判決 |
| F3 | **「aborting due to …」摘要行同為 error 級**,但 `code=null` 且 `spans=[]` ⇒ 以「無碼且無 span」結構化排除;否則每個拒絕都攜帶幻影 uncoded | `oracle_reject_uncoded_syntax_error`:uncoded 恰 1 條 |
| F4 | span 物件含 `byte_start/byte_end`(字節精確)⇒ 直接映射 §1.2 半開區間;line/col 換算僅作舊版退化路徑 | 基線記錄的 span 與源碼字節逐一對得上(§五) |
| F5 | 語法錯誤(uncoded)的主 span 可以是**零字節插入點**(start == end)—— 與 `span.rs`「空區間 = 插入點」的既有語義一致 | `uncoded_syntax_error.rs`:span = [25, 25) |
| F6 | 退出碼:接受 = 0;診斷後拒絕 = 1;無可解析診斷的非 0(如 ICE)= `RustcCrash`(不算判決) | 探針實測(2026-09-06) |
| F7 | 錯誤碼字段:對象 `{code: "E0502", …}` 或 `null`;**被 deny 的 lint 是非 E 碼**(如 "unused_variables")—— 判決如實轉發,不做白名單過濾 | warning 探針輸出實測 |
| F8 | 同輸入同版本兩趟判決全等(碼 + span 逐字節) | `oracle_determinism` + gate ① 兩趟全等 |
| F9 | `--emit=metadata` 的 `.rmeta` 產物**落地 CWD**(除非 `--out-dir` 導流)⇒ 必須導回 temp,否則語料跑者在倉庫根目錄撒垃圾 | 首版實測:repo 根目錄累積 60+ 個 `libcl0r0_oracle_*.rmeta`;修復後 CWD 乾淨 |

---

## 三、parity 對帳(P4-1 主表):rustc × 模型三軌

> 完整逐案例表:`cargo run --features oracle --bin oracle -- parity corpus/curated`。
> 比對粒度 = **accept/reject**(碼級 parity 是 P4-3;見 §六邊界)。
> L/N/R = Lexical/Nll/Referent 三軌是否拒絕(紅邊非空);⚠ = 與 rustc 分歧。

### 3.1 主結果(2026-09-06,rustc 1.98.1)

| 分組 | 數量 | 結果 |
|---|---|---|
| 借用衝突類拒絕(E0499 ×2 / E0502 ×3 / E0503 / E0506 ×3) | 9 | **三軌至少一軌全中**;其中 8 例三軌全拒(與 rustc 完全一致);**全部 9 例 Lexical 必拒 ⇒ 幾何保守下界定律成立**(O-6 斷言 ②) |
| 純讀取類接受(無借用互動) | 12 | 三軌全綠,零分歧 |
| 借用相關接受(NLL 行為) | 12 | Referent 與 Nll 各有少量 over(已歸檔,F-A/F-B/F-C) |
| 移動/不可變性/逃逸類拒絕(E0382/E0384/E0505/E0597) | 4 | 模型 under(幾何範圍外/移動未建模,已歸檔 F-F/F-G) |
| 語法錯誤(uncoded) | 1 | 模型範圍外(Error 節點不產事實),不參與對帳 |
| **合計** | **39**(38 在模型範圍內) | **gate 軌道分歧 24 項 = 註冊表 24 項;BUG 候選 0;stale 0** |

### 3.2 分歧家族註冊表摘要(詳表:`corpus/PARITY-REGISTRY.json`)

| 家族 | 方向/軌道 | 案例數 | 歸因 | 掛號 |
|---|---|---|---|---|
| F-A 軌道分工 | over(nll 5 / referent 3) | 8 | Nll 的寫事件無回溯截斷、Referent 的非借用事件無截斷 —— 兩軌各管一半活性,單軌必然 over | P4-3 S2 |
| F-B call-arg 借用無借鏈 | over(nll 1 / referent 2) | 2 案例 | `g(&mut x)` 無 let 綁定 ⇒ 無借鏈 ⇒ 保守;rustc 視臨時借用死於呼叫返回 | P4-3 S1 |
| F-C Copy 盲 | over(referent 1) | 1 | call-arg 單 ident 一律記 Move;rustc 視 `i32` 拷貝不殺借用 | P4-3 S1(型別面) |
| F-D Nll killer 過早 | under(nll 2) | 2 | 寫截斷借用區間,但借用活性由引用最後使用決定 —— 本類由 Referent 承接(同案 Referent 判紅 = 與 rustc 一致) | S2 |
| F-E 回邊活性 | under(nll 1 / referent 2) | 2 案例 | while 條件重複求值:source 線性最後使用 ≠ CFG 最後使用;模型無控制流 | P4-3 S2(CFG killer) |
| F-F let-init 移動未建模 | under(both,2 案例) | 2 | `let m2 = m;` 記 Read;`&mut` 非 Copy,rustc 報 E0382/E0505 | P4-3 S1 |
| F-G 範圍外(非幾何) | under(both,2 案例) | 2 | E0384 不可變性 = 型別面;E0597 作用域逃逸 = 生命週期求解;**如實申報不覆蓋** | 範圍聲明 |

**定律級結論(O-6)**:rustc 以借用衝突類碼(E0499/E0502/E0503/E0506)拒絕的
每一個在域案例,Lexical 軌必拒絕 —— **區間幾何保守下界覆蓋全部借用衝突現場,
零漏報**。這是三軌中第一條被機械驗證的「模型 ⇒ rustc 單向蘊含」。

### 3.3 gate 判別力演練紀錄

- 註冊表為空(發現輪):24+5 條未註冊分歧全紅 ✅(2026-09-06);
- 註冊 24 項後:GREEN,無 stale ✅;
- lexical blanket 誤設為「對一切碼不準漏報」時:e0382/e0384/e0505/e0597 紅
  ⇒ 收緊定義為「僅借用衝突類碼承諾零漏報」(精確化而非放水 —— 範圍外家族
  的 under 本就是申報過的邊界)✅;
- P4-0 的四情境演練(無基線/冒名案例/基線竄改/語料變化)見 git 歷史,全部正確變紅。

---

## 四、gate 紀律(bench_gate 教訓的 oracle 版)

1. **兩趟全等**(null 紀律):判決無計時噪聲,同機同版兩趟必須逐案例一致,
   不一致 = 環境在動,先紅不猜;
2. **BUG 候選 = 0**:判決不符檔名期望 → 紅;`--update` 不能洗白
   (基線如實記錄 `pass=false`);
3. **基線對帳**:版本漂移只警告;**判決漂移**才是紅,人工歸因後 `--update`;
4. **parity 註冊表**:gate 軌道(nll/referent)分歧必須精確命中註冊項
   (檔+軌+向),註冊項必須仍在發生(stale 即紅);Lexical blanket:
   借用衝突類碼零漏報,範圍外家族 under 需屬已申報集合;
5. 判定邏輯**單一源於 Rust 側**(`cl0r0::model::parity_violations`),
   gate 腳本只中繼 —— 防雙語言重複實作漂移。

---

## 五、種子語料 v1(39 案例;判決即基線,2026-09-06)

| 類別 | 案例數 | 覆蓋 |
|---|---|---|
| `accept_*` | 24 | 純讀取、NLL 最後使用、先後借用、嵌套/遮蔽作用域、參數、迴圈/分支、塊表達式、三個 MODEL-DIFF 探針(copy-call / cond-borrow / call-sh) |
| `e0xxx_*` | 14 | E0382 / E0384 / E0499×2 / E0502×3 / E0503 / E0505 / E0506×3 / E0597 |
| `uncoded_*` | 1 | 語法錯誤(零字節插入點,F5) |

碼標註方法:**模板系統化生成 → rustc bootstrap 實測命名**(生成器一次性,
不入庫;碼不是人手宣稱)。語料全 ASCII(line/col 退化路徑的邊界,見 §六)。

---

## 六、已知邊界(如實申報,不假裝覆蓋)

1. **比對粒度 accept/reject**:碼級(E0502 vs E0499)與 span 級 parity 是
   P4-3(需先落 S1 place 敏感 + 型別面);現以 rustc 碼做語料期望、以
   accept/reject 做模型對帳;
2. **單一裁判版本**:stable 1.98.1;stable×2 + nightly 版本矩陣待 CI 落地
   (`CL0R0_ORACLE_RUSTC` 已預留);
3. **非 ASCII 語料未入庫**(同 P4-0 邊界);
4. **CL0 載體語義面事件真空未修**(發現 #1)—— R₀(`model.rs`)是當前
   唯一非空的語義實現;CL0 修復排 P4-1a;
5. **O 系是外律**:測的是 oracle 通道與 parity 引擎;模型的幾何宣稱以
   §3.2 註冊表為準,範圍外家族明示不覆蓋;
6. oracle feature 需 PATH 有 rustc;缺 rustc 時 O 系以明確訊息失敗,
   不污染默認 47 條測試矩陣。

---

## 七、P4-2 生成式差分(已交付 2026-09-07)

> 機制:樣本**不是**隨機文本問 rustc 怎麼說,而是 `gen::gen_r0_semantic`
> by-construction 生成——借用紀律由生成器內部的活躍借用簿記把關
> (寫只在目標無活躍 let 借用時發出;&mut let 借用從不同時活躍;
> call-arg 借用死於呼叫返回;while 條件借用以迴圈出口為最後使用),
> 衝突模式被明確注入(5 種借用衝突形狀 + 1 種 use-after-move)。
> 期望判決於生成時已知,由此「期望 ≠ rustc 判決」= BUG 候選。

**實測(rustc 1.98.1,2026-09-07,本沙箱;重現:`cargo test --features oracle
--test oracle_laws oracle_fuzz_agreement`,或 `cargo run --features oracle
--release --bin fuzz`)**:

| 指標 | 數值 |
|---|---|
| 具名測試(250 輪,seed 0x0F4220260002) | 0 失配,0 範圍外 |
| fuzz 併軌(2000 輪,seed 0x0F4220260003,release) | 0 失配,0 範圍外 |
| by-construction 分布(2000) | accept 812 / 借用衝突 988 / 移動 200 |
| 借用衝突碼實見 | E0502(絕大多數)/ E0503 / E0506 —— 全落 BORROW_CONFLICT_CODES |
| 移動類實見 | E0382 |
| gate 軌道 over/under(2000;MODEL-DIFF 候選,非失敗) | nll 476/593;referent 402/410 |

**紀律**:① 判定權不轉移——gate 軌道分歧只記錄、歸因仍由 parity 註冊表單源
承擔;② 版本見證:`CliOracle::cached_version`(按工具鏈快取 `--version`,
fuzz 每樣本不再多一個子進程;O-4 兩趟全等律不變);③ 失敗縮小:O-7 測試
失敗時 ddmin 縮到最小反例 → 歸檔 `tests/fixtures/oracle_fuzz_<law>.txt`
(P0 #3 防線沿用);④ 探針史:交付前以 3000 輪獨立探針實測 0 失配(探針已清退,
正式載體 = O-7 + fuzz 併軌)。

## 八、P4-3 語義深化(完成,2026-09-07)

**動工前對照表(§七 = P4-3 前;P4-3 後 = 同一 seed 0x0F4220260003、
同一 2000 輪重跑)**:

| 指標 | P4-3 前(§七) | P4-3 後 |
|---|---|---|
| curated 案例 | 39 | **41**(+`accept_field_borrows_disjoint` / `accept_branches_disjoint`) |
| parity 註冊表 | 24 項 / 7 家族 | **4 項 / 1 家族(F-G 範圍外)** |
| fuzz gate over(nll, referent) | (476, 402) | **(0, 0)** |
| fuzz gate under(nll, referent) | (593, 410) | **(0, 0)** |
| fuzz 失配 | 0 | 0 |

**機制(src/ast.rs + src/model.rs,全部實測驗證)**:

1. **S1 place 敏感度**:`Event.place: Vec<String>`(字段鏈);`&s.a` /
   `&s.b` 不同 place ⇒ Nll/Referent 軌不衝突(rustc 接受,雙錨 O-8);
   同 place 雙 `&mut` 必拒(雙錨 O-8 對照)。`red_edges` 以 place_compatible
   過濾(Lexical 軌保持綁定粒度 = 幾何保守下界,如實過報)。
2. **S1 型別面**:`TypeClass`{Int, Ref, Unknown} + btypes 貫穿:
   - 單一 ident 讀:Ref ⇒ Move(consumes=true)→ dead-use 紅邊(e0382 機制);
     Int ⇒ Read;未知 ⇒ Move(false)。
   - `*p = …` 寫:p 為 Ref ⇒ consumes=true(其後再用 ⇒ e0382 死用邊);
     `*p = 1; *p = 2;` 雙拒,與 rustc 一致。
   - 根指派 `x = …`:consumes=(型別 Ref);字段指派 = Move(該 place)。
3. **S2 回邊活性**:while 條件反覆求值 ⇒ 迴圈前借用跨越迴圈出口
   (`e0506_assign_in_while_under_mut` 雙軌拒,rustc 拒)。精確化(實測收斂):
   - dies_at 臨時借用(呼叫實參)**不**回邊延伸 —— 精確限於呼叫;
   - let 形式借用僅當**引用綁定在迴圈內被使用**時跨越迴圈(未用引用即死;
     引用最後使用在迴圈前則迴圈內是直接訪問,不延續本借用)。
4. **區間語義收斂**:Read/Deref/Decl 在 Nll/Referent 軌皆**點事件**
   [s,s);值生命週期(e0382)改由 dead-use 紅邊承載,不再靠區間延伸
   —— 否則「讀區間 × 已死借用」虛假衝突(fuzz 實測 over 案例,已修)。
5. **Move = 點 [s,s)** + **linked borrow end = 引用綁定最後使用**
   (未用引用 ⇒ 點,NLL「未用借用即死」一致)+ **call-arg 借用 dies_at
   = 呼叫右括號 span 末**(S1 臨時區域)。
6. **軌道分工收斂**:E0506 家族由 nll 獨拒 → nll + referent 雙拒
   (原 F-D 分歧消滅);F-A(過報)隨 dies_at/點寫收斂。

**家族消滅對帳(註冊表 24 → 4)**:F-A 至 F-F 家族共 20 項
(過報:accept 行 11 項 nll/referent;欠報:e0382/e0502-loop/e0505/
e0506 共 9 項)全數轉 stale 並移除(gate 機器核實,2026-09-07 實測);
餘 4 項 = **F-G 範圍外家族**(E0384 不可變性 ×2、E0597 作用域逃逸 ×2)
—— 如實申報不覆蓋,保留註冊。

**具名律(雙錨:模型 × rustc)**:`test_law_semantic_place_orthogonality`
(O-8:正交字段放行 ×2 + 同 place 必拒 ×2)、`test_law_semantic_cfg_liveness`
(O-9:(a) 分支不相交放行、(b) 迴圈回邊雙軌必拒、(c) 迴圈後借用放行)。

**全矩陣(2026-09-07,rustc 1.98.1)**:核心 32 單測 + 32 律(18.9s)+
O 系 9/9;oracle gate 41/41(parity 分歧 4 = 註冊表精確相等,BUG 0,
stale 0);clippy 0 警;rustdoc 淨。

## 九、下一步(P4-4 起,見 PIVOT-RUSTC-ORACLE §八)

- **P4-4**:Tier-B MIR borrowck 對照(rustc-private;nightly only);
  雙軌 vs rustc 區域結論的第三錨;polonius 單獨跑。
- 剩餘邊界(如實):① 泛型/閉包/生命周期註記仍 out-of-scope 排除;
  ② 二階引用(`&&T`)型別面未細分(現按 Ref 處理);
  ③ 迴巢迴圈的回邊延伸只取最外 span(保守,允許過報);
  ④ F-G 家族(E0384/E0597)持續不覆蓋,註冊表保留。
