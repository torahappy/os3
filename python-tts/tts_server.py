#!/usr/bin/env python3
"""
FastAPI TTS dispatcher that replaces the old speech‑dispatcher‑helper script.

python -m piper.http_server -m ./models/en_US-libritts_r-medium.onnx --port 5000
"""

import os
import subprocess
from pathlib import Path
from typing import List
import time
import asyncio

import requests
from fastapi import BackgroundTasks, FastAPI, HTTPException, status
from pydantic import BaseModel

app = FastAPI(title="Speech Dispatcher API")

# ------------------------------------------------------------------
# Configuration – can be overridden by an env var if you want.
# ------------------------------------------------------------------
SPEECH_INSTALL_PREFIX = Path(__file__).resolve().parent.parent / "external-apps"
TRAIN_DATA_PREFIX = Path(__file__).resolve().parent.parent / "external-sources"

# Paths we need
OPEN_JTALK_BIN = SPEECH_INSTALL_PREFIX / "open_jtalk" / "bin" / "open_jtalk"
DICTIONARY_DIR = TRAIN_DATA_PREFIX.glob("open_jtalk_dic*").__next__()
MMD_MODEL_DIR = TRAIN_DATA_PREFIX.glob("mmdagent_voice*").__next__()

# ------------------------------------------------------------------
# 1️⃣  Voices that the API knows about
# ------------------------------------------------------------------
ALLOWED_VOICES = ["takumi_happy", "libritts_r-medium"]

# ------------------------------------------------------------------
# 2️⃣  Pydantic model for the POST /api/say body
# ------------------------------------------------------------------
class SayRequest(BaseModel):
    voice: str
    text: str

# ------------------------------------------------------------------
# 3️⃣  /api/voices – just return the list
# ------------------------------------------------------------------
@app.get("/api/voices")
async def get_voices() -> List[str]:
    """Return the list of supported voice IDs."""
    return ALLOWED_VOICES


# ------------------------------------------------------------------
# 4️⃣  /api/say – main handler
# ------------------------------------------------------------------

def tasks_openjtalk(open_jtalk_cmd: list[str], mpv_cmd: list[str], text: str):
    open_proc = subprocess.Popen(open_jtalk_cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    mpv_proc = subprocess.Popen(mpv_cmd, stdin=open_proc.stdout)

    # Close open_jtalk's stdout so mpv gets EOF when done
    open_proc.stdin.write(text)
    open_proc.stdin.close()
    open_proc.stdout.close()
    open_proc.wait()
    mpv_proc.wait()


def tasks_piper(mpv_cmd: list[str], text: str):

    # -------------------------------------------------------------
    # POST the text to the local Piper-TTS server
    # -------------------------------------------------------------
    payload = {"text": text}
    try:
        r = requests.post(
            "http://localhost:5000",
            json=payload,
            stream=True,
            timeout=10,
        )
        r.raise_for_status()
    except Exception as exc:
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail=f"Error contacting Piper-TTS: {exc}"
        )

    # -------------------------------------------------------------
    # Pipe the streamed audio to mpv
    # -------------------------------------------------------------
    try:
        mpv_proc = subprocess.Popen(mpv_cmd, stdin=subprocess.PIPE)

        # Stream the response directly to mpv
        for chunk in r.iter_content(chunk_size=4096):
            if chunk:
                mpv_proc.stdin.write(chunk)

        # Close the pipe and wait
        mpv_proc.stdin.close()
        mpv_proc.wait()

    except Exception as exc:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error playing LibrtTS output: {exc}"
        )

@app.post("/api/say")
async def say(req: SayRequest, tasks: BackgroundTasks):
    """
    Speak the supplied text with the requested voice.

    * For `takumi_happy` we run open_jtalk and pipe its stdout to mpv.
    * For `libritts_r-medium` we POST to http://localhost:5000 and pipe the
      response raw bytes to mpv.
    """
    if req.voice not in ALLOWED_VOICES:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Unsupported voice: {req.voice}. Allowed: {ALLOWED_VOICES}"
        )

    if req.voice == "takumi_happy":
        # -------------------------------------------------------------
        # Build the command for open_jtalk
        # -------------------------------------------------------------
        model_path = MMD_MODEL_DIR.rglob(f"**/{req.voice}.htsvoice").__next__()
        if not model_path.is_file():
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail=f"Model file not found: {model_path}"
            )

        # open_jtalk command
        open_jtalk_cmd = [
            str(OPEN_JTALK_BIN),
            "-x", str(DICTIONARY_DIR),
            "-m", str(model_path),
            "-r", "1",
            "-fm", "20",
            "-ow", "/dev/stdout"
        ]

        # -------------------------------------------------------------
        # Run open_jtalk → mpv
        # -------------------------------------------------------------
        # mpv reads from stdin when we pass '-' as the file name
        mpv_cmd = ["mpv", "-"]

        # We start open_jtalk, capture its stdout and feed it to mpv
        try:
            # open_jtalk → mpv (both as subprocesses)
            tasks.add_task(tasks_openjtalk, open_jtalk_cmd, mpv_cmd, req.text)

        except Exception as exc:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail=f"Error running open_jtalk: {exc}"
            )

    else:   # libritts_r-medium
        mpv_cmd = ["mpv", "-"]
        tasks.add_task(tasks_piper, mpv_cmd, req.text)

    # We only return after the audio finished – the caller can ignore the body
    return {"status": "ok"}

