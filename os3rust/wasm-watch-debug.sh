#!/bin/bash

chokidar 'src/**/*' 'build.rs' --initial -c "cargo build --target=wasm32-unknown-emscripten --bin $1; cp ./target/wasm32-unknown-emscripten/debug/$1.js ./wasm/; cp ./target/wasm32-unknown-emscripten/debug/$1.wasm ./wasm;"
