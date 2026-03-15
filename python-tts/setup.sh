#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

mkdir models

pushd models

wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx

wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx.json

popd

if [[ -f /opt/homebrew/bin/python3 ]]; then
  MYPYTHON=/opt/homebrew/bin/python3
else
  MYPYTHON=python3
fi

"$MYPYTHON" -m venv venv

. ./venv/bin/activate

pip install -r requirements.txt
