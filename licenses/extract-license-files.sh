#!/bin/bash

cd "$(dirname "$0")"

find . | rg -i 'license|copying|author' | xargs -n1 bash -c 'FN="$0"; mkdir -p ../licenses/"$(dirname "$FN")"; cp "$FN" ../licenses/"$FN"'
