#!/bin/bash

set -eu

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

cd "$SCRIPT_DIR"

BINARYEN_VERSION=132

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    BINARYEN_OS=linux
elif [[ "$OSTYPE" == "darwin"* ]]; then
    BINARYEN_OS=macos
fi

BINARYEN_ARCH="$(uname -m)"

if [[ ! -d "../external-apps/wasm-opt" ]]; then
  mkdir -p ../external-apps/wasm-opt || true
  pushd ../external-apps/wasm-opt
  wget "https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}/binaryen-version_${BINARYEN_VERSION}-${BINARYEN_ARCH}-${BINARYEN_OS}.tar.gz"
  tar xvf "binaryen-version_${BINARYEN_VERSION}-${BINARYEN_ARCH}-${BINARYEN_OS}.tar.gz"
  mv "binaryen-version_${BINARYEN_VERSION}"/* ./
  rm *.tar.gz
  rmdir "binaryen-version_${BINARYEN_VERSION}"
  popd
fi
