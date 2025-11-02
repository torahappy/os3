set -euo pipefail

NPR=$(dc -e "$(nproc) 2 / p")

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

. sources

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
# -- Could NOT find TIFF (missing: TIFF_LIBRARY TIFF_INCLUDE_DIR)
# -- Could NOT find GIF (missing: GIF_LIBRARY GIF_INCLUDE_DIR) (Required is at least version "5")
# -- Could NOT find WebP (missing: WebP_DIR)
# -- Could NOT find OpenJPEG (missing: OpenJPEG_DIR)
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

  emcmake cmake -B build -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract-wasm"

  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi



if [ ! -f ../external-apps/tesseract/share/tessdata/eng.traineddata ]; then

  cp ./eng.traineddata ../external-apps/tesseract/share/tessdata/

fi
