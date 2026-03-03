#!/bin/bash

cd "$(dirname "$0")"

chokidar $HOME/.config/joplin-desktop/database.sqlite --initial -c './venv/bin/python3 ./auto-fetch.py shimbun-data'
