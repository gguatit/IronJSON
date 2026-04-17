#!/bin/bash
set -euo pipefail

if ! command -v cargo &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
    source "$HOME/.cargo/env"
fi

rustup target add wasm32-unknown-unknown

cargo install -q worker-build

worker-build --release
