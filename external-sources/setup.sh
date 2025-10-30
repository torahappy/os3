set -euo pipefail

NPR=$(dc -e "$(nproc) 2 / p")

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

if [ ! -d ../external-apps/tesseract ]; then

pushd ./tesseract-5.5.1

  cmake -B build -DCMAKE_INSTALL_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract"
  cmake --build build --config Release -j$NPR

  pushd build

    make install -j$NPR

  popd

popd

fi

if [ ! -f ../external-apps/tesseract/share/tessdata/eng.traineddata ]; then

  cp ./eng.traineddata ../external-apps/tesseract/share/tessdata/

fi
