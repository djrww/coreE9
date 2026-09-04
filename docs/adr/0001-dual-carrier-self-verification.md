# ADR-0001:雙載體自我驗證架構(Dual-carrier self-verification)

- **狀態**:已接受(Accepted,2026-09-05)
- **決策者**:CL0/R0(cl0r0)維護者
- **日期**:2026-09-05
- **相關文件**:`ROADMAP.md` §7、`docs/ROCQ-PLAN.md`、`docs/R3-NEXT-STEP.md`、`docs/ROCQ-TRACE.md`、`docs/ARCHITECTURE.md`

---

## 背景(Context)

cl0r0 的核心主張是「九條定律(L1–L9)為機械自証」。但單一載體(純 Rust 可執行層)只能**測**、不能**証**;
純定理層(Rocq)只能**証**、難以**執行/對照真實語義**。若兩者脫節,「理論正確」不代表「實現正確」。

歷史教訓(見 `docs/ROCQ-TRACE.md`):若不把語義寫成**可執行 + 可證件**雙份,並逐點對照,
則容易出現「捷徑為假」——例如本以為「修剪只改 `iend` ⇒ 他人 `cut_for` 不變」,經實測與對帳**為假**
(修剪會把被剪者自身從他人候選集移除)。
因此需要一個**跨載體的對帳機制**,把**實現**與**定理**釘死在同一個語義上。

## 決策(Decision)

採用**雙載體 + 交叉對帳**:

1. **載體一(可執行)**:Rust crate `cl0r0`,語義以可執行、可測試、可基準的形式存在。
2. **載體二(可證件)**:Rocq(Coq)在 `rocq/theories/` 提供同一語義的**鏡像**,以定理形式存在,
   且每條定理經 `Print Assumptions` **無 Axiom / Admitted**。
3. **對帳(耦合點)**:`tools/rocq_reconcile.py` 產出「Rust 執行期事實」(原名具事實,現 **19 項**,
   含 `appli(ct,g)`、`count(3,3)`、`measure`、`red_edges`、`ct_step_measure` …),
   並生成 `rocq/theories/reconcile_gen.v`,由 Rocq `vm_compute` **重算複驗 `=`**。
   這把兩載體綁到字面一致的語義。
4. **閘門(驗收契約)**:`.github/workflows/ci.yml` 四道閘(fmt/clippy/test/doc、rocq build+reconcile、
   coverage≥90%、bench ±25%)是**唯一**驗收管道。

## 後果(Consequences)

**正面**
- 「理論對」與「實現對」由同一語義綁定;任何語義更動都會在 reconcile 被逐點拆穿。
- 定理層可獨立地向上組裝(Newman、WCR、SN、合流),不必遷就可執行層的細節。
- 外部自証(external cross-check,Maude/NaTT)與鏡像互為**第三方**佐證。

**反面 / 成本**
- 需**雙份語義**,重複維護的共晶;語義改動要同步兩處。
- reconcile 依賴 Rust 執行期事實與 Rocq `vm_compute` 的一致性,對工具版本敏感。
- 新定理須在 `rocq/theories/` 有鏡像與 `make reconcile` 通過,增加產出成本。

**替代方案(未採用)**
- `rocq-of-rust`(把 Rust 完整翻譯成 Coq)未採,理由見 `ROCQ-PLAN §4.2`(成本/可擴充性不划算)。
- 純測試不設證件層:被拒(無法支撐「定律為**真**」,只能「為綠」)。

## 覆核(Review)

- 2026-09-05:R3Guard 落地後,`make -C rocq`(6 理論)+ `make reconcile`(19 點)全綠;
  `cargo test --all`(33 lib + 集成)全綠。此 ADR 與現況一致。
