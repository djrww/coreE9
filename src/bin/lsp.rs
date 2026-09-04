//! lsp —— ROADMAP **P2 #10「最小 LSP(僅診斷)」** 的 CLI 啟動器。
//!
//! 在 stdin/stdout 上跑一個 JSON-RPC(Content-Length 訊框)的 LSP 伺服器,
//! 對「開啟/修改的文檔」發布 `textDocument/publishDiagnostics`。
//! 測試(`cargo test`)與本文檔不啟動伺服器,改以 `cl0r0 --check <file>` 走 CLI。
//!
//! 互動方式:用任何 LSP 客戶端(如 VS Code / Neovim)連接,或手動餵 JSON-RPC。
//! 一鍵端到端測試(printf 餵一則 didOpen 並讀回應):
//!     printf '%s' ... | cargo run --bin lsp

fn main() {
    cl0r0::lsp::run();
}
