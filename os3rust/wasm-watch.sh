#!/bin/bash

chokidar 'src/**/*' 'build.rs' --initial -c "cargo build --profile=release --target=wasm32-unknown-unknown --bin $1; mkdir ./wasm/$1; cp ./target/wasm32-unknown-unknown/release/$1.wasm ./wasm/$1;$HOME/.cargo/bin/wasm-bindgen --target web --out-dir ./wasm/$1 ./wasm/$1/$1.wasm"
