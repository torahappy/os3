set -euo pipefail

NPR=$(dc -e "$(nproc) 2 / p")

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

. sources

emsdk install 4.0.10
emsdk activate 4.0.10

if [ ! -d ../external-apps/tesseract ]; then

pushd ./tesseract-$TESSERACT_VERSION

  git clean -dfx

  cmake -B build -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -d ../external-apps/zlibng-wasm ]; then

pushd zlibng-$ZLIBNG_VERSION
  git clean -dfx

  emcmake cmake -B build -DZLIB_COMPAT=ON -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/zlibng-wasm"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi


if [ ! -d ../external-apps/png-wasm ]; then

pushd png-$PNG_VERSION
  git clean -dfx

  emcmake cmake -B build -DPNG_STATIC=ON -DPNG_SHARED=OFF -DZLIB_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/zlibng-wasm/include" -DZLIB_LIBRARY="${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib" -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/png-wasm"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -d ../external-apps/jpeg-wasm ]; then

pushd jpeg-$JPEG_VERSION
  git clean -dfx

  emconfigure ./configure --prefix="${SCRIPT_DIR}/../external-apps/jpeg-wasm"
  make -j$NPR
  make install -j$NPR

popd

fi


if [ ! -d ../external-apps/leptonica-wasm ]; then
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

if [ ! -d ../external-apps/tesseract-wasm ]; then

pushd tesseract-$TESSERACT_VERSION

  git clean -dfx
  
  cp "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/zlib.pc" "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/ZLIB.pc"
  cp "${SCRIPT_DIR}/../external-apps/png-wasm/lib/pkgconfig/libpng.pc" "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/PNG.pc"
  cp "${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib/pkgconfig/libjpeg.pc" "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/JPEG.pc"

  PKG_CONFIG_PATH="${SCRIPT_DIR}/pkgconfig:${SCRIPT_DIR}/../external-apps/leptonica-wasm/lib/pkgconfig:${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig:${SCRIPT_DIR}/../external-apps/png-wasm/lib/pkgconfig:${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib/pkgconfig" emcmake cmake -B build -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract-wasm" -DBUILD_TRAINING_TOOLS=OFF -DCMAKE_CXX_FLAGS="-sUSE_ICU=1 -sALLOW_MEMORY_GROWTH=1 -I\"${SCRIPT_DIR}/../external-apps/leptonica-wasm/include/leptonica\" -Wl,\"-L${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib/\",-ljpeg,\"-L${SCRIPT_DIR}/../external-apps/png-wasm/lib/\",-lpng,\"-L${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/\",-lz" -DGRAPHICS_DISABLED=ON

  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR
    cp ./bin/tesseract.wasm "${SCRIPT_DIR}/../external-apps/tesseract-wasm/bin"

  popd

popd

fi

if [ ! -d ../external-apps/harfbuzz-wasm ]; then

pushd harfbuzz-$HARFBUZZ_VERSION
  # git clean -dfx
  EXFUNCS='["_hb_blob_create_from_file", "_hb_face_create", "_hb_font_create", "_hb_font_get_glyph", "_hb_font_get_glyph_extents", "_hb_font_get_glyph_advance_for_direction", "_hb_buffer_create", "_hb_buffer_set_content_type", "_hb_buffer_destroy", "_hb_buffer_set_direction", "_hb_buffer_add_codepoints", "_hb_shape", "_hb_buffer_get_glyph_positions", "_malloc", "_free"]'

  emcmake cmake -B build -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/harfbuzz-wasm"
  cmake --build build --config Release -j$NPR

  pushd build
    make install -j$NPR
    cd "${SCRIPT_DIR}/../external-apps/harfbuzz-wasm/lib"
    em++ libharfbuzz.a -o harfbuzz -sEXPORTED_FUNCTIONS="$EXFUNCS" -sEXPORTED_RUNTIME_METHODS=stringToUTF8,UTF8ToString,AsciiToString,intArrayFromString,intArrayToString,writeArrayToMemory,setValue,getValue,HEAP8,HEAP16,HEAP32,HEAPU8,HEAPU16,HEAPU32
  popd

popd

fi

if [ ! -f ../external-apps/tesseract/share/tessdata/eng.traineddata ]; then

  cp ./eng.traineddata ../external-apps/tesseract/share/tessdata/

fi
