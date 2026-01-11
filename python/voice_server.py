import asyncio
import logging
import atexit
from asyncio.subprocess import Process
from typing import Any, Union
from fastapi import FastAPI
logger = logging.getLogger('uvicorn')
from typing import TypedDict
import json

class GlobalData(TypedDict):
    proc: Process | None
    data_queue: list[list[int | list[int]]]

from contextlib import asynccontextmanager

global_data: GlobalData = {
    'proc': None,
    'data_queue': []
}

async def initialize_proc():
    if global_data['proc'] is None:
        proc = await asyncio.create_subprocess_exec(
            './venv/bin/python', 'voice.py',
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            limit=1024**2
        )
        global_data['proc'] = proc

@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    アプリケーションのライフサイクルを管理するコンテキストマネージャー
    """
    logger.info("Application is starting up...")
    yield
    if global_data['proc'] is not None:
        logger.info('terminate the child process')
        global_data['proc'].terminate()
    logger.info("Application is shutting down...")
    logger.info("Cleanup complete.")

app = FastAPI(lifespan=lifespan)

@app.get("/")
async def read_root():
    return {"Hello": "World"}

@app.get("/start")
async def read_start():
    await initialize_proc()
    return {"status": "OK"}

@app.get("/readlines")
async def read_read():
    await initialize_proc()
    proc = global_data['proc']
    if proc is not None and proc.stdout is not None:
        js = []
        while proc.stdout._buffer:
            b = await proc.stdout.readline()
            j = json.loads(b.decode())
            js.append(j)
        return {"status": js}
    else:
        return {"status": "ERROR", "error": "Process not started yet"}
