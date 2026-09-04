//! §7 診斷(diagnostics)—— 把 CL0 解析樹的誤差節點投射成 LSP 可用的
//! `textDocument/publishDiagnostics` 診斷(severity + range + message)。
//!
//! 對應 ROADMAP **P2 #10「最小 LSP(僅診斷)」**:現有 span 族 + ERROR 節點已足夠
//! 輸出診斷 —— 這裡提供「樹 → 診斷」的純函數(library 層,CLI 與 LSP 共用),
//! `textDocument/publishDiagnostics` 的 JSON 打包在 `lsp.rs`,CLI `--check` 在此消費。

use crate::ast;
use crate::parse::{Kind, Tree};

/// LSP `DiagnosticSeverity` 數值(恰為 LSP 規範):
/// LSP `DiagnosticSeverity` Error 值。
pub const SEV_ERROR: u8 = 1;
/// LSP `DiagnosticSeverity` Warning 值。
pub const SEV_WARNING: u8 = 2;

/// 供命令行 `--check` 的人類可讀標簽。
pub fn severity_label(s: u8) -> &'static str {
    match s {
        1 => "error",
        2 => "warning",
        3 => "info",
        4 => "hint",
        _ => "?",
    }
}

/// LSP `Position`(line 0-based;character 按字節計,CL0 為 ASCII 結構故等同於字符)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// 0-based 行號。
    pub line: u32,
    /// 該行內的位元組偏移。
    pub character: u32,
}

/// LSP `Range`(半開:[start, end))。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    /// 起點位置。
    pub start: Position,
    /// 終點位置。
    pub end: Position,
}

impl Range {
    /// 建構一個 range。
    pub fn new(start: Position, end: Position) -> Range {
        Range { start, end }
    }
}

/// 診斷(與 LSP `Diagnostic` 一對一;severity 用 LSP 數值)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// 診斷覆蓋範圍(基於 `tree.src` 的字節偏移)。EOF 處用零長 range。
    pub range: Range,
    /// LSP severity 數值。
    pub severity: u8,
    /// 人類可讀訊息。
    pub message: String,
    /// 診斷來源標簽(輸出成 `source`)。
    pub source: String,
}

/// 位元組偏移 → `Position`(line 以 `\n` 計;character 為該行內的位元組偏移)。
/// 用於把 span 的位元組區間映射成 LSP 行/列。
pub fn offset_to_position(src: &str, byte: u32) -> Position {
    let mut line = 0u32;
    let mut line_start = 0u32;
    for (i, ch) in src.char_indices() {
        if (i as u32) >= byte {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = i as u32 + 1;
        }
    }
    Position {
        line,
        character: byte - line_start,
    }
}

/// 聲明一個錯誤節點的診斷訊息(Kind → 人類可讀短語)。
fn describe(kind: Kind) -> &'static str {
    match kind {
        Kind::Error => "syntax error",
        Kind::BadTok => "illegal byte",
        _ => "parse issue",
    }
}

/// 把一棵已解析樹的**誤差節點**(`Error` / `BadTok`)投射成診斷列表。
/// 這是 P2 #10 的主體:ERROR 節點 → `severity = Error` 的診斷。
pub fn diagnostics(tree: &Tree) -> Vec<Diagnostic> {
    let src = tree.src.as_str();
    let mut out = Vec::new();
    for n in &tree.nodes {
        let d = match n.kind {
            Kind::Error | Kind::BadTok => {
                let start = offset_to_position(src, n.span.start);
                let end = offset_to_position(src, n.span.end);
                Diagnostic {
                    range: Range::new(start, end),
                    severity: SEV_ERROR,
                    message: if n.kind == Kind::BadTok {
                        format!(
                            "illegal byte ({}): {:?}",
                            describe(n.kind),
                            &src[n.span.start as usize..n.span.end.min(src.len() as u32) as usize]
                        )
                    } else {
                        format!("{} at {}", describe(n.kind), n.span)
                    },
                    source: String::from("cl0r0"),
                }
            }
            _ => continue,
        };
        out.push(d);
    }
    out
}

/// 語義診斷(進一步的價值,把 liveness 衝突圖的「紅邊」標成 `Warning`)。
/// 依賴 `ast`(§3.2 / §3.3);若語料不含語法樹可解出的事實則為空。
/// 回傳的診斷以「僭越推導」的衝突來呈現 —— 即三軌 liveness 的紅邊。
pub fn semantic_diagnostics(tree: &Tree) -> Vec<Diagnostic> {
    use crate::span::Span;

    let facts = ast::extract(tree);
    let nbind = facts.bindings.len();
    let nevt = facts.events.len();
    if nevt == 0 || nbind == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    // 對三軌都掃描紅邊,以「重複出現」近似去重(同一衝突可能跨軌重用)。
    let mut seen: Vec<(u32, u32)> = Vec::new();
    for track in [ast::Track::Lexical, ast::Track::Nll, ast::Track::Referent] {
        for e in ast::red_edges(&facts, track) {
            let a = e.a as u32;
            let b = e.b as u32;
            let key = if a <= b { (a, b) } else { (b, a) };
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            // 衝突是兩個事件 e.a, e.b 的 span;用其綁定名描述。
            let eva = &facts.events[e.a];
            let evb = &facts.events[e.b];
            let name_a = facts.bindings[eva.binding].name.clone();
            let name_b = facts.bindings[evb.binding].name.clone();
            let sp = Span::new(
                eva.span.start.min(evb.span.start),
                eva.span.end.max(evb.span.end),
            );
            let start = offset_to_position(tree.src.as_str(), sp.start);
            let end = offset_to_position(tree.src.as_str(), sp.end);
            let k_a = kind_label(eva.kind);
            let k_b = kind_label(evb.kind);
            out.push(Diagnostic {
                range: Range::new(start, end),
                severity: SEV_WARNING,
                message: format!("borrow conflict on `{name_a}` ({k_a}) vs `{name_b}` ({k_b})"),
                source: String::from("cl0r0/semantic"),
            });
        }
    }
    out
}

/// 把 ast 事件種類簡化為人類可讀借用途徑。
fn kind_label(k: ast::EvKind) -> &'static str {
    match k {
        ast::EvKind::Decl => "decl",
        ast::EvKind::Read => "read",
        ast::EvKind::Move => "move",
        ast::EvKind::BorrowSh => "&",
        ast::EvKind::BorrowMut => "&mut",
        ast::EvKind::Deref => "deref",
    }
}
