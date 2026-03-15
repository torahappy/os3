set -euo pipefail

case "$OSTYPE" in
  darwin*)   NPR=$(dc -e "$(sysctl -n hw.physicalcpu) 2 / p"); ADDITIONAL_PKGCONFIG_PATH=(/opt/homebrew/Cellar/icu4c@78/*/lib/pkgconfig);;
  linux*)    NPR=$(dc -e "$(nproc) 2 / p") ADDITIONAL_PKGCONFIG_PATH="";;
esac

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

. sources

if [ ! -d ../external-apps/ ]; then

fi

