#!/bin/bash
chokidar 'src/**/*' 'build.rs' --initial -c "killall $1; cargo run --bin $1"
