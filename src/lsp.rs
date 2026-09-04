//! 最小 LSP(僅診斷)伺服器 —— ROADMAP **P2 #10**。
//!
//! 使用現有的 span 族 + ERROR 節點(§2.3)輸出 `textDocument/publishDiagnostics`。
//! 零依賴:自帶極小 JSON / JSON-RPC(見 `json.rs`),以 **Content-Length 訊息框** 在
//! stdin/stdout 上通訊。支援的請求/通知:
//!   * `initialize`               → 回應 capabilities(`textDocumentSync = 1` Full)。
//!   * `initialized`              → 通知,忽略。
//!   * `shutdown` / `exit`        → 依 LSP 生命週期停止。
//!   * `textDocument/didOpen`     → 對該文檔解析並發布診斷。
//!   * `textDocument/didChange`   → 取最後一份全文重析並發布診斷。
//!   * `textDocument/didClose`    → 清空該文檔診斷。
//!
//! 診斷來源:`diag::diagnostics`(語法,ERROR/BadTok)+ `diag::semantic_diagnostics`
//! (liveness 紅邊 → Warning)。兩者皆可關閉(見 `run`)。

use crate::diag::{diagnostics, semantic_diagnostics, Diagnostic};
use crate::json::{self, Value};
use crate::parse::{parse, ParseIssue};
use std::io::{self, Read, Write};

/// LSP JSON-RPC 版本。
const RPC: &str = "2.0";

/// 讀取一則 Content-Length 訊框的 body。
fn read_frame(r: &mut impl Read) -> Option<Vec<u8>> {
    // 讀取 header(至空行)。
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if r.read(&mut byte).ok()? == 0 {
            return None; // EOF
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header = String::from_utf8_lossy(&header);
    let mut content_len = 0usize;
    for line in header.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_len = v.trim().parse().ok()?;
            }
        }
    }
    let mut body = vec![0u8; content_len];
    r.read_exact(&mut body).ok()?;
    Some(body)
}

/// 寫出一則訊框(JSON-RPC 訊息)。
fn write_frame(w: &mut impl Write, v: &Value) {
    let s = v.to_string();
    let _ = write!(w, "Content-Length: {}\r\n\r\n", s.len());
    let _ = w.write_all(s.as_bytes());
    let _ = w.flush();
}

fn response(id: Option<Value>, result: Value) -> Value {
    match id {
        Some(i) => json::object(vec![
            ("jsonrpc", Value::string(RPC)),
            ("id", i),
            ("result", result),
        ]),
        None => Value::Null,
    }
}

fn notify(method: &str, params: Value) -> Value {
    json::object(vec![
        ("jsonrpc", Value::string(RPC)),
        ("method", Value::string(method)),
        ("params", params),
    ])
}

/// 把診斷序列化為 LSP `Diagnostic` JSON。
fn diag_to_json(d: &Diagnostic) -> Value {
    let pos = |x: crate::diag::Position| {
        json::object(vec![
            ("line", Value::Number(x.line as f64)),
            ("character", Value::Number(x.character as f64)),
        ])
    };
    json::object(vec![
        (
            "range",
            json::object(vec![
                ("start", pos(d.range.start)),
                ("end", pos(d.range.end)),
            ]),
        ),
        ("severity", Value::Number(d.severity as f64)),
        ("message", Value::string(d.message.clone())),
        ("source", Value::string(d.source.clone())),
    ])
}

fn publish(w: &mut impl Write, uri: &str, diags: &[Diagnostic]) {
    let arr = Value::Array(diags.iter().map(diag_to_json).collect());
    let params = json::object(vec![
        ("uri", Value::string(uri.to_string())),
        ("diagnostics", arr),
    ]);
    write_frame(w, &notify("textDocument/publishDiagnostics", params));
}

/// 解析文檔並生成診斷;解析器引擎深度極限(ParseIssue::Depth)時,以全文一份
/// ERROR 診斷如實申報(「誠實缺口」而非掩蓋)。
fn doc_diagnostics(text: &str) -> Vec<Diagnostic> {
    match parse(text) {
        Ok(t) => {
            let mut d = diagnostics(&t);
            d.extend(semantic_diagnostics(&t));
            d
        }
        Err(ParseIssue::Depth) | Err(ParseIssue::Syntax) => vec![Diagnostic {
            range: crate::diag::Range::new(
                crate::diag::Position {
                    line: 0,
                    character: 0,
                },
                crate::diag::offset_to_position(text, text.len() as u32),
            ),
            severity: crate::diag::SEV_ERROR,
            message: String::from("parser engine limit / parse failed"),
            source: String::from("cl0r0"),
        }],
    }
}

fn uri_of(params: &Value, doc: &str) -> Option<String> {
    // `doc` 即 "textDocument":`params.get(doc)` 已回傳該文字文件物件,再取其中的
    // `uri`。★ 歷史 bug:曾又對該物件 `.get("textDocument")`(二次取鍵,其內無此鍵)
    // ⇒ uri_of 恆失敗 ⇒ LSP 永不發佈診斷。此處只取一次。
    params
        .get(doc)
        .and_then(|td| td.get("uri"))
        .and_then(Value::as_str)
        .map(String::from)
}

fn handle_open(w: &mut impl Write, msg: &Value) {
    let params = msg.get("params");
    if let Some(params) = params {
        if let Some(uri) = uri_of(params, "textDocument") {
            let text = params
                .get("textDocument")
                .and_then(|td| td.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let d = doc_diagnostics(text);
            publish(w, &uri, &d);
        }
    }
}

fn handle_change(w: &mut impl Write, msg: &Value) {
    let params = msg.get("params");
    if let Some(params) = params {
        if let Some(uri) = uri_of(params, "textDocument") {
            // 全文同步(textDocumentSync = 1):contentChanges 末項的 text 即全文。
            let text = last_change_text(params);
            let d = doc_diagnostics(text);
            publish(w, &uri, &d);
        }
    }
}

/// 取 `contentChanges` 末項的 `text`。
fn last_change_text(params: &Value) -> &str {
    params
        .get("contentChanges")
        .and_then(|cc| match cc {
            Value::Array(a) => a.last(),
            _ => None,
        })
        .and_then(|c| c.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn handle_close(w: &mut impl Write, msg: &Value) {
    let params = msg.get("params");
    if let Some(params) = params {
        if let Some(uri) = uri_of(params, "textDocument") {
            publish(w, &uri, &[]);
        }
    }
}

/// 主迴圈:讀取 stdin 的 JSON-RPC,處理直到 `exit`。
pub fn run() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut r = stdin.lock();
    let mut w = stdout.lock();
    while let Some(body) = read_frame(&mut r) {
        let msg = match json::parse_bytes(&body) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[lsp] bad JSON-RPC frame: {e}");
                continue;
            }
        };
        let method = msg
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let id = msg.get("id").cloned();
        match method.as_str() {
            "initialize" => {
                let caps = json::object(vec![(
                    "capabilities",
                    json::object(vec![
                        ("textDocumentSync", Value::Number(1.0)),
                        ("diagnosticProvider", json::object(vec![])),
                    ]),
                )]);
                write_frame(&mut w, &response(id, caps));
            }
            "shutdown" => {
                write_frame(&mut w, &response(id, Value::Null));
            }
            "exit" => break,
            "textDocument/didOpen" => handle_open(&mut w, &msg),
            "textDocument/didChange" => handle_change(&mut w, &msg),
            "textDocument/didClose" => handle_close(&mut w, &msg),
            // `initialized` 等僅通知:無需回應。
            "initialized" => { /* ignore */ }
            _ => {
                // 未知請求:回應 null(對一個僅診斷的 demo 足夠)。
                if id.is_some() {
                    write_frame(&mut w, &response(id, Value::Null));
                }
            }
        }
    }
}
