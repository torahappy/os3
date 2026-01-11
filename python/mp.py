from re import sub
from matplotlib.animation import FuncAnimation
import matplotlib.pyplot as plt
import random
import subprocess
import os
import asyncio
import fcntl

p1 = subprocess.Popen(['./venv/bin/python', 'voice.py'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
flag = fcntl.fcntl(p1.stdout.fileno(), fcntl.F_GETFL)
fcntl.fcntl(p1.stdout.fileno(), fcntl.F_SETFL, flag | os.O_NONBLOCK)

# initial data

# creating the first plot and frame
fig, ax = plt.subplots()
import json

history = []
history_2 = []
num = 20

# updates the data and graph
def update(frame):
    for l in p1.stdout.readlines():
        j = json.loads(l)
        print(j)
        if len(j[1]) > 1:
            ax.clear()
            f1 = j[1][0]
            f2 = j[1][1]
            history.append([f1, f2])

        if len(j[1]) > 3:
            ax.clear()
            f1 = j[1][2]
            f2 = j[1][3]
            history_2.append([f1, f2])

        if len(history) > num:
            history_slice = history[-num:]
            ax.scatter([i[0] for i in history_slice], [i[1] for i in history_slice])
        if len(history_2) > num:
            history_slice = history_2[-num:]
            ax.scatter([i[0] for i in history_slice], [i[1] for i in history_slice], c="r")
        plt.xlim(0,1000)
        plt.ylim(0,1000)

anim = FuncAnimation(fig, update, interval = 0)
plt.show()

import atexit

def process_exit():
    p1.terminate()

atexit.register(process_exit)
