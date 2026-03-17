#!/bin/python3

# shimbun lvl.1 watcher

import subprocess
import sys
import time
from typing import TypedDict
import os
import atexit
import re
import signal
import time

SERVER_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "python-tts"))
ELECTRON_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "electron"))

# todo: change output via maos scripts?
# DEFAULT_SINK = 'alsa_output.pci-0000_e5_00.6.analog-stereo'
# SINK_VOLUME = '80%'

def check_pulseaudio():
    pass

class MyState(TypedDict):
    started: bool
    process_shimbun: subprocess.Popen | None
    process_piper: subprocess.Popen | None
    process_fastapi: subprocess.Popen | None
    to_check_startup_flag: list[bool]

def exit_func():
    to_terminate = [my_state['process_shimbun'], my_state['process_piper'], my_state['process_fastapi']]
    for p in to_terminate:
        if p is not None and p.poll() is None:
            p.terminate()
            p.wait()

atexit.register(exit_func)

def sigterm_handler(signum, frame):
    exit_func()
    sys.exit()

signal.signal(signal.SIGTERM, sigterm_handler)

def initial_state() -> MyState:
    return {
        "started": False,
        "process_shimbun": None,
        "process_piper": None,
        "process_fastapi": None,
        "to_check_startup_flag": [False, False, False],
    }

basepath = os.path.dirname(os.path.abspath(__file__))

src_path = os.path.join(
            basepath, "..", "os3bevy"
        )

my_state: MyState = initial_state()
import shutil

while True:
    check_pulseaudio()
    piper_py_args = ["-m", "piper.http_server", "-m", os.path.join(SERVER_DIR, "models", "en_US-libritts_r-medium.onnx"), "--port", "5111"] 
    fastapi_py_args = [os.path.join(SERVER_DIR, "venv", "bin", "uvicorn"), "tts_server:app", "--port", "3000"]

    if not my_state['started']:
        shutil.rmtree(os.path.join(os.path.expanduser('~'), 'Library', 'Application Support', 'shimbun-wa-jibun'), ignore_errors=True)
        my_state['started'] = True
        subprocess.Popen(["osascript", "-e", "set Volume 5.4"])
        my_state['process_piper'] = subprocess.Popen([os.path.join(SERVER_DIR, "venv", "bin", "python3")] + piper_py_args, cwd=SERVER_DIR)
        my_state['process_fastapi'] = subprocess.Popen(fastapi_py_args, cwd=SERVER_DIR)
        time.sleep(5)
        my_state['process_shimbun'] = subprocess.Popen([os.path.join(ELECTRON_DIR, "node_modules", ".bin", "electron"), ELECTRON_DIR])

    pids_raw = subprocess.run(["ps", "-Ao", "pid,args"], capture_output=True, text=True)
    pids = [re.match(r'^\s*(\d+)\s*(.*)$', l) for l in pids_raw.stdout.split("\n")]
    pids = [(int(p[1]), str(p[2])) for p in pids if p is not None]

    p_shimbun = [p[0] for p in pids if os.path.join(ELECTRON_DIR, "node_modules", ".bin", "electron") in p[1]]
    p_piper = [p[0] for p in pids if " ".join(piper_py_args) in p[1]]
    p_fastapi = [p[0] for p in pids if " ".join(fastapi_py_args) in p[1]]

    to_check = [p_shimbun, p_piper, p_fastapi]

    dup_procs = False
    sent_sigterm = []

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

                dup_procs = True
                sent_sigterm.append(pid)

    if dup_procs:
        time.sleep(10)
        for pid in sent_sigterm:
            try:
                os.kill(pid, signal.SIGKILL)
            except:
                pass
        sys.exit()

    time.sleep(1)

