#!/bin/bash

cd "$(dirname "$0")"

rm -rf ./data
mkdir ./data
cp -r ../../os3yew/wasm/shimbun/* data/
rm -rf ./data/node_modules
mkdir -p ./data/node_modules/onnxruntime-web/dist/
cp -r ../../os3yew/wasm/shimbun/node_modules/onnxruntime-web/dist/ort.min.mjs ./data/node_modules/onnxruntime-web/dist/
cp -r ../../os3yew/wasm/shimbun/node_modules/onnxruntime-web/dist/*.jsep.* ./data/node_modules/onnxruntime-web/dist/
cp -r ../../os3yew/assets data/
