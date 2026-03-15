#!/usr/bin/env python3
"""
FastAPI TTS dispatcher that replaces the old speech‑dispatcher‑helper script.
"""

import os
import subprocess
from pathlib import Path
from typing import List
import time

import requests
from fastapi import FastAPI, HTTPException, status
from pydantic import BaseModel

app = FastAPI(title="Speech Dispatcher API")

# ------------------------------------------------------------------
# Configuration – can be overridden by an env var if you want.
# ------------------------------------------------------------------
SPEECH_INSTALL_PREFIX = Path(__file__).resolve().parent.parent / "external-apps"

# Paths we need
OPEN_JTALK_BIN = SPEECH_INSTALL_PREFIX / "open_jtalk" / "bin" / "open_jtalk"
DICTIONARY_DIR = SPEECH_INSTALL_PREFIX / "open_jtalk_dic_utf_8"
MMD_MODEL_DIR = SPEECH_INSTALL_PREFIX / "MMDAgent_voices"

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
@app.post("/api/say")
async def say(req: SayRequest):
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
        model_path = MMD_MODEL_DIR / f"{req.voice}.htsvoice"
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
            "-r", "1",            # rate – default
            "-fm", "24",           # pitch – default
        ]

        # -------------------------------------------------------------
        # Run open_jtalk → mpv
        # -------------------------------------------------------------
        # mpv reads from stdin when we pass '-' as the file name
        mpv_cmd = ["mpv", "-"]

        # We start open_jtalk, capture its stdout and feed it to mpv
        try:
            # open_jtalk → mpv (both as subprocesses)
            open_proc = subprocess.Popen(open_jtalk_cmd, stdout=subprocess.PIPE, text=True)
            mpv_proc = subprocess.Popen(mpv_cmd, stdin=open_proc.stdout, text=True)

            # Close open_jtalk's stdout so mpv gets EOF when done
            open_proc.stdout.close()

            # Wait for both to finish
            time.sleep(0.3)

        except Exception as exc:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail=f"Error running open_jtalk: {exc}"
            )

    else:   # libritts_r-medium
        # -------------------------------------------------------------
        # POST the text to the local Piper-TTS server
        # -------------------------------------------------------------
        payload = {"text": req.text}
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
        mpv_cmd = ["mpv", "-"]
        try:
            mpv_proc = subprocess.Popen(mpv_cmd, stdin=subprocess.PIPE, text=True)

            # Stream the response directly to mpv
            for chunk in r.iter_content(chunk_size=4096):
                if chunk:
                    mpv_proc.stdin.write(chunk.decode(errors="ignore"))

            # Close the pipe and wait
            mpv_proc.stdin.close()
            mpv_proc.wait()

        except Exception as exc:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail=f"Error playing LibrtTS output: {exc}"
            )

    # We only return after the audio finished – the caller can ignore the body
    return {"status": "ok"}

# ------------------------------------------------------------------
# 5️⃣  Run via: uvicorn this_module:app
# ------------------------------------------------------------------
if __name__ == "__main__":
    import uvicorn
    uvicorn.run("this_module:app", host="0.0.0.0", port=8000, reload=True)

