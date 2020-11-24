#!/usr/bin/env sh
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --target wasm32-unknown-unknown --features wasm --no-default-features --release
wasm-bindgen --out-dir pkg --web ../target/wasm32-unknown-unknown/release/cheese.wasm
python -m http.server 8000
