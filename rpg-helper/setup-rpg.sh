#!/bin/bash

set -e

if [ "$BUILD_WASM" == "" ]; then
  BUILD_WASM=1
fi

if [ "$BUILD_NATIVE" == "" ]; then
  BUILD_NATIVE=1
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

if [ ! -d ../external-apps/inih ] && [ $BUILD_NATIVE -eq 1 ]; then

pushd ../external-sources/inih-$INIH_VERSION

  git clean -dfx

  mkdir build

  pushd build

    meson --prefix="${SCRIPT_DIR}/../external-apps/inih" ..

    ninja

    ninja install

  popd

popd

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

if [ ! -d ../external-apps/lcf ] && [ $BUILD_NATIVE -eq 1 ]; then

pushd ../external-sources/liblcf-$LCF_VERSION

  git clean -dfx

  cmake -B build  -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/lcf"

  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

  git restore .

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


if [ ! -d "${SCRIPT_DIR}/dist" ] && [ $BUILD_NATIVE -eq 1 ]; then
  mkdir "${SCRIPT_DIR}/dist"

  pushd "${SCRIPT_DIR}/dist"

  git clean -dfx
  
  g++ ../rpg_lsd_io.cpp ../rpg_lsd_io_main.cpp $(find ../../external-apps/lcf -name "liblcf.so") -I"../../external-apps/lcf/include" -O3 -o rpg_lsd_io

  g++ ../catch_amalgamated.cpp ../rpg_lsd_io.cpp ../rpg_lsd_io_test.cpp $(find ../../external-apps/lcf -name "liblcf.so") -I"../../external-apps/lcf/include" -O3 -o rpg_lsd_io_test

  popd

fi

if [ ! -d "${SCRIPT_DIR}/dist-wasm" ] && [ $BUILD_WASM -eq 1 ]; then
  mkdir "${SCRIPT_DIR}/dist-wasm"

  pushd "${SCRIPT_DIR}/dist"

  git clean -dfx

  EXFUNCS='["_malloc",
"_free",
"_fopen",
"_fclose",
"_read_rpg_var",
"_write_rpg_var",
"_read_rpg_switch",
"_write_rpg_switch",
"_read_rpg_var_lgs",
"_write_rpg_var_lgs",
"_read_rpg_switch_lgs",
"_write_rpg_switch_lgs",
"FS"]'
  
  cp ../test1.lgs ../test1.lsd .

  em++ ../rpg_lsd_io.cpp ../../external-apps/lcf-wasm/lib/liblcf.a -O3 -o rpg_lsd_io -sALLOW_MEMORY_GROWTH=1 -sMODULARIZE=1 -sEXPORT_ES6=1 -sEXPORTED_FUNCTIONS="$EXFUNCS" -sEXPORTED_RUNTIME_METHODS=stringToUTF8,UTF8ToString,AsciiToString,intArrayFromString,intArrayToString,writeArrayToMemory,setValue,getValue,HEAP8,HEAP16,HEAP32,HEAPU8,HEAPU16,HEAPU32 -I"../../external-apps/lcf-wasm/include" -sUSE_ICU=1

  em++ ../catch_amalgamated.cpp ../rpg_lsd_io.cpp ../rpg_lsd_io_test.cpp ../../external-apps/lcf-wasm/lib/liblcf.a -O3 -o rpg_lsd_io_test -sALLOW_MEMORY_GROWTH=1 -sEXPORTED_RUNTIME_METHODS=stringToUTF8,UTF8ToString,AsciiToString,intArrayFromString,intArrayToString,writeArrayToMemory,setValue,getValue,HEAP8,HEAP16,HEAP32,HEAPU8,HEAPU16,HEAPU32 --embed-file test1.lgs --embed-file test1.lsd -I"../../external-apps/lcf-wasm/include" -sUSE_ICU=1

  mv rpg_lsd_io rpg_lsd_io.js
  
  popd

fi
