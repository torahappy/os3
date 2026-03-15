set -euo pipefail

case "$OSTYPE" in
  darwin*)   NPR=$(dc -e "$(sysctl -n hw.physicalcpu) 2 / p"); ADDITIONAL_PKGCONFIG_PATH=(/opt/homebrew/Cellar/icu4c@78/*/lib/pkgconfig);;
  linux*)    NPR=$(dc -e "$(nproc) 2 / p") ADDITIONAL_PKGCONFIG_PATH="";;
esac

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

. sources

if [ ! -d ../external-apps/open_jtalk ]; then
  pushd ./hts_engine_api-$HTS_ENGINE_API_VERSION
  ./configure --prefix="$SCRIPT_DIR/../external-apps/open_jtalk"
  make -j$NPR
  make install
  popd

  pushd ./open_jtalk-$OPEN_JTALK_VERSION/src
  mkdir build
  pushd build
  cmake -DBUILD_PROGRAMS=1 -DCMAKE_INSTALL_PREFIX="$SCRIPT_DIR/../external-apps/open_jtalk" ..
  make -j$NPR
  make install
  popd
  popd
fi

