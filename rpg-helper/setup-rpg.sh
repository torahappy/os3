#!/bin/bash

set -e

if [ "$BUILD_WASM" == "" ]; then
  BUILD_WASM=1
fi

set -euo pipefail

case "$OSTYPE" in
  darwin*)   NPR=$(dc -e "$(sysctl -n hw.physicalcpu) 2 / p"); ADDITIONAL_PKGCONFIG_PATH=(/opt/homebrew/Cellar/icu4c@78/*/lib/pkgconfig);;
  linux*)    NPR=$(dc -e "$(nproc) 2 / p") ADDITIONAL_PKGCONFIG_PATH="";;
esac

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

. ../external-sources/sources

if [ $BUILD_WASM -eq 1 ]; then
  emsdk install 4.0.10
  emsdk activate 4.0.10
fi

if [ ! -d ../external-apps/inih-wasm ] && [ $BUILD_WASM -eq 1 ]; then

pushd ../external-sources/inih-$INIH_VERSION

  git clean -dfx

  mkdir build

  pushd build

    meson -Ddefault_library=static --cross-file="${SCRIPT_DIR}/../external-sources/cross-file/emscripten-inih.txt" --prefix="${SCRIPT_DIR}/../external-apps/inih-wasm" ..

    ninja

    ninja install

  popd

popd

fi

if [ ! -d ../external-apps/lcf-wasm ] && [ $BUILD_WASM -eq 1 ]; then

pushd ../external-sources/liblcf-$LCF_VERSION

  git clean -dfx

  patch -Np1 < ../patches/lcf_wasm.patch || true

  emcmake cmake -B build -DLIBLCF_WITH_XML=OFF -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/lcf-wasm" -DINIH_LIBRARY="${SCRIPT_DIR}/../external-apps/inih-wasm/lib/libinih.a" -DINIH_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/inih-wasm/include" -DCMAKE_CXX_FLAGS="-sUSE_ICU=1"
  
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

  git restore .

popd

fi
