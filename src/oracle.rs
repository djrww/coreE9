//! oracle —— Tier-A 行為權威通道(P4-0;docs/PIVOT-RUSTC-ORACLE.md §四)。
//!
//! 以 rustc 本體為權威指標:同一份源碼,問 rustc 要**判決**
//! `Accept | Reject{code, span}`,作為語義/語法層開發的行為標尺。
//!
//! 紀律(與九律一脈相承,裁判換人、紀律不換):
//!   * 比對粒度 = **錯誤碼集合 + 涉事 span**,不是診斷文本 —— 文本逐版漂移,
//!     不是合同;`RustcError` 因此刻意不含 message/rendered;
//!   * span 是字節半開區間,與 `span.rs`(§1.2 位址映象)同一代數;
//!   * 判定權不轉移:本通道只報判決;BUG / MODEL-DIFF / RUSTC-BUG 的歸因
//!     在 `docs/ORACLE-TRACE.md` 人工入表(§二分歧三分法);
//!   * core 零依賴不破壞:本模組 `feature = "oracle"` 隔離,發佈面不變;
//!     JSON 解析用自帶迷你解析器(與手寫 lexer/parser 同哲學)。
//!
//! 判決抽取規則(實測 2026-09-06,rustc 1.98.1;見 docs/ORACLE-TRACE.md):
//!   * JSON 診斷流走 **stderr**(rustc 常規:診斷走 stderr,stdout 留給編譯
//!     產物)—— 逐行解析 `--error-format=json` 的 stderr;僅取
//!     `level ∈ {error, fatal}`;warning/note/help/failure-note 一律不入判決;
//!   * 「aborting due to …」摘要行同為 error 級,但 `code = null` 且
//!     `spans = []` ⇒ 以「無碼且無 span」結構化排除(不是文本匹配);
//!   * 錯誤碼取 `code.code`(如 "E0502");無碼(語法錯誤類)記 `None`,
//!     `codes()` 以 `"uncoded"` 顯示;
//!   * span 優先取 `byte_start/byte_end`(字節精確);缺失時退化
//!     line/col → 字節換算 —— 後者僅對 ASCII 語料與 rustc 內部偏移嚴格一致
//!     (rustc 的 column 語義按字符計),非 ASCII 案例待 P4-1 驗證後才入語料;
//!   * 退出碼契約:0 = Accept;非 0 且有可解析 error 級診斷 = Reject;
//!     非 0 且無可解析診斷(如 ICE)= `OracleError::RustcCrash`,不算判決。

use crate::span::Span;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

// ===========================================================================
// 判決(Verdict)—— oracle 的輸出合同
// ===========================================================================

/// rustc 判決:Accept(編譯通過)或 Reject(附錯誤清單)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// rustc 接受(`--emit=metadata` 成功;等價於「無 error 級診斷」)。
    Accept,
    /// rustc 拒絕;`errors` 為全部 error 級診斷的結構化投影
    /// (已按上述規則排除「aborting due to …」摘要行)。
    Reject {
        /// 錯誤清單(順序即 rustc 給出順序,判定論下恆定)。
        errors: Vec<RustcError>,
    },
}

impl Verdict {
    /// 是否為接受。
    pub fn is_accept(&self) -> bool {
        matches!(self, Verdict::Accept)
    }

    /// 錯誤碼清單(排序去重;無碼診斷以 `"uncoded"` 代表)。
    pub fn codes(&self) -> Vec<String> {
        match self {
            Verdict::Accept => Vec::new(),
            Verdict::Reject { errors } => {
                let mut cs: Vec<String> = errors
                    .iter()
                    .map(|e| e.code.clone().unwrap_or_else(|| "uncoded".to_string()))
                    .collect();
                cs.sort();
                cs.dedup();
                cs
            }
        }
    }

    /// 是否含指定錯誤碼(不分大小寫)。
    pub fn has_code(&self, want: &str) -> bool {
        match self {
            Verdict::Accept => false,
            Verdict::Reject { errors } => errors.iter().any(|e| {
                e.code
                    .as_deref()
                    .is_some_and(|c| c.eq_ignore_ascii_case(want))
            }),
        }
    }

    /// 全部主 span(每條錯誤取 is_primary 的那個;無則首個)。
    pub fn primary_spans(&self) -> Vec<Span> {
        match self {
            Verdict::Accept => Vec::new(),
            Verdict::Reject { errors } => errors.iter().filter_map(|e| e.span).collect(),
        }
    }
}

/// 一條 rustc 錯誤級診斷的結構化投影:錯誤碼 + 主 span。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RustcError {
    /// 錯誤碼(如 `"E0502"`;被 deny 的 lint 亦可能,如 `"unused_variables"`)。
    /// `None` = 無碼診斷(如語法錯誤)。
    pub code: Option<String>,
    /// 主 span(字節半開區間,§1.2 同一代數);`None` = rustc 未給出可用 span。
    pub span: Option<Span>,
}

/// 一次 oracle 查詢的完整報告:判決 + 版本見證。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OracleReport {
    /// rustc 判決。
    pub verdict: Verdict,
    /// `rustc --version` 輸出(基線對帳的版本見證;P4-1 版本矩陣的錨點)。
    pub rustc_version: String,
}

// ===========================================================================
// 錯誤(OracleError)—— 環境/工具鏈問題,與「判決」嚴格分離
// ===========================================================================

/// oracle 面錯誤:不是判決,是「問不到判決」。
#[derive(Clone, Debug)]
pub enum OracleError {
    /// PATH 找不到 rustc(安裝工具鏈,或以 `CL0R0_ORACLE_RUSTC` 指定路徑)。
    RustcNotFound(String),
    /// IO 失敗(暫存檔寫入/子進程執行)。
    Io(String),
    /// rustc 輸出非 UTF-8。
    NonUtf8Output,
    /// JSON 解析失敗(`--error-format=json` 的輸出理應逐行合法)。
    JsonParse(String),
    /// rustc 異常退出且無可解析判決(如 ICE)。
    RustcCrash(String),
}

impl std::fmt::Display for OracleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OracleError::RustcNotFound(p) => write!(
                f,
                "找不到 rustc({p});安裝工具鏈或設 CL0R0_ORACLE_RUSTC 指定路徑"
            ),
            OracleError::Io(m) => write!(f, "IO 失敗:{m}"),
            OracleError::NonUtf8Output => write!(f, "rustc 輸出非 UTF-8"),
            OracleError::JsonParse(m) => write!(f, "JSON 解析失敗:{m}"),
            OracleError::RustcCrash(m) => write!(f, "rustc 異常(無可解析判決):{m}"),
        }
    }
}

impl std::error::Error for OracleError {}

// ===========================================================================
// Oracle 接口 + Tier-A 實作(CliOracle)
// ===========================================================================

/// 行為權威接口:P4 系列一切門檻建立在這層之上(語料對帳/parity/版本矩陣)。
pub trait Oracle {
    /// 實作名(報告與對帳表的標識;如 "cli-rustc")。
    fn name(&self) -> &'static str;
    /// 對單份源碼給出判決。
    fn check(&self, src: &str) -> Result<OracleReport, OracleError>;
}

/// Tier-A oracle:以 `rustc` 命令行為裁判(stable 即可,無 rustc_private、無 nightly)。
///
/// 判決查詢走 `rustc --edition=2021 --crate-type=lib --emit=metadata
/// --error-format=json <暫存檔>`;暫存檔寫在系統 temp 目錄,用後即刪。
#[derive(Clone, Debug)]
pub struct CliOracle {
    rustc: String,
    edition: String,
}

impl CliOracle {
    /// 默認實例:edition 2021;rustc 路徑取環境變量 `CL0R0_ORACLE_RUSTC`,
    /// 缺省 `"rustc"`(版本矩陣 P4-1 以不同路徑掛多個 stable/nightly)。
    pub fn new() -> Self {
        Self {
            rustc: std::env::var("CL0R0_ORACLE_RUSTC").unwrap_or_else(|_| "rustc".to_string()),
            edition: "2021".to_string(),
        }
    }

    /// 目前使用的 rustc 路徑。
    pub fn rustc_path(&self) -> &str {
        &self.rustc
    }

    /// `rustc --version`(單行;基線的版本見證)。
    pub fn toolchain_version(&self) -> Result<String, OracleError> {
        let out = Command::new(&self.rustc)
            .arg("--version")
            .output()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => OracleError::RustcNotFound(self.rustc.clone()),
                _ => OracleError::Io(format!("spawn {}: {e}", self.rustc)),
            })?;
        let s = String::from_utf8(out.stdout).map_err(|_| OracleError::NonUtf8Output)?;
        Ok(s.trim().to_string())
    }

    fn temp_src_path() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("cl0r0-oracle-{}-{n}.rs", std::process::id()))
    }

    /// 跑一趟 rustc;回傳 (退出碼, 診斷流=stderr, 其餘=stdout)。
    fn run_rustc(&self, src: &str) -> Result<(Option<i32>, String, String), OracleError> {
        let path = Self::temp_src_path();
        std::fs::write(&path, src)
            .map_err(|e| OracleError::Io(format!("write {}: {e}", path.display())))?;
        let out = Command::new(&self.rustc)
            .args([
                "--edition",
                self.edition.as_str(),
                "--crate-type=lib",
                "--emit=metadata",
                // 產物導回 temp:否則 --emit=metadata 的 .rmeta 會落地 CWD
                // 形成垃圾(2026-09-06 實測教訓,見 ORACLE-TRACE F9)。
                "--out-dir",
            ])
            .arg(std::env::temp_dir())
            .arg("--error-format=json")
            .arg(&path)
            .output()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => OracleError::RustcNotFound(self.rustc.clone()),
                _ => OracleError::Io(format!("spawn {}: {e}", self.rustc)),
            })?;
        // 暫存檔用後即刪(刪除失敗不升級為錯誤:temp 目錄本就會被系統清理)。
        let _ = std::fs::remove_file(&path);
        let stdout = String::from_utf8(out.stdout).map_err(|_| OracleError::NonUtf8Output)?;
        let stderr = String::from_utf8(out.stderr).map_err(|_| OracleError::NonUtf8Output)?;
        // 診斷流(JSON)走 stderr;stdout 幾乎恆空,僅作 crash 訊息附件。
        Ok((out.status.code(), stderr, stdout))
    }
}

impl Default for CliOracle {
    fn default() -> Self {
        Self::new()
    }
}

impl Oracle for CliOracle {
    fn name(&self) -> &'static str {
        "cli-rustc"
    }

    fn check(&self, src: &str) -> Result<OracleReport, OracleError> {
        let rustc_version = self.toolchain_version()?;
        let (code, diagnostics, other_out) = self.run_rustc(src)?;
        let mut errors = Vec::new();
        for line in diagnostics.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v = mini_json::parse(line).map_err(OracleError::JsonParse)?;
            let level = v
                .get("level")
                .and_then(mini_json::Json::as_str)
                .unwrap_or("");
            if level != "error" && level != "fatal" {
                continue; // warning / note / help / failure-note 不入判決
            }
            let code_f = v.get("code").and_then(|c| match c {
                mini_json::Json::Null => None,
                mini_json::Json::Obj(_) => c
                    .get("code")
                    .and_then(mini_json::Json::as_str)
                    .map(str::to_owned),
                _ => None,
            });
            let span = extract_primary_span(v.get("spans").and_then(mini_json::Json::as_arr), src);
            // 摘要行(「aborting due to …」)= 無碼且無 span ⇒ 結構化排除,不入判決。
            if code_f.is_none() && span.is_none() {
                continue;
            }
            errors.push(RustcError { code: code_f, span });
        }
        if !errors.is_empty() {
            return Ok(OracleReport {
                verdict: Verdict::Reject { errors },
                rustc_version,
            });
        }
        match code {
            Some(0) => Ok(OracleReport {
                verdict: Verdict::Accept,
                rustc_version,
            }),
            other => Err(OracleError::RustcCrash(format!(
                "exit={other:?};輸出尾部:{}",
                tail(&diagnostics, &other_out, 300)
            ))),
        }
    }
}

// ===========================================================================
// span 抽取:byte 優先,line/col 退化
// ===========================================================================

/// 從診斷 `spans` 陣列取主 span:優先 `byte_start/byte_end`(字節精確);
/// 缺失時退化 line/col → 字節換算(僅 ASCII 語料嚴格一致,見模組文檔)。
fn extract_primary_span(spans: Option<&[mini_json::Json]>, src: &str) -> Option<Span> {
    let arr = spans?;
    let prim = arr
        .iter()
        .find(|s| s.get("is_primary").and_then(mini_json::Json::as_bool) == Some(true))
        .or_else(|| arr.first())?;
    if let (Some(a), Some(b)) = (json_u32(prim, "byte_start"), json_u32(prim, "byte_end")) {
        if a <= b && (b as usize) <= src.len() {
            return Some(Span::new(a, b));
        }
    }
    let table = line_start_table(src);
    let s = lc_to_byte(
        &table,
        json_u32(prim, "line_start")?,
        json_u32(prim, "column_start")?,
    )?;
    let e = lc_to_byte(
        &table,
        json_u32(prim, "line_end")?,
        json_u32(prim, "column_end")?,
    )?;
    if s <= e && (e as usize) <= src.len() {
        Some(Span::new(s, e))
    } else {
        None
    }
}

/// crash 診斷用:兩條輸出流串接後的尾部 n 字符(僅供人讀,不入判決)。
fn tail(a: &str, b: &str, n: usize) -> String {
    let joined = format!("{a}\n{b}");
    joined
        .chars()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// 物件取整數值欄位(f64 中轉;rustc 診斷數字皆遠小於 2^53 的整數)。
fn json_u32(o: &mini_json::Json, key: &str) -> Option<u32> {
    let f = o.get(key)?.as_f64()?;
    if f.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(&f) {
        Some(f as u32)
    } else {
        None
    }
}

/// 行起點字節表:`table[i]` = 第 i+1 行的首字節偏移。
fn line_start_table(src: &str) -> Vec<u32> {
    let mut t = vec![0u32];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            t.push(i as u32 + 1);
        }
    }
    t
}

/// (line, column)(皆 1 基)→ 字節偏移。column 按字節計 —— 僅 ASCII 語料與
/// rustc 內部偏移嚴格一致(rustc 的 column 對非 ASCII 按字符計)。
fn lc_to_byte(table: &[u32], line: u32, col: u32) -> Option<u32> {
    let base = table.get(line.checked_sub(1)? as usize)?;
    base.checked_add(col.checked_sub(1)?)
}

// ===========================================================================
// 語料期望(Expectation)—— 檔名即期望,零配置
// ===========================================================================

/// 語料案例的期望判決(由檔名前綴推導;見 `bin/oracle` 文檔的約定)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Expectation {
    /// 期望 rustc 接受。
    Accept,
    /// 期望 rustc 拒絕且**含**該錯誤碼(不分大小寫;「含」語義,容許伴生碼)。
    RejectCode(String),
    /// 期望 rustc 拒絕且含無碼診斷(語法錯誤類)。
    RejectUncoded,
}

impl Expectation {
    /// 檔名 stem → 期望。約定:`accept_*` / `e<四位碼>_*` / `uncoded_*`;
    /// 無法推導 → `None`(調用方應視為配置錯誤,直接紅)。
    pub fn from_stem(stem: &str) -> Option<Expectation> {
        let (head, _) = stem.split_once('_')?;
        if head == "accept" {
            Some(Expectation::Accept)
        } else if head == "uncoded" {
            Some(Expectation::RejectUncoded)
        } else if head.len() == 5
            && head.starts_with('e')
            && head[1..].bytes().all(|b| b.is_ascii_digit())
        {
            Some(Expectation::RejectCode(head.to_ascii_uppercase()))
        } else {
            None
        }
    }

    /// 期望的顯示標籤(基線記錄用;如 "accept" / "e0502" / "uncoded")。
    pub fn label(&self) -> String {
        match self {
            Expectation::Accept => "accept".to_string(),
            Expectation::RejectCode(c) => c.to_ascii_lowercase(),
            Expectation::RejectUncoded => "uncoded".to_string(),
        }
    }

    /// 判決是否符合期望。
    pub fn matches(&self, v: &Verdict) -> bool {
        match self {
            Expectation::Accept => v.is_accept(),
            Expectation::RejectCode(c) => v.has_code(c),
            Expectation::RejectUncoded => {
                !v.is_accept() && v.codes().iter().any(|c| c == "uncoded")
            }
        }
    }
}

// ===========================================================================
// 迷你 JSON 解析器(oracle 自用;零依賴哲學,非通用庫)
// ===========================================================================

pub(crate) mod mini_json {
    //! JSON 全語法解析(Null/Bool/Num/Str/Arr/Obj;轉義含 `\uXXXX` 與代理對)。
    //! 數值以 f64 承載 —— rustc 診斷裡的數字(行/列/字節偏移)皆遠小於 2^53
    //! 的整數,f64 表示精確。落單代理以 U+FFFD 如實降級(不靜默、不 panic)。

    /// 解析結果值樹(物件保持鍵序;rustc 輸出無重複鍵,首鍵匹配即取)。
    #[derive(Clone, Debug, PartialEq)]
    pub(crate) enum Json {
        /// null
        Null,
        /// true / false
        Bool(bool),
        /// 數值(f64)
        Num(f64),
        /// 字串(已解轉義)
        Str(String),
        /// 陣列
        Arr(Vec<Json>),
        /// 物件(鍵值對有序)
        Obj(Vec<(String, Json)>),
    }

    impl Json {
        /// 取鍵(首個匹配;非物件 → None)。
        pub(crate) fn get(&self, key: &str) -> Option<&Json> {
            match self {
                Json::Obj(kvs) => kvs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
                _ => None,
            }
        }
        /// 作字串取值。
        pub(crate) fn as_str(&self) -> Option<&str> {
            match self {
                Json::Str(s) => Some(s),
                _ => None,
            }
        }
        ///作數值取值。
        pub(crate) fn as_f64(&self) -> Option<f64> {
            match self {
                Json::Num(n) => Some(*n),
                _ => None,
            }
        }
        /// 作布林取值。
        pub(crate) fn as_bool(&self) -> Option<bool> {
            match self {
                Json::Bool(b) => Some(*b),
                _ => None,
            }
        }
        /// 作陣列取值。
        pub(crate) fn as_arr(&self) -> Option<&[Json]> {
            match self {
                Json::Arr(xs) => Some(xs),
                _ => None,
            }
        }
    }

    /// 解析單個 JSON 文檔(不允許尾隨內容)。
    pub(crate) fn parse(text: &str) -> Result<Json, String> {
        let mut p = P {
            b: text.as_bytes(),
            i: 0,
        };
        p.ws();
        let v = p.value()?;
        p.ws();
        if p.i != p.b.len() {
            return Err(format!("JSON 尾隨內容 @ byte {}", p.i));
        }
        Ok(v)
    }

    struct P<'a> {
        b: &'a [u8],
        i: usize,
    }

    impl<'a> P<'a> {
        fn ws(&mut self) {
            while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            }
        }

        fn peek(&self) -> Option<u8> {
            self.b.get(self.i).copied()
        }

        fn eat(&mut self, c: u8) -> bool {
            if self.peek() == Some(c) {
                self.i += 1;
                true
            } else {
                false
            }
        }

        fn lit(&mut self, s: &[u8]) -> bool {
            if self.b.len() >= self.i + s.len() && &self.b[self.i..self.i + s.len()] == s {
                self.i += s.len();
                true
            } else {
                false
            }
        }

        fn err(&self, what: &str) -> String {
            format!(
                "JSON 解析失敗 @ byte {}:期望 {what},實得 {:?}",
                self.i,
                self.peek().map(char::from)
            )
        }

        fn value(&mut self) -> Result<Json, String> {
            match self.peek() {
                Some(b'n') if self.lit(b"null") => Ok(Json::Null),
                Some(b't') if self.lit(b"true") => Ok(Json::Bool(true)),
                Some(b'f') if self.lit(b"false") => Ok(Json::Bool(false)),
                Some(b'"') => Ok(Json::Str(self.string()?)),
                Some(b'[') => self.array(),
                Some(b'{') => self.object(),
                Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
                _ => Err(self.err("值(null/true/false/\"…\"/[…]/{…}/數值)")),
            }
        }

        fn string(&mut self) -> Result<String, String> {
            if !self.eat(b'"') {
                return Err(self.err("\""));
            }
            let mut out: Vec<u8> = Vec::new();
            loop {
                let Some(c) = self.peek() else {
                    return Err("JSON 字串未閉合".to_string());
                };
                self.i += 1;
                match c {
                    b'"' => break,
                    b'\\' => {
                        let Some(e) = self.peek() else {
                            return Err("JSON 轉義未閉合".to_string());
                        };
                        self.i += 1;
                        match e {
                            b'"' => out.push(b'"'),
                            b'\\' => out.push(b'\\'),
                            b'/' => out.push(b'/'),
                            b'b' => out.push(0x08),
                            b'f' => out.push(0x0C),
                            b'n' => out.push(b'\n'),
                            b'r' => out.push(b'\r'),
                            b't' => out.push(b'\t'),
                            b'u' => {
                                let cp = self.hex4()?;
                                self.push_cp(cp, &mut out);
                            }
                            _ => return Err(format!("非法轉義 \\{}", char::from(e))),
                        }
                    }
                    // 原始字節直通:輸入本是合法 &str,非 ASCII 直通即合法 UTF-8。
                    _ => out.push(c),
                }
            }
            String::from_utf8(out).map_err(|_| "JSON 字節序列非法 UTF-8".to_string())
        }

        fn hex4(&mut self) -> Result<u32, String> {
            let mut v = 0u32;
            for _ in 0..4 {
                let Some(c) = self.peek() else {
                    return Err("\\u 轉義不足 4 位十六進制".to_string());
                };
                let d = (c as char)
                    .to_digit(16)
                    .ok_or_else(|| format!("非法十六進制 `{}`", char::from(c)))?;
                self.i += 1;
                v = v * 16 + d;
            }
            Ok(v)
        }

        /// 碼點入列:代理對合併;落單代理 → U+FFFD(如實降級)。
        fn push_cp(&mut self, cp: u32, out: &mut Vec<u8>) {
            let mut buf4 = [0u8; 4];
            let mut buf3 = [0u8; 3];
            if (0xD800..=0xDBFF).contains(&cp) {
                // 高代理:偷看後隨 `\uXXXX` 是否為合法低代理;不是則回退不動。
                if self.b.get(self.i) == Some(&b'\\') && self.b.get(self.i + 1) == Some(&b'u') {
                    let save = self.i;
                    self.i += 2;
                    match self.hex4() {
                        Ok(lo) if (0xDC00..=0xDFFF).contains(&lo) => {
                            let c = char::from_u32(0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00))
                                .unwrap_or('\u{FFFD}');
                            out.extend_from_slice(c.encode_utf8(&mut buf4).as_bytes());
                            return;
                        }
                        _ => self.i = save,
                    }
                }
                out.extend_from_slice('\u{FFFD}'.encode_utf8(&mut buf3).as_bytes());
            } else if (0xDC00..=0xDFFF).contains(&cp) {
                out.extend_from_slice('\u{FFFD}'.encode_utf8(&mut buf3).as_bytes());
            } else {
                let c = char::from_u32(cp).unwrap_or('\u{FFFD}');
                out.extend_from_slice(c.encode_utf8(&mut buf4).as_bytes());
            }
        }

        fn number(&mut self) -> Result<Json, String> {
            let start = self.i;
            if self.peek() == Some(b'-') {
                self.i += 1;
            }
            while matches!(
                self.peek(),
                Some(c) if c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'+' | b'-')
            ) {
                self.i += 1;
            }
            let s = std::str::from_utf8(&self.b[start..self.i])
                .map_err(|_| "數值切片非法 UTF-8".to_string())?;
            s.parse::<f64>()
                .map(Json::Num)
                .map_err(|e| format!("數值 `{s}` 無法解析:{e}"))
        }

        fn array(&mut self) -> Result<Json, String> {
            if !self.eat(b'[') {
                return Err(self.err("["));
            }
            let mut items = Vec::new();
            self.ws();
            if self.eat(b']') {
                return Ok(Json::Arr(items));
            }
            loop {
                self.ws();
                items.push(self.value()?);
                self.ws();
                if self.eat(b',') {
                    continue;
                }
                if self.eat(b']') {
                    break;
                }
                return Err(self.err(", 或 ]"));
            }
            Ok(Json::Arr(items))
        }

        fn object(&mut self) -> Result<Json, String> {
            if !self.eat(b'{') {
                return Err(self.err("{"));
            }
            let mut kvs = Vec::new();
            self.ws();
            if self.eat(b'}') {
                return Ok(Json::Obj(kvs));
            }
            loop {
                self.ws();
                let key = self.string()?;
                self.ws();
                if !self.eat(b':') {
                    return Err(self.err(":"));
                }
                self.ws();
                let val = self.value()?;
                kvs.push((key, val));
                self.ws();
                if self.eat(b',') {
                    continue;
                }
                if self.eat(b'}') {
                    break;
                }
                return Err(self.err(", 或 }"));
            }
            Ok(Json::Obj(kvs))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn mini_json_parses_rustc_diagnostic_shape() {
            let line = r#"{"$message_type":"diagnostic","message":"cannot borrow `x`","code":{"code":"E0502","explanation":"A\nB\tC"},"level":"error","spans":[{"file_name":"t.rs","byte_start":85,"byte_end":91,"line_start":5,"line_end":5,"column_start":7,"column_end":13,"is_primary":true,"label":"mutable borrow occurs here","suggested_replacement":null,"expansion":null}],"children":[],"rendered":"error[E0502]: …\n"}"#;
            let v = parse(line).expect("rustc 診斷行應可解析");
            assert_eq!(v.get("level").and_then(Json::as_str), Some("error"));
            let code = v.get("code").expect("code 鍵");
            assert_eq!(code.get("code").and_then(Json::as_str), Some("E0502"));
            let spans = v.get("spans").and_then(Json::as_arr).expect("spans 陣列");
            let prim = spans.first().expect("首 span");
            assert_eq!(prim.get("is_primary").and_then(Json::as_bool), Some(true));
            assert_eq!(prim.get("byte_start").and_then(Json::as_f64), Some(85.0));
            assert_eq!(
                v.get("children").and_then(Json::as_arr).map(<[Json]>::len),
                Some(0)
            );
        }

        #[test]
        fn mini_json_escapes_and_surrogates() {
            let v = parse(r#""Aé😀""#).expect("轉義解析");
            assert_eq!(v.as_str(), Some("Aé😀"));
            // 落單高代理 → U+FFFD(如實降級,不 panic)
            let lone = parse(r#""\uD83D""#).expect("落單代理解析");
            assert_eq!(lone.as_str(), Some("\u{FFFD}"));
            let simple = parse(r#""\n\t\\\"\/\b\f\r""#).expect("基礎轉義");
            assert_eq!(simple.as_str(), Some("\n\t\\\"/\u{8}\u{C}\r"));
        }

        #[test]
        fn mini_json_nesting_and_literals() {
            let v = parse(r#"{"a":[1,-2.5,true,false,null,{"b":[]}]}"#).expect("嵌套解析");
            let a = v.get("a").and_then(Json::as_arr).expect("a 陣列");
            assert_eq!(a[0].as_f64(), Some(1.0));
            assert_eq!(a[1].as_f64(), Some(-2.5));
            assert_eq!(a[2].as_bool(), Some(true));
            assert_eq!(a[4], Json::Null);
            assert_eq!(
                a[5].get("b").and_then(Json::as_arr).map(<[Json]>::len),
                Some(0)
            );
        }

        #[test]
        fn mini_json_rejects_garbage() {
            assert!(parse("").is_err());
            assert!(parse("{").is_err());
            assert!(parse("[1,]").is_err());
            assert!(parse("nul").is_err());
            assert!(parse("\"\\q\"").is_err());
            assert!(parse("1 2").err().unwrap().contains("尾隨"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expectation_from_stem_grammar() {
        assert_eq!(
            Expectation::from_stem("accept_clean"),
            Some(Expectation::Accept)
        );
        assert_eq!(
            Expectation::from_stem("e0502_classic"),
            Some(Expectation::RejectCode("E0502".to_string()))
        );
        assert_eq!(
            Expectation::from_stem("uncoded_syntax"),
            Some(Expectation::RejectUncoded)
        );
        // 無 '_' 分隔 / 碼非四位 / 未知前綴 → None(配置錯誤,調用方應紅)
        assert_eq!(Expectation::from_stem("accept"), None);
        assert_eq!(Expectation::from_stem("e50_x"), None);
        assert_eq!(Expectation::from_stem("weird_name"), None);
    }

    #[test]
    fn expectation_matches_semantics() {
        let reject = Verdict::Reject {
            errors: vec![
                RustcError {
                    code: Some("E0499".to_string()),
                    span: None,
                },
                RustcError {
                    code: Some("E0502".to_string()),
                    span: None,
                },
            ],
        };
        // 「含」語義:RejectCode 只要求包含該碼(容許伴生碼)
        assert!(Expectation::RejectCode("E0502".to_string()).matches(&reject));
        assert!(Expectation::RejectCode("e0502".to_string()).matches(&reject));
        assert!(!Expectation::RejectCode("E0384".to_string()).matches(&reject));
        assert!(!Expectation::Accept.matches(&reject));
        let uncoded = Verdict::Reject {
            errors: vec![RustcError {
                code: None,
                span: Some(Span::new(0, 3)),
            }],
        };
        assert!(Expectation::RejectUncoded.matches(&uncoded));
        assert!(!Expectation::RejectUncoded.matches(&reject));
        // codes():排序去重 + uncoded 顯示
        assert_eq!(uncoded.codes(), vec!["uncoded".to_string()]);
        assert_eq!(
            reject.codes(),
            vec!["E0499".to_string(), "E0502".to_string()]
        );
    }

    #[test]
    fn line_col_fallback_table() {
        let src = "ab\ncd\nef\n";
        let t = line_start_table(src);
        assert_eq!(t, vec![0, 3, 6, 9]); // 三行 + EOF 行起點
        assert_eq!(lc_to_byte(&t, 1, 1), Some(0));
        assert_eq!(lc_to_byte(&t, 2, 2), Some(4)); // 'd'
        assert_eq!(lc_to_byte(&t, 3, 3), Some(8)); // 'f'
        assert_eq!(lc_to_byte(&t, 4, 1), Some(9)); // EOF 插入點
        assert_eq!(lc_to_byte(&t, 0, 1), None); // 行號 1 基,0 越界
        assert_eq!(lc_to_byte(&t, 1, 0), None); // 列號 1 基,0 越界
    }
}
