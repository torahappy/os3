#!/bin/bash

. ./shell-common.sh
chokidar "${WATCHFILES[@]}" --initial -c "killall $1; cargo run --bin $1"
