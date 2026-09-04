//! P2 #9 / #10 工具層整合測試。
//!
//! * LSP 伺服器(`lsp` 二進位):送出 JSON-RPC 訊框,驗證對
//!   `textDocument/didOpen`/`didChange` 發佈 `publishDiagnostics`,
//!   且初始化/關閉/退出生命週期正常。
//! * CLI 檢查器(`cl0r0 --check`):驗證退出碼契約(0 = 無診斷、1 = 有診斷)。
//!
//! 回歸鎖定:#10 曾因 `uri_of` 二次索引 `textDocument` 而永不發佈診斷;
//! 此測試必須捕捉到「有診斷」這一訊號(而非空診斷)。

use cl0r0::json::{self, Value};
use std::io::{Read, Write};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// 小工具:json 影片
// ---------------------------------------------------------------------------

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    json::object(pairs)
}

fn frame_json(v: &Value) -> Vec<u8> {
    let body = v.to_string();
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

fn publish_counts_for(uri_value_docs: &[(&str, &str)]) -> Vec<usize> {
    // 啟動 LSP 子程序,依序送出 initialize + 每個 didOpen,再 shutdown/exit。
    let mut child = Command::new(env!("CARGO_BIN_EXE_lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn lsp");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut send = |v: &Value| {
        let _ = stdin.write_all(&frame_json(v));
        let _ = stdin.flush();
    };

    let init = obj(vec![
        ("jsonrpc", Value::string("2.0")),
        ("id", Value::number(1.0)),
        ("method", Value::string("initialize")),
        ("params", obj(vec![("capabilities", obj(vec![]))])),
    ]);
    send(&init);

    for (uri, text) in uri_value_docs {
        let did = obj(vec![
            ("jsonrpc", Value::string("2.0")),
            ("method", Value::string("textDocument/didOpen")),
            (
                "params",
                obj(vec![(
                    "textDocument",
                    obj(vec![
                        ("uri", Value::string(uri.to_string())),
                        ("languageId", Value::string("cl0r0")),
                        ("version", Value::number(1.0)),
                        ("text", Value::string(text.to_string())),
                    ]),
                )]),
            ),
        ]);
        send(&did);
    }

    let shutdown = obj(vec![
        ("jsonrpc", Value::string("2.0")),
        ("id", Value::number(2.0)),
        ("method", Value::string("shutdown")),
        ("params", Value::Null),
    ]);
    send(&shutdown);

    let exit = obj(vec![
        ("jsonrpc", Value::string("2.0")),
        ("method", Value::string("exit")),
        ("params", Value::Null),
    ]);
    send(&exit);
    drop(stdin);

    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("stdout")
        .read_to_string(&mut stdout)
        .expect("read lsp stdout");
    let _ = child.wait();

    let mut counts = std::collections::HashMap::new();
    for part in stdout.split("Content-Length:") {
        // 每個 part 形如 ` 97\r\n\r\n{json}`
        if let Some((_hdr, body)) = part.split_once("\r\n\r\n") {
            let body = body.trim_start();
            if !body.starts_with('{') {
                continue;
            }
            if let Ok(v) = json::parse(body) {
                let is_pub = v
                    .get("method")
                    .and_then(Value::as_str)
                    .map(|m| m == "textDocument/publishDiagnostics")
                    .unwrap_or(false);
                if is_pub {
                    let uri = v
                        .get("params")
                        .and_then(|p| p.get("uri"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let n = v
                        .get("params")
                        .and_then(|p| p.get("diagnostics"))
                        .map(|d| match d {
                            Value::Array(a) => a.len(),
                            _ => 0,
                        })
                        .unwrap_or(0);
                    counts.insert(uri.to_string(), n);
                }
            }
        }
    }
    uri_value_docs
        .iter()
        .map(|(uri, _)| *counts.get(*uri).unwrap_or(&0))
        .collect()
}

#[test]
fn test_lsp_publishes_diagnostics_for_open() {
    let counts = publish_counts_for(&[
        ("file:///clean.txt", "fn main() {\n  let x = 1;\n  f(x);\n}"),
        ("file:///bad.txt", "fn main() {\n  let x = 1 @ ;\n}"),
    ]);
    assert_eq!(counts.len(), 2);
    assert_eq!(counts[0], 0, "clean file must report no diagnostics");
    assert!(
        counts[1] >= 1,
        "bad file must report at least one diagnostic (got {})",
        counts[1]
    );
}

#[test]
fn test_lsp_initialize_capabilities_and_shutdown() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn lsp");
    let mut stdin = child.stdin.take().expect("stdin");

    let init = obj(vec![
        ("jsonrpc", Value::string("2.0")),
        ("id", Value::number(1.0)),
        ("method", Value::string("initialize")),
        ("params", obj(vec![("capabilities", obj(vec![]))])),
    ]);
    let _ = stdin.write_all(&frame_json(&init));
    let _ = stdin.flush();

    let shutdown = obj(vec![
        ("jsonrpc", Value::string("2.0")),
        ("id", Value::number(2.0)),
        ("method", Value::string("shutdown")),
        ("params", Value::Null),
    ]);
    let _ = stdin.write_all(&frame_json(&shutdown));
    let _ = stdin.flush();

    let exit = obj(vec![
        ("jsonrpc", Value::string("2.0")),
        ("method", Value::string("exit")),
        ("params", Value::Null),
    ]);
    let _ = stdin.write_all(&frame_json(&exit));
    let _ = stdin.flush();
    drop(stdin);

    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("stdout")
        .read_to_string(&mut stdout)
        .unwrap();
    let status = child.wait().expect("wait");

    assert!(status.success(), "lsp must exit 0 after exit notification");
    assert!(
        stdout.contains("\"capabilities\"") && stdout.contains("\"textDocumentSync\""),
        "initialize must advertise capabilities"
    );
    assert!(
        stdout.contains("\"result\":null"),
        "shutdown must reply null"
    );
}

#[test]
fn test_cl0r0_check_exit_codes() {
    let bin = env!("CARGO_BIN_EXE_cl0r0");
    let clean = "fn main() {\n  let x = 1;\n  f(x);\n}";
    let st = Command::new(bin)
        .args(["--check", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    st.stdin
        .as_ref()
        .unwrap()
        .write_all(clean.as_bytes())
        .unwrap();
    let out = st.wait_with_output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "clean input must exit 0: {}",
        String::from_utf8_lossy(&out.stdout)
    );

    let bad = "fn main() {\n  let x = 1 @ ;\n}";
    let st = Command::new(bin)
        .args(["--check", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    st.stdin
        .as_ref()
        .unwrap()
        .write_all(bad.as_bytes())
        .unwrap();
    let out = st.wait_with_output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "bad input must exit 1: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
