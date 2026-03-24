#!/bin/bash

set -eu

cd "$(dirname "$0")"

cp ../python-tts/models/en_US-libritts_r-medium.onnx ../python-tts/models/en_US-libritts_r-medium.onnx.json ./wasm/shimbun/

cp ../external-apps/espeak_ng-wasm/espeak-ng-slim ./wasm/shimbun/espeak-ng-slim.js
cp ../external-apps/espeak_ng-wasm/espeak-ng-slim.wasm ./wasm/shimbun
cp ../external-apps/open_jtalk-wasm/openjtalk-slim ./wasm/shimbun/openjtalk-slim.js
cp ../external-apps/open_jtalk-wasm/openjtalk-slim.wasm ./wasm/shimbun

cd ./wasm/shimbun

yarn
