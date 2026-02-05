#!/bin/bash

cd "$(dirname "$0")"/../

. ./build_scripts/shell-common.sh

cd $1

chokidar "${WATCHFILES[@]}" --initial -c "killall $2; cargo run --bin $2"
