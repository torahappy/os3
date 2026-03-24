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

. sources

if [ ! -d ../external-apps/open_jtalk ] && [ $BUILD_NATIVE -eq 1 ]; then
  pushd ./hts_engine_api-$HTS_ENGINE_API_VERSION
  git clean -dfx

  autoreconf

  ./configure --prefix="$SCRIPT_DIR/../external-apps/open_jtalk"
  make -j$NPR
  make install
  git restore .
  popd

  pushd ./open_jtalk-$OPEN_JTALK_VERSION/src
  git clean -dfx

  mkdir build
  pushd build
  cmake -DBUILD_PROGRAMS=1 -DCMAKE_INSTALL_PREFIX="$SCRIPT_DIR/../external-apps/open_jtalk" ..
  make -j$NPR
  make install
  popd
  popd
fi

emsdk install 5.0.3
emsdk activate 5.0.3

if [ ! -d ../external-apps/open_jtalk-wasm ] && [ $BUILD_WASM -eq 1 ]; then
  pushd ./hts_engine_api-$HTS_ENGINE_API_VERSION
    patch -Np1 < ../patches/hts_engine_wasm.patch || true
    git clean -dfx

    autoreconf

    emconfigure ./configure --prefix="$SCRIPT_DIR/../external-apps/open_jtalk-wasm"
    make -j$NPR
    make install
    cp bin/hts_engine.wasm "$SCRIPT_DIR/../external-apps/open_jtalk-wasm/bin/hts_engine.wasm"
    git restore .
  popd

  pushd ./open_jtalk-$OPEN_JTALK_VERSION/src
    git clean -dfx

    mkdir build
    pushd build
      emcmake cmake -DBUILD_PROGRAMS=1 -DCMAKE_INSTALL_PREFIX="$SCRIPT_DIR/../external-apps/open_jtalk-wasm" -DHTS_ENGINE_LIB="$SCRIPT_DIR/../external-apps/open_jtalk-wasm/lib/libHTSEngine.a" -DHTS_ENGINE_INCLUDE_DIR="$SCRIPT_DIR/../external-apps/open_jtalk-wasm/include" ..
      make -j$NPR
      make install
      pushd $SCRIPT_DIR/../external-apps/open_jtalk-wasm/
        EXFUNCS='["_malloc",
"_free",
"_fopen",
"_fclose",
"_Open_JTalk_initialize", 
"_Open_JTalk_clear", 
"_Open_JTalk_load", 
"_Open_JTalk_set_sampling_frequency", 
"_Open_JTalk_set_fperiod", 
"_Open_JTalk_set_alpha", 
"_Open_JTalk_set_beta", 
"_Open_JTalk_set_speed", 
"_Open_JTalk_add_half_tone", 
"_Open_JTalk_set_msd_threshold", 
"_Open_JTalk_set_gv_weight", 
"_Open_JTalk_set_volume", 
"_Open_JTalk_set_audio_buff_size", 
"_Open_JTalk_synthesis",
"_get_struct_metrics",
"FS" ]'
	emcc $SCRIPT_DIR/open_jtalk_custom_lib/open_jtalk_custom_lib.c ./lib/libopenjtalk.a ./lib/libHTSEngine.a "-I$PWD/include/openjtalk" "-I$PWD/include" -O3 -sALLOW_MEMORY_GROWTH=1 -sEXPORTED_FUNCTIONS="$EXFUNCS" -sEXPORTED_RUNTIME_METHODS=stringToUTF8,UTF8ToString,AsciiToString,intArrayFromString,intArrayToString,writeArrayToMemory,setValue,getValue,HEAP8,HEAP16,HEAP32,HEAPU8,HEAPU16,HEAPU32 -sMODULARIZE=1 -sEXPORT_ES6=1 --embed-file "$SCRIPT_DIR"/open_jtalk_dic-1.11/@/dic --embed-file "$SCRIPT_DIR"/mmdagent_voice-1.8/takumi/takumi_happy.htsvoice@/takumi_happy.htsvoice -o openjtalk-slim
      popd
    popd
  popd
fi


if [ ! -d ../external-apps/espeak_ng-data ] && [ $BUILD_WASM -eq 1 ]; then
  pushd ./espeak_ng-$ESPEAK_NG_VERSION

  git clean -dfx
  rm -rf build-native

  echo ">>> configuring espeak_ng (native)"
  
  mkdir build-native

  pushd build-native

    cmake -DCMAKE_INSTALL_PREFIX="$SCRIPT_DIR/../external-apps/espeak_ng-data" ..

    echo ">>> building espeak_ng (native)"

    make -j$NPR

    make install

  popd

  rm -rf build-native
  popd

fi


if [ ! -d ../external-apps/espeak_ng-wasm ] && [ $BUILD_WASM -eq 1 ]; then
  pushd ./espeak_ng-$ESPEAK_NG_VERSION

  git clean -dfx
  rm -rf build-wasm

  patch -Np1 < ../patches/ucd_tools_wasm.patch || true

  mkdir build-wasm

  pushd build-wasm

    echo ">>> configuring espeak_ng (wasm)"

    emcmake cmake -DCMAKE_C_FLAGS="-O3" -DCMAKE_CXX_FLAGS="-O3" -DUSE_SPEECHPLAYER=0 -DCMAKE_INSTALL_PREFIX="$SCRIPT_DIR/../external-apps/espeak_ng-wasm" ..

    echo ">>> building espeak_ng (wasm)"

    make -j$NPR

    make install

    cp -r "$SCRIPT_DIR/../external-apps/espeak_ng-data/share/espeak-ng-data" "$SCRIPT_DIR/../external-apps/espeak_ng-wasm/share/"

    cp src/espeak-ng.wasm "$SCRIPT_DIR/../external-apps/espeak_ng-wasm/bin/"

    cp src/ucd-tools/libucd.a "$SCRIPT_DIR/../external-apps/espeak_ng-wasm/lib/"

  popd

  pushd "$SCRIPT_DIR/../external-apps/espeak_ng-wasm/"

    EXFUNCS='["_espeak_Initialize", "_espeak_SetVoiceByName", "_espeak_TextToPhonemesWithTerminator", "_malloc", "_free"]'
  
    em++ lib/libespeak-ng.a lib/libucd.a -O3 -o espeak-ng-slim -sALLOW_MEMORY_GROWTH=1 -sMODULARIZE=1 -sEXPORT_ES6=1 -sEXPORTED_FUNCTIONS="$EXFUNCS" -sEXPORTED_RUNTIME_METHODS=stringToUTF8,UTF8ToString,AsciiToString,intArrayFromString,intArrayToString,writeArrayToMemory,setValue,getValue,HEAP8,HEAP16,HEAP32,HEAPU8,HEAPU16,HEAPU32 --embed-file "$(realpath "$SCRIPT_DIR/../external-apps/espeak_ng-wasm/share/espeak-ng-data")"@"/usr/share/espeak-ng-data" 
    
  popd

  git restore .

  rm -rf build-wasm
  popd
fi

