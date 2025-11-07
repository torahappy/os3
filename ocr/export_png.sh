#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

export PATH="${SCRIPT_DIR}/../external-apps/tesseract/bin:${PATH}"

export TESSDATA_PREFIX="${SCRIPT_DIR}/../external-apps/tesseract/share/tessdata/"

find ./out -type f -exec bash -c 'inkscape -b white --export-type=png --export-width=1920 "{}"' \;

mv ./out/*.png ./out_png
