#!/bin/bash

chokidar 'src/**/*' 'build.rs' --initial -c "cargo build --profile=release --target=wasm32-unknown-emscripten --bin $1; mkdir wasm/$1; cp ./target/wasm32-unknown-emscripten/release/$1.js ./wasm/$1; cp ./target/wasm32-unknown-emscripten/release/$1.wasm ./wasm/$1;"
