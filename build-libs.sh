#!/bin/bash
set -xe

# Build the client
cargo build -p player --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/debug/player.wasm --out-dir kibble_ctf/lib