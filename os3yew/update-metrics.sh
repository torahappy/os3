cd "$(dirname "$0")"

python ../util-scripts/metrics.py --source assets/doomscroll/komagire/ --output metadata/doomscroll/komagire.json
python ../util-scripts/metrics.py --source assets/doomscroll/inquire/ --output metadata/doomscroll/inquire.json
