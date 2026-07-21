#!/bin/bash

cd "$(dirname "$0")"

cd ./games/default

chokidar --initial "../../server.py" "*.lmu" "*.ldb" "*.lmt" -c 'server_pid=$(ps -Ao pid,args | rg server.py | rg -v "(rg|vim)" | awk "{print \$1}"); kill $server_pid; killall easyrpg-player; killall zbarcam; easyrpg-player x x Window --no-pause-focus-lost 2>&1 | ts > /tmp/rpg_log1 & python ../../server.py 2>&1 | ts > /tmp/rpg_log2 &'

