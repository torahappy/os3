#!/bin/bash
chokidar 'src/**/*' --initial -c "cargo build --profile=release --target=wasm32-unknown-emscripten --bin $1; cp ./target/wasm32-unknown-emscripten/release/$1.js ./wasm/; cp ./target/wasm32-unknown-emscripten/release/$1.wasm ./wasm;"
