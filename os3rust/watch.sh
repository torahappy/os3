#!/bin/bash
chokidar 'src/**/*' 'templates/*' 'build.rs' --initial -c "killall $1; cargo run --bin $1"
