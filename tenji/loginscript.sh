#!/bin/bash

cd "$(dirname "$0")"

killall tenji_movie_infloop tenji_rireki_infloop

./tenji_movie_infloop &

./tenji_rireki_infloop &

