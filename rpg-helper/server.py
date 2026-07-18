#!/usr/bin/env python3
"""
rpg_lsd_io controller

This script does the following:

* 1.  Creates a SQLite database with the required tables.
* 2.  Performs the “user login” flow:
        – launches `zbarcam` and parses a line like
              QR-Code:105 <base64‑signature>
        – reads the signing key from `credentials.py`
        – validates the signature
        – inserts/updates the user and login tables
        – runs the required `./dist/rpg_lsd_io` write command.
* 3.  Process “progression” signal from the Game.
        – every 100 ms reads from `./dist/rpg_lsd_io read_rpg_var_lgs … 99 1`
        – when the last output line is `1` starts a 2500 ms timer
        – after the timer runs `... 99 100` and parses the result
        – if the first number is `2` updates the progression table
        – if the first number is `3` triggers the data‑input stage
* 4.  Handles the optional data‑input stage
        – launches `zbarcam` and parses a line like
              QR-Code:<data1> <data2> … <dataN> <signature>
        – validates the signature
        – runs the required `./dist/rpg_lsd_io` write command
"""

from __future__ import annotations

import base64
import hashlib
import os
import queue
import re
import sqlite3
import subprocess
import sys
import threading
import time
import datetime
from pathlib import Path
from typing import Any, Dict, Iterable, List, Literal, Optional, Tuple
from queue import Queue
import os

os.chdir(Path(__file__).parent.absolute())

# --------------------------------------------------------------------------- #
# Configuration
# --------------------------------------------------------------------------- #

# Location of the SQLite file – adjust if you want it elsewhere
DB_PATH = Path("rpg_lsd_io.db")

# Where the binary lives – adjust as necessary
RPG_BIN = Path("./dist/rpg_lsd_io")

# The maximum amount of data that can be supplied by the user
MAX_DATA = 10

# The sync interval – 50 ms
SYNC_WINDOW = 0.05  # seconds

# The timeout for the ping → read‑100 loop
WRITE_WINDOW = 0.5  # seconds

RPG_TIMEOUT_WINDOW = 10

# prevents immense amount of zbarcam launching attempt due to camera device blocking etc.
QR_APP_WINDOW = 2.0

# --------------------------------------------------------------------------- #
#  Helper functions
# --------------------------------------------------------------------------- #

def debug(msg: str) -> None:
    """Print a debug line if the script is run interactively."""
    if sys.stdout.isatty():
        print(f"[DEBUG] {msg}", file=sys.stderr)


def run_cmd(
    *args: str,
    capture_output: bool = True,
    cwd: Optional[Path] = None,
) -> subprocess.CompletedProcess:
    """Run a subprocess and return the completed process."""
    if cwd is None:
        cwd = Path.cwd()
    debug(f"Running command: {args}")
    return subprocess.run(
        args,
        cwd=cwd,
        capture_output=capture_output,
        text=True,
        check=False,
    )


# --------------------------------------------------------------------------- #
#  Database layer
# --------------------------------------------------------------------------- #

class DB:
    """Thin wrapper around a SQLite connection."""

    def __init__(self, path: Path) -> None:
        self.conn = sqlite3.connect(path, detect_types=sqlite3.PARSE_DECLTYPES)
        self.conn.row_factory = sqlite3.Row
        self._create_tables()

    # --------------------------------------------------------------------- #
    def _create_tables(self) -> None:
        """Create the four tables if they do not exist."""
        cur = self.conn.cursor()
        cur.executescript(
            """
            CREATE TABLE IF NOT EXISTS users (
                user_id       INTEGER PRIMARY KEY,
                creation_date TEXT NOT NULL,
                current_progression INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS logins (
                user_id INTEGER NOT NULL,
                login_date TEXT NOT NULL,
                progression INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS logouts (
                user_id INTEGER NOT NULL,
                login_date TEXT NOT NULL,
                progression INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS progressions (
                user_id INTEGER NOT NULL,
                progression_date TEXT NOT NULL,
                progression INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS choices (
                user_id INTEGER NOT NULL,
                choice_date TEXT NOT NULL,
                progression INTEGER NOT NULL,
                details TEXT NOT NULL
            );
            """
        )
        self.conn.commit()

    # --------------------------------------------------------------------- #
    def get_user(self, user_id: int) -> Optional[sqlite3.Row]:
        """Return a user row or None."""
        cur = self.conn.cursor()
        cur.execute(
            "SELECT * FROM users WHERE user_id = ?", (user_id,)
        )
        return cur.fetchone()

    # --------------------------------------------------------------------- #
    def create_user(self, user_id: int) -> None:
        """Create a brand‑new user."""
        now = datetime.datetime.now(datetime.timezone.utc).isoformat()
        cur = self.conn.cursor()
        cur.execute(
            "INSERT INTO users(user_id,creation_date,current_progression) "
            "VALUES (?,?,?)",
            (user_id, now, 1),
        )
        self.conn.commit()

    # --------------------------------------------------------------------- #
    def update_user_progression(self, user_id: int, progression: int) -> None:
        """Update the `current_progression` column."""
        cur = self.conn.cursor()
        cur.execute(
            "UPDATE users SET current_progression = ? WHERE user_id = ?",
            (progression, user_id),
        )
        self.conn.commit()

    # --------------------------------------------------------------------- #
    def insert_login(self, user_id: int, progression: int) -> None:
        """Insert a login record."""
        now = datetime.datetime.now(datetime.timezone.utc).isoformat()
        cur = self.conn.cursor()
        cur.execute(
            "INSERT INTO logins(user_id,login_date,progression) "
            "VALUES (?,?,?)",
            (user_id, now, progression),
        )
        self.conn.commit()

    def insert_logout(self, user_id: int, progression: int) -> None:
        """Insert a logout record."""
        now = datetime.datetime.now(datetime.timezone.utc).isoformat()
        cur = self.conn.cursor()
        cur.execute(
            "INSERT INTO logouts(user_id,login_date,progression) "
            "VALUES (?,?,?)",
            (user_id, now, progression),
        )
        self.conn.commit()

    # --------------------------------------------------------------------- #
    def insert_progression(self, user_id: int, progression: int) -> None:
        """Insert a progression record."""
        now =  datetime.datetime.now(datetime.timezone.utc).isoformat()
        cur = self.conn.cursor()
        cur.execute(
            "INSERT INTO progressions(user_id,progression_date,progression) "
            "VALUES (?,?,?)",
            (user_id, now, progression),
        )
        self.conn.commit()

    # --------------------------------------------------------------------- #
    def insert_choice(
        self,
        user_id: int,
        progression: int,
        details: str,
    ) -> None:
        """Insert a choice record."""
        now =  datetime.datetime.now(datetime.timezone.utc).isoformat()
        cur = self.conn.cursor()
        cur.execute(
            "INSERT INTO choices(user_id,choice_date,progression,details) "
            "VALUES (?,?,?,?)",
            (user_id, now, progression, details),
        )
        self.conn.commit()


# --------------------------------------------------------------------------- #
#  Credentials handling
# --------------------------------------------------------------------------- #

def read_signing_key(path: Path = Path("credentials.py")) -> str:
    """Return the value of the global variable SIGNING_KEY."""
    ns: Dict[str, Any] = {}
    with open(path, encoding="utf-8") as f:
        exec(compile(f.read(), str(path), "exec"), ns)
    if "SIGNING_KEY" not in ns:
        raise RuntimeError("SIGNING_KEY not found in credentials.py")
    return str(ns["SIGNING_KEY"])


# --------------------------------------------------------------------------- #
#  QR‑Code parsing & validation
# --------------------------------------------------------------------------- #

def parse_qr_code_line(line: str) -> Tuple[int, str]:
    """
    Return (user_id, signature_base64) from a line like:

        QR-Code:105 <base64‑sequence>
    """
    m = re.search(r"QR-Code:(\d+)\s+([A-Za-z0-9+/=]+)", line)
    if not m:
        raise ValueError(f"Line does not match QR‑Code pattern: {line!r}")
    return int(m.group(1)), m.group(2)


def verify_signature(
    data: str,
    signature_b64: str,
    signing_key: str,
    purpose: str,
) -> bool:
    """
    Verify that the base64‑encoded SHA‑256 of the string

        <SIGNING_KEY>/<purpose>/<data>

    matches the supplied signature.
    """
    text = f"{signing_key}/{purpose}/{data}".encode("utf-8")
    sha = hashlib.sha256(text).digest()
    expected_b64 = base64.b64encode(sha).decode("ascii")
    debug(f"Signature check: {expected_b64 == signature_b64}")
    return expected_b64 == signature_b64


# --------------------------------------------------------------------------- #
#  RPG command helpers
# --------------------------------------------------------------------------- #

def rpg_write_generic(
    data: list[int],
    target: int = 199
) -> None:
    """
    Run the command:
        rpg_write_generic ([1, <user id>, <current progression>])
    Translates to:
        ./dist/rpg_lsd_io write_rpg_var_lgs <file> <file> 199 3 [ 1 <user_id> <current_progression> ]
    """
    args = [
        str(RPG_BIN),
        "write_rpg_var_lgs",
        "games/default/Save.lgs",
        "games/default/Save.lgs",
        str(target),
        str(len(data)),
        f"[ {" ".join([str(i) for i in data])} ]"
    ]
    run_cmd(*args)


def rpg_write_error(code: int) -> None:
    """Run the error command, e.g. 199 1 [ 10001 ]."""
    rpg_write_generic([10000 + code])

def rpg_read_generic(count: int, target = 99) -> list[int]:
    """
    Run the command

        ./dist/rpg_lsd_io read_rpg_var_lgs <file> 99 1

    and return all non‑empty, non‑debug lines.
    """
    out = run_cmd(
        str(RPG_BIN),
        "read_rpg_var_lgs",
        "games/default/Save.lgs",
        str(target),
        str(count),
    )
    # Split into lines, strip, drop empty & debug lines
    results = [
        l.strip()
        for l in out.stdout.splitlines()
        if l.strip() and not l.strip().startswith("Debug:")
    ]
    if len(results) > 0:
        return [int(x) for x in results[0].split(' ')]
    else:
        return []


# --------------------------------------------------------------------------- #
#  QR Processing Flow
# --------------------------------------------------------------------------- #

current_qr_state: Literal["login"] | Literal["data-input"] | Literal[""] = "login"

# user_id, error_code, error_msg
login_queue: Queue[tuple[int | None, int | None, str | None]] = Queue(maxsize=1)

# data, error_code, error_msg
data_input_queue: Queue[tuple[list[int] | None, int | None, str | None]] = Queue(maxsize=1)


def process_qr_login(data: str, signing_key: str) -> bool:
    # 2. find a matching QR‑code line
    user_id: int
    signature_b64: str
    for l in data.splitlines():
        try:
            user_id, signature_b64 = parse_qr_code_line(l)
            break
        except ValueError:
            continue
    else:
        # No QR‑code line
        login_queue.put_nowait((None, 1, "No QR‑code line found while logging in"))
        return False

    # 3. validate
    if not verify_signature(str(user_id), signature_b64, signing_key, "ikiteikou_os_v0.0002_aone_cards"):
        login_queue.put_nowait((None, 1, f"Invalid QR‑code signature for user {user_id}"))
        return False

    login_queue.put_nowait((user_id, None, None))
    return True

def process_qr_data_input(
    qr_data: str,
    signing_key: str,
) -> bool:
    """
    Handle the data‑input stage that is triggered when the last read
    command returns a line that starts with the number 3.
    """
    data: List[int] = []
    signature_b64: str
    for l in qr_data.splitlines():
        # Expected format:
        #   QR-Code:2 5 3 7 <signature>
        m = re.search(r"QR-Code:((\d+ )+)([A-Za-z0-9+/=]+)", l)
        if not m:
            continue

        # Split the data part
        data_part = m.group(1)
        data = [int(x) for x in data_part.split()]
        signature_b64 = m.group(3)
        break
    else:
        data_input_queue.put_nowait((None, 2, "No QR‑code data line found while data‑input"))
        return False

    if len(data) > MAX_DATA:
        data_input_queue.put_nowait((None, 2, "Too many data provided"))
        return False

    # Verify the signature
    path = "/".join( [str(x) for x in data])
    if not verify_signature(path, signature_b64, signing_key, "ikiteikou_os_v0.0002_data_input"):
        data_input_queue.put_nowait((None, 2, "Invalid data‑input QR‑code signature"))
        return False

    data_input_queue.put_nowait((data, None, None))
    return True

def qr_reader(signing_key: str):
    """
    Perform the login flow or data insersion flow.

    Returns a tuple (user_id, current_progression).
    """
    while True:
        time.sleep(QR_APP_WINDOW)

        # 1. launch zbarcam
        out = run_cmd("zbarcam", "-1", "--nodisplay")
        try:
            if current_qr_state == "login":
                process_qr_login(out.stdout, signing_key)
            elif current_qr_state == "data-input":
                process_qr_data_input(out.stdout, signing_key)
            else:
                debug("qr code provided but no use")
        except queue.Full:
            debug("Full queue; discarding the newest input")

# --------------------------------------------------------------------------- #
#  Queue misc
# --------------------------------------------------------------------------- #

def flush_queue(q: Queue):
    # Flush the queue
    while not q.empty():
        try:
            q.get_nowait()
            q.task_done()
        except queue.Empty:
            break

# --------------------------------------------------------------------------- #
#  Progression loop
# --------------------------------------------------------------------------- #

def sanitize_queues():
    if current_qr_state == "":
        flush_queue(login_queue)
        flush_queue(data_input_queue)
    elif current_qr_state == "login":
        flush_queue(data_input_queue)
    elif current_qr_state == "data-input":
        flush_queue(login_queue)

def progression_loop(db: DB, signing_key: str) -> None:
    """
    Run the continuous progression loop.
    """
    global current_qr_state
    user_id: None | int = None
    current_progression: None | int = None
    ping_start: None | float = None
    processed = True

    while True:
        # 1. read the ping / status
        data = rpg_read_generic(1, 98)

        if len(data) == 0:
            time.sleep(SYNC_WINDOW)
            continue

        # We have a ping
        if data[0] == 1 and processed == True:
            ping_start = time.time()
            rpg_write_generic([0], 98)
            processed = False
        
        if ping_start is not None and (time.time() - ping_start >= RPG_TIMEOUT_WINDOW):
            raise RuntimeError("Lost connection from RPG!")

        # 1a. if we have a ping and we have waited enough
        if ping_start is not None and processed == False and (time.time() - ping_start >= WRITE_WINDOW):
            processed = True
            sanitize_queues()

            if user_id is None:
                # ===================================
                # BEFORE LOGIN
                # ===================================

                # Get login data, erase other queue
                current_qr_state = "login"

                try:
                    data = login_queue.get_nowait()
    
                    if data[0] is None:
                        if data[1] is not None:
                            rpg_write_error(data[1])
                        else:
                            raise RuntimeError("Error code is not provided on login failure!")
                    else:
                        user_id = data[0]
            
                        # 4. create / update DB
                        user = db.get_user(user_id)
                        if user is None:
                            db.create_user(user_id)
                            current_progression = 1
                        else:
                            current_progression = user["current_progression"]

                        if current_progression is None:
                            raise RuntimeError("SQL data type error; perhaps Database structure is malformed")
                    
                        # 5. log the login
                        db.insert_login(user_id, current_progression)
                    
                        # 6. write the required command
                        rpg_write_generic([1, user_id, current_progression])
                        
                        # On success Login, disable qr input
                        current_qr_state = ""
    
                    login_queue.task_done()
                except queue.Empty:
                    pass
            else:
                # ===================================
                # AFTER_LOGIN
                # ===================================

                if current_qr_state == "data-input":
                    try:
                        data = data_input_queue.get_nowait()
                        if data[0] is None:
                            if data[1] is not None:
                                rpg_write_error(data[1])
                            else:
                                raise RuntimeError("Error code is not provided on data input!")
                        elif current_progression is not None:
                            db.insert_choice(user_id, current_progression, f"(data input) {data[0]}")
                            rpg_write_generic([2, user_id, current_progression, *data[0]])
                        else:
                            raise RuntimeError("something wrong happened")
                        data_input_queue.task_done()
                        current_qr_state = ""
                    except queue.Empty:
                        pass


                # 2. read the 100-199 (Internally, 99-198)
                out = rpg_read_generic(100)
                debug("Command ID from RPG : %s" % out[0])
    
                # The first number is the *command* indicator
                cmd = out[0]
                if cmd == 2: # Progression command
                    # The third number is the next progression
                    if user_id != out[1]:
                        raise RuntimeError("User ID desync from RPG!!")
                    next_prog = out[2]
                    # Update DB
                    db.insert_progression(user_id, next_prog)
                    # Also update the current progression in the users table
                    db.update_user_progression(user_id, next_prog)
                    current_progression = next_prog
                elif cmd == 3: # Data-Input command
                    if user_id != out[1]:
                        raise RuntimeError("User ID desync from RPG!!")
                    # data‑input required
                    current_qr_state = "data-input"
                elif cmd == 4: # Logout command
                    if user_id != out[1]:
                        raise RuntimeError("User ID desync from RPG!!")
                    debug(f"{user_id} Logout")
                    db.insert_logout(user_id, current_progression or 0)
                    user_id = None
                    current_progression = None
                elif cmd >= 10000:
                    raise RuntimeError(f"Desync from RPG with Error Code: {cmd}")

        # 2.   100ms loop – re‑run
        time.sleep(SYNC_WINDOW)


# --------------------------------------------------------------------------- #
#  Main entry point
# --------------------------------------------------------------------------- #

def main() -> None:
    """Entry point of the script."""
    db = DB(DB_PATH)
    signing_key = read_signing_key()

    thread_qr = threading.Thread(target=qr_reader, args=(signing_key,))
    thread_qr.start()

    # Start the progression loop – this will never return
    try:
        progression_loop(db, signing_key)
    except KeyboardInterrupt:
        print("\n[INFO] Stopping…", file=sys.stderr)
        sys.exit(0)


# --------------------------------------------------------------------------- #
#  Guard
# --------------------------------------------------------------------------- #

if __name__ == "__main__":
    main()

