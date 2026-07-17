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

#  Create the output directory
mkdir -p cells

#  Create each SVG file.  We use a simple for‑loop so that the
#  shell can do the substitution on $i for us.
for ((i = 1; i <= NUM; i++)); do
    #  The content of the file is literally the output of
    #  `qrencode $i -tsvg`.  We capture that output into a variable
    #  and then write it to the file.
    #
    #  If you want the file to be *exactly* the raw output of
    #  qrencode (i.e. no quoting, no extra newlines) you can just
    #  run the command directly into the file:
    #
    #      qrcode=$(qrencode "$i" -tsvg)
    #      printf "%s" "$qrcode" > "cells/c${i}.svg"
    #
    #  The above two lines are equivalent to the single one below:
    #
    signature=$(echo -n "${password}/ikiteikou_os_v0.0002_aone_cards/${i}" | openssl sha256 -binary | base64)
    qrencode "$i $signature" -tsvg > "cells/${i}.svg"
    metrics_data="$(cat "cells/${i}.svg" | rg -o 'width="\d\.\d+cm"' | rg -o "\d\.\d+")"
    if [[ "${SCALE}" != "" ]]; then
      metrics_data_after="$(echo "${metrics_data} * ${SCALE}" | bc)"
    elif [[ "${TARGET}" != "" ]]; then
      metrics_data_after="$TARGET"
    fi
    if [[ "${metrics_data_after}" != "" ]]; then
      sed -i -e 's/"'"${metrics_data}"'cm"/"'"${metrics_data_after}"'cm"/g' "cells/${i}.svg"
    fi
done

