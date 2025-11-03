set -euo pipefail

NPR=$(dc -e "$(nproc) 2 / p")

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

. sources

emsdk install 4.0.10
emsdk activate 4.0.10

if [ ! -d ../external-apps/tesseract ]; then

pushd ./tesseract-$TESSERACT_VERSION

  cmake -B build -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -d ../external-apps/zlibng-wasm ]; then

pushd zlibng-$ZLIBNG_VERSION

  emcmake cmake -B build -DZLIB_COMPAT=ON -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/zlibng-wasm"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi


if [ ! -d ../external-apps/png-wasm ]; then

pushd png-$PNG_VERSION

  emcmake cmake -B build -DPNG_STATIC=ON -DPNG_SHARED=OFF -DZLIB_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/zlibng-wasm/include" -DZLIB_LIBRARY="${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib" -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/png-wasm"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -d ../external-apps/jpeg-wasm ]; then

pushd jpeg-$JPEG_VERSION

  emconfigure ./configure --prefix="${SCRIPT_DIR}/../external-apps/jpeg-wasm"
  make -j$NPR
  make install -j$NPR

popd

fi


if [ ! -d ../external-apps/leptonica-wasm ]; then
# missing libs: tiff, gif, webp, openjpeg
pushd leptonica-$LEPTONICA_VERSION

  emcmake cmake -B build -DBUILD_SHARED_LIBS=OFF -DZLIB_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/zlibng-wasm/include" -DZLIB_LIBRARY="${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib" -DPNG_PNG_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/png-wasm/include" -DPNG_LIBRARY="${SCRIPT_DIR}/../external-apps/png-wasm/lib" -DJPEG_INCLUDE_DIR="${SCRIPT_DIR}/../external-apps/jpeg-wasm/include" -DJPEG_LIBRARY="${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib" -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/leptonica-wasm"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -d ../external-apps/tesseract-wasm ]; then

pushd tesseract-$TESSERACT_VERSION
  
  cp "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/zlib.pc" "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/ZLIB.pc"
  cp "${SCRIPT_DIR}/../external-apps/png-wasm/lib/pkgconfig/libpng.pc" "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/PNG.pc"
  cp "${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib/pkgconfig/libjpeg.pc" "${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig/JPEG.pc"

  CFLAGS='-sUSE_ICU=1' PKG_CONFIG_PATH="${SCRIPT_DIR}/pkgconfig:${SCRIPT_DIR}/../external-apps/leptonica-wasm/lib/pkgconfig:${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/pkgconfig:${SCRIPT_DIR}/../external-apps/png-wasm/lib/pkgconfig:${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib/pkgconfig" emcmake cmake -B build -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract-wasm" -DBUILD_TRAINING_TOOLS=OFF -DCMAKE_CXX_FLAGS="-sUSE_ICU=1 -I\"${SCRIPT_DIR}/../external-apps/leptonica-wasm/include/leptonica\" -Wl,\"-L${SCRIPT_DIR}/../external-apps/jpeg-wasm/lib/\",-ljpeg,\"-L${SCRIPT_DIR}/../external-apps/png-wasm/lib/\",-lpng,\"-L${SCRIPT_DIR}/../external-apps/zlibng-wasm/lib/\",-lz" -DGRAPHICS_DISABLED=ON

  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -f ../external-apps/tesseract/share/tessdata/eng.traineddata ]; then

  cp ./eng.traineddata ../external-apps/tesseract/share/tessdata/

fi
