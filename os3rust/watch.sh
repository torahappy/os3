#!/bin/bash
chokidar 'src/**/*' --initial -c "killall $1; cargo run --bin $1"
