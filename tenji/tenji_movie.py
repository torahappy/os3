#!/bin/python3

# douga lvl.1 watcher

import subprocess
import sys
import time
from typing import TypedDict
import os
import atexit
import re
import signal
import time

DEFAULT_SINK = 'alsa_output.pci-0000_e5_00.6.analog-stereo'
SINK_VOLUME = '80%'

def check_pulseaudio():
    r = subprocess.run(["pactl", "get-default-sink"], stderr=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    if not DEFAULT_SINK in r.stdout:
        subprocess.run(["pactl", "set-default-sink", DEFAULT_SINK], stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
    time.sleep(1.0)
    r = subprocess.run(["pactl", "get-sink-volume", "@DEFAULT_SINK@"], stderr=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    # TODO: mono/stereo
    if not re.match(r'.*%s.*' % SINK_VOLUME, r.stdout):
        subprocess.run(["pactl", "set-sink-volume", "@DEFAULT_SINK@", SINK_VOLUME], stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)

try:
    display_pos = 0
    number_of_displays = 2
    wmctrl_result = subprocess.run(["/usr/bin/wmctrl", "-d"], capture_output=True, text=True)
    m = [re.match(r"^\d+\s+\*\s+DG:\s+(\d+)x(\d+)", l) for l in wmctrl_result.stdout.split('\n')]
    m = [l for l in m if l is not None]
    if len(m) > 0:
        m = m[0]
        whole_w = int(m[1])
        whole_h = int(m[2])
        if whole_w > whole_h:
            single_w = whole_w // number_of_displays
            single_h = whole_h
            pos_x = single_w // 2 + display_pos * single_w
            pos_y = single_h // 2
            subprocess.run(["xdotool", "mousemove", str(pos_x), str(pos_y)])
            time.sleep(0.1)
except Exception as e:
    print(str(e))

class MyState(TypedDict):
    started: bool
    process_douga: subprocess.Popen | None
    to_check_startup_flag: list[bool]

def exit_func():
    to_terminate = [my_state['process_douga']]
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
        "process_douga": None,
        "to_check_startup_flag": [False],
    }

basepath = os.path.dirname(os.path.abspath(__file__))

src_path = os.path.join(
            basepath, "..", "os3rust"
        )

my_state: MyState = initial_state()
import shutil

while True:
    check_pulseaudio()

    if not my_state['started']:
        my_state['started'] = True
        try:
            shutil.rmtree(os.path.join(os.path.expanduser('~'), '.config', 'vlc'))
        except Exception as e:
            print(e)
        my_state['process_douga'] = subprocess.Popen(["vlc", "--loop", "--fullscreen", "--no-video-title-show", "--no-qt-privacy-ask", "/douga/douga.mov"], stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)

    pids_raw = subprocess.run(["ps", "-Ao", "pid,args"], capture_output=True, text=True)
    pids = [re.match(r'^\s*(\d+)\s*(.*)$', l) for l in pids_raw.stdout.split("\n")]
    pids = [(int(p[1]), str(p[2])) for p in pids if p is not None]

    voice_mpv = [p[0] for p in pids if '/douga/douga.mov' in p[1]]

    to_check = [voice_mpv]

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

