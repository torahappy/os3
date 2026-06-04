cd "$(dirname "$0")"

python ../util-scripts/metrics.py --source assets/doomscroll/komagire/ --output assets/doomscroll/komagire.json
python ../util-scripts/metrics.py --source assets/doomscroll/inquire/ --output assets/doomscroll/inquire.json
