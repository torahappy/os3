#!/bin/bash

cd "$(dirname "$0")"

BUILD_NATIVE=0 BUILD_WASM=1 ../../external-sources/setup-tts.sh
../../build_scripts/wasm-once.sh os3yew shimbun
../../os3yew/install-wasm-binaries.sh

pushd ../../os3yew/wasm/shimbun/
NODE_ENV=production yarn
popd

rm -rf ./data
mkdir ./data
cp -r ../../os3yew/wasm/shimbun/* data/
rm -rf ./data/node_modules
mkdir -p ./data/node_modules/onnxruntime-web/dist/
cp -r ../../os3yew/wasm/shimbun/node_modules/onnxruntime-web/dist/ort.min.mjs ./data/node_modules/onnxruntime-web/dist/
cp -r ../../os3yew/wasm/shimbun/node_modules/onnxruntime-web/dist/*.jsep.* ./data/node_modules/onnxruntime-web/dist/
cp -r ../../os3yew/assets data/
