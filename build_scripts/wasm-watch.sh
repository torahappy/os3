#!/bin/bash

cd "$(dirname "$0")"/../

. ./build_scripts/shell-common.sh

cd $1

chokidar "${WATCHFILES[@]}" --initial -c "cargo build --profile=release --target=wasm32-unknown-unknown --bin $2; mkdir ./wasm/$2; cp ./target/wasm32-unknown-unknown/release/$2.wasm ./wasm/$2;$HOME/.cargo/bin/wasm-bindgen --target web --out-dir ./wasm/$2 ./wasm/$2/$2.wasm"
