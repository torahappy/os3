#!/usr/bin/env bash
# ------------------------------------------------------------------
#  A tiny script that writes c1.svg … c$NUM.svg, each containing
#  "qrencode $i -tsvg", and finally prints "cells/c1.svg cells/c2.svg …"
#
#  Usage:   ./make_qrcodes.sh <number_of_cells>
# ------------------------------------------------------------------

#  Grab the number of cells from the first argument (or default to 10)
NUM=${1:-10}

read -r -s -p "Password: " password
read -r -s -p "Data ('/' delimiter): " data

data="${data}/$(date +%s)"

#  Create the output directory
mkdir -p cells

signature=$(echo -n "${password}/ikiteikou_os_v0.0002_data_input/${data}" | openssl sha256 -binary | base64)

qrencode -tsvg "$(echo $data | sed -e 's/\// /g') $signature" > /tmp/test-data-input.png

ristretto /tmp/test-data-input.png
