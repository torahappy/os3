#!/bin/bash

cd "$(dirname "$0")"/../

cd "$1/wasm/$2"

browser-sync start --no-ghost-mode -s . -f .
