#!/usr/bin/env sh
# 一鍵重建 cl0r0 開發環境(沙箱/CI 通用;需 sudo 與網路)。
# 快照不保留 ~/.rustup 工具鏈、~/.cargo/bin 符號連結與 apt 安裝的 coq,
# 換環境後執行本檔即可恢復。
#
# 用法:sh scripts/setup_dev.sh [--without-rocq]

set -eu

# 關鍵:sudo 執行時 $HOME=/root,會把 rustup 裝進 /root/.cargo(用戶 shell 找不到 cargo)。
# 因此以 SUDO_USER 的真實 home 為工具鏈落點(本專案實測踩過的坑,見 docs/HARD-ITEMS.md §#4)。
REAL_HOME="$HOME"
if [ "$(id -u)" = 0 ] && [ -n "${SUDO_USER:-}" ]; then
  REAL_HOME="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
fi

WITH_ROCQ=1
for a in "$@"; do
  case "$a" in
    --without-rocq) WITH_ROCQ=0 ;;
  esac
done

# ── Rocq(Coq)+ MathComp ────────────────────────────────────────────────
if [ "$WITH_ROCQ" = 1 ]; then
  if ! command -v coqc >/dev/null 2>&1; then
    echo "== installing coq + mathcomp (apt) =="
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
      coq libcoq-mathcomp-ssreflect
  fi
  coqc --version | head -1
fi

# ── Rust(工具鏈可能在快照後消失)───────────────────────────────────────
if ! command -v cargo >/dev/null 2>&1; then
  if [ ! -x "$REAL_HOME/.cargo/bin/cargo" ]; then
    echo "== installing rust toolchain (rustup) =="
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      -o /tmp/rustup-init.sh
    # 2026-09-03 修:rustup-init ≥1.28 不再接受位置參數形式的組件名
    # (「unexpected argument 'rustfmt'」);改為安裝後 rustup component add。
    CARGO_HOME="$REAL_HOME/.cargo" RUSTUP_HOME="$REAL_HOME/.rustup" \
      sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain stable
    CARGO_HOME="$REAL_HOME/.cargo" RUSTUP_HOME="$REAL_HOME/.rustup" \
      "$REAL_HOME/.cargo/bin/rustup" component add rustfmt clippy
  fi
  PATH="$REAL_HOME/.cargo/bin:$PATH"; export PATH
fi
# --check:只報版本,不安裝(供 CI/文檔驗證可重建性)
if [ "${1:-}" = "--check" ]; then
  cargo --version; rustc --version; command -v coqc >/dev/null && coqc --version | head -1
  echo "== check only, no changes =="; exit 0
fi
cargo --version
rustc --version

echo "== setup done =="
