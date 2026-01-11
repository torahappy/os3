#!/bin/python3

import subprocess
import sys
import time
from typing import TypedDict
import os
import atexit
import re
import signal
import time

MAX_APP_TIME = 3600
APP_START_TIME = time.perf_counter()

class MyState(TypedDict):
    started: bool
    process_voice_server: subprocess.Popen | None
    process_rireki: subprocess.Popen | None
    to_check_startup_flag: list[bool]

def exit_func():
    to_terminate = [my_state['process_rireki'], my_state['process_voice_server']]
    for p in to_terminate:
        if p is not None and p.poll() == None:
            p.terminate()
            p.wait()

atexit.register(exit_func)

def initial_state() -> MyState:
    return {
        "started": False,
        "process_rireki": None,
        "process_voice_server": None,
        "to_check_startup_flag": [False, False],
    }

basepath = os.path.dirname(os.path.abspath(__file__))

src_path = os.path.join(
            basepath, "..", "os3rust"
        )

my_state: MyState = initial_state()

while True:
    if time.perf_counter() - APP_START_TIME > MAX_APP_TIME:
        print("Something Bad happened !!! app executing time is way longer than expected !! shutting down...")
        sys.exit()
    if not my_state['started']:
        my_state['started'] = True
        my_state['process_rireki'] = subprocess.Popen(["cargo", "run", "--profile", "release", "--bin", "rireki"], cwd=src_path, stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
        my_state['process_rireki'] = subprocess.Popen([os.path.join(
            basepath, "..", "python", "venv", "bin", "uvicorn"
        ), "voice_server:app"], cwd=os.path.join(
            basepath, "..", "python"
        ), stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)

    pids_raw = subprocess.run(["ps", "-Ao", "pid,args"], capture_output=True, text=True)
    pids = [re.match(r'^\s*(\d+)\s*(.*)$', l) for l in pids_raw.stdout.split("\n")]
    pids = [(int(p[1]), str(p[2])) for p in pids if p is not None]

    voice_server_pids = [p[0] for p in pids if 'python/venv/bin/uvicorn voice_server:app' in p[1]]

    rireki_pids = [p[0] for p in pids if p[1].endswith('target/release/rireki')]

    to_check = [voice_server_pids, rireki_pids]

    for i, x in enumerate(to_check):
        if len(x) > 0 and my_state['to_check_startup_flag'][i] == False:
            my_state['to_check_startup_flag'][i] = True
        if len(x) == 0 and my_state['to_check_startup_flag'][i] == True:
            print("Process is down unexpectedly or expectedly. Shutting down...")
            sys.exit()
        if len(x) > 1:
            print("Something Bad happened !!! Duplicate processes !!! Terminating them all and shutting down ...")
            for pid in x:
                try:
                    os.kill(pid, signal.SIGTERM)
                except:
                    pass

                time.sleep(10)

                try:
                    os.kill(pid, signal.SIGKILL)
                except:
                    pass

                sys.exit()

    time.sleep(1)

