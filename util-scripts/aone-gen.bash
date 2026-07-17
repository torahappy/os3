#!/bin/bash

TARGET=0.8 bash gen-cells.bash 100

mv cells aone-cells

python grid-placement-batch.py --base ../rpg-helper/aone-ura.svg --cells-dir ./aone-cells --out-dir ./aone-out --interval-w 91 --interval-h 55 --start-x 66.412 --start-y 24 --num-each 10

cp ../rpg-helper/ikiteikou-logo.png aone-out

cd aone-out

mkdir pdf

find . -name "*.svg" -exec inkscape {} --export-dpi=300 --export-filename=pdf/{}.pdf \;
