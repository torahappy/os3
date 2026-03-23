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

emsdk install 4.0.10
emsdk activate 4.0.10

if [ ! -d ../external-apps/tesseract ] && [ $BUILD_NATIVE -eq 1 ]; then

pushd ./tesseract-$TESSERACT_VERSION

  git clean -dfx


  PKG_CONFIG_PATH="$ADDITIONAL_PKGCONFIG_PATH" cmake -B build -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -d ../external-apps/zlibng-wasm ] && [ $BUILD_WASM -eq 1 ]; then

pushd zlibng-$ZLIBNG_VERSION
  git clean -dfx

  emcmake cmake -B build -DZLIB_COMPAT=ON -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/zlibng-wasm"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi


if [ ! -d ../external-apps/png-wasm ] && [ $BUILD_WASM -eq 1 ]; then

pushd png-$PNG_VERSION
  git clean -dfx

  emcmake cmake -B build -DPNG_STATIC=ON -DPNG_SHARED=OFF -DZLIB_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/zlibng-wasm/include" -DZLIB_LIBRARY="${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib" -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/png-wasm"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -d ../external-apps/jpeg-wasm ] && [ $BUILD_WASM -eq 1 ]; then

pushd jpeg-$JPEG_VERSION
  git clean -dfx

  emconfigure ./configure --prefix="${SCRIPT_DIR}/../external-apps/jpeg-wasm"
  make -j$NPR
  make install -j$NPR

popd

fi


if [ ! -d ../external-apps/leptonica-wasm ] && [ $BUILD_WASM -eq 1 ]; then
# missing libs: tiff, gif, webp, openjpeg
pushd leptonica-$LEPTONICA_VERSION
  git clean -dfx

  emcmake cmake -B build -DBUILD_SHARED_LIBS=OFF -DZLIB_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/zlibng-wasm/include" -DZLIB_LIBRARY="${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib" -DPNG_PNG_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/png-wasm/include" -DPNG_LIBRARY="${SCRIPT_DIR}/../external-apps/png-wasm/lib" -DJPEG_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/jpeg-wasm/include" -DJPEG_LIBRARY="${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib" -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/leptonica-wasm"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -d ../external-apps/tesseract-wasm ] && [ $BUILD_WASM -eq 1 ]; then

pushd tesseract-$TESSERACT_VERSION

  git clean -dfx

  mkdir "${SCRIPT_DIR}/../external-apps/pc-wasm" || true
  
  cp "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/zlib.pc" "${SCRIPT_DIR}/../external-apps/pc-wasm/ZLIB.pc" || true
  cp "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/zlib.pc" "${SCRIPT_DIR}/../external-apps/pc-wasm/zlib.pc" || true
  cp "${SCRIPT_DIR}/../external-apps/png-wasm/lib/pkgconfig/libpng.pc" "${SCRIPT_DIR}/../external-apps/pc-wasm/PNG.pc" || true
  cp "${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib/pkgconfig/libjpeg.pc" "${SCRIPT_DIR}/../external-apps/pc-wasm/JPEG.pc" || true
  cp "${SCRIPT_DIR}/../external-apps/leptonica-wasm/lib/pkgconfig/lept.pc" "${SCRIPT_DIR}/../external-apps/pc-wasm/lept.pc" || true

  sed -i'.bak' -e s/libwebp// -e s/libwebpmux// "${SCRIPT_DIR}/../external-apps/pc-wasm/lept.pc"

  PKG_CONFIG_PATH="${SCRIPT_DIR}/pc-wasm:${SCRIPT_DIR}/../external-apps/pc-wasm" emcmake cmake -B build -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract-wasm" -DBUILD_TRAINING_TOOLS=OFF -DCMAKE_CXX_FLAGS="-O3 -sUSE_ICU=1 -sALLOW_MEMORY_GROWTH=1 -I\"${SCRIPT_DIR}/../external-apps/leptonica-wasm/include/leptonica\" -Wl,\"-L${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib/\",-ljpeg,\"-L${SCRIPT_DIR}/../external-apps/png-wasm/lib/\",-lpng,\"-L${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/\",-lz" -DGRAPHICS_DISABLED=ON

  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR
    cp ./bin/tesseract.wasm "${SCRIPT_DIR}/../external-apps/tesseract-wasm/bin"

  popd

popd

fi

if [ ! -d ../external-apps/harfbuzz-wasm ] && [ $BUILD_WASM -eq 1 ]; then

pushd harfbuzz-$HARFBUZZ_VERSION
  git clean -dfx

  EXFUNCS='["_hb_blob_create_from_file", "_hb_face_create", "_hb_font_create", "_hb_font_get_glyph", "_hb_font_get_glyph_extents", "_hb_font_get_glyph_advance_for_direction", "_hb_buffer_create", "_hb_buffer_set_content_type", "_hb_buffer_destroy", "_hb_buffer_set_direction", "_hb_buffer_add_codepoints", "_hb_shape", "_hb_buffer_get_glyph_positions", "_malloc", "_free"]'

  emcmake cmake -B build -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/harfbuzz-wasm"
  cmake --build build --config Release -j$NPR

  pushd build
    make install -j$NPR
    cd "${SCRIPT_DIR}/../external-apps/harfbuzz-wasm/lib"
    em++ libharfbuzz.a -O3 -o harfbuzz -sEXPORTED_FUNCTIONS="$EXFUNCS" -sEXPORTED_RUNTIME_METHODS=stringToUTF8,UTF8ToString,AsciiToString,intArrayFromString,intArrayToString,writeArrayToMemory,setValue,getValue,HEAP8,HEAP16,HEAP32,HEAPU8,HEAPU16,HEAPU32
  popd

popd

fi

if [ ! -f ../external-apps/tesseract/share/tessdata/eng.traineddata ]; then

  mkdir -p ../external-apps/tesseract/share/tessdata/ || true

  cp ./eng.traineddata ../external-apps/tesseract/share/tessdata/

fi
