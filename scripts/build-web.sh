#!/usr/bin/env sh
set -eu

cargo build --locked --release --target wasm32-unknown-unknown

mkdir -p dist
cp web/index.html dist/index.html
cp target/wasm32-unknown-unknown/release/pocket-orrery.wasm dist/pocket-orrery.wasm
