#!/bin/bash

cd "$(dirname "$0")"/../

cd $1

cargo build --profile=release --target=wasm32-unknown-unknown --bin $2

mkdir ./wasm/$2

cp ./target/wasm32-unknown-unknown/release/$2.wasm ./wasm/$2

wasm-opt -O3 ./wasm/$2/$2.wasm -o ./wasm/$2/$2.wasm

wasm-bindgen --target web --out-dir ./wasm/$2 ./wasm/$2/$2.wasm

