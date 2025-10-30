#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

export PATH="${SCRIPT_DIR}/../external-apps/tesseract/bin:${PATH}"

export TESSDATA_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract/share/tessdata/"

find ../data-source-local/shorui/ -type f -exec bash -c 'if [ ! -f "out/$(basename "{}").svg" ]; then ./venv/bin/python ./script.py {} > out/$(basename "{}").svg; fi' \;
