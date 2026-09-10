#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

cargo build --target wasm32-unknown-unknown --release
wasm-bindgen \
  --target no-modules \
  --no-typescript \
  --out-dir extension/pkg \
  target/wasm32-unknown-unknown/release/discord_mass.wasm

echo "pronto: $(du -h extension/pkg/discord_mass_bg.wasm | cut -f1) em extension/pkg"
