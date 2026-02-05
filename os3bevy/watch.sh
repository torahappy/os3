#!/bin/bash

. ./shell-common.sh

chokidar "${WATCHFILES[@]}" --initial -c "killall $1; cargo run --profile=release --bin $1"
