#!/bin/bash

TARGET=0.8 bash gen-cells.bash 1000

python grid-placement-batch.py --base ../rpg-helper/aone-ura.svg --cells-dir ./cells --out-dir ./aone-out --interval-w 91 --interval-h 55 --start-x 66.412 --start-y 22 --num-each 10

cp ../rpg-helper/ikiteikou-logo.png aone-out

cd aone-out

mkdir pdf

find . -name "*.svg" -exec inkscape {} --export-dpi=300 --export-filename=pdf/{}.pdf \;
