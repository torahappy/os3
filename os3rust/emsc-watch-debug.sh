#!/bin/bash

. ./shell-common.sh

chokidar "${WATCHFILES[@]}" --initial -c "cargo build --target=wasm32-unknown-emscripten --bin $1; mkdir wasm/$1; cp ./target/wasm32-unknown-emscripten/debug/$1.js ./wasm/$1; cp ./target/wasm32-unknown-emscripten/debug/$1.wasm ./wasm/$1;"
