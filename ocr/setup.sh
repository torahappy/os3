#!/bin/bash

set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$SCRIPT_DIR"

if [ ! -d venv ]; then

python3 -m venv venv
. ./venv/bin/activate
pip install -r requirements.txt
pip-audit --fix || true

fi
