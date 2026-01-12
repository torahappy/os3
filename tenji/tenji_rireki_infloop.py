#!/bin/python3

# rireki lvl.2 watcher

import subprocess
import os

basepath = os.path.abspath(os.path.dirname(__file__))

while True:
    subprocess.run(["/usr/bin/python3", os.path.join(basepath, "tenji_rireki.py")], stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)

