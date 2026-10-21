#!/bin/sh
# Builds the WebAssembly module into web/.
# Needs the wasm32 target once:
#   Fedora: sudo dnf install rust-std-static-wasm32-unknown-unknown
#   rustup: rustup target add wasm32-unknown-unknown
set -e
cd "$(dirname "$0")"
cargo build --release --lib --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/domination_solver.wasm web/
echo "Built web/domination_solver.wasm"
echo "Serve with: ./serve.py  ->  http://localhost:8000"
