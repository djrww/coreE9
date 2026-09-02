#!/usr/bin/env sh
# 一鍵重建 cl0r0 開發環境(沙箱/CI 通用;需 sudo 與網路)。
# 快照不保留 ~/.rustup 工具鏈、~/.cargo/bin 符號連結與 apt 安裝的 coq,
# 換環境後執行本檔即可恢復。
#
# 用法:sh scripts/setup_dev.sh [--without-rocq]

set -eu

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
  if [ ! -x "$HOME/.cargo/bin/cargo" ]; then
    echo "== installing rust toolchain (rustup) =="
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      -o /tmp/rustup-init.sh
    sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain stable
  fi
  PATH="$HOME/.cargo/bin:$PATH"; export PATH
fi
cargo --version
rustc --version

echo "== setup done =="
