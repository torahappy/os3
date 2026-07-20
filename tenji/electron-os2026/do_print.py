#!/usr/bin/env python3

import sys
import os
import sys
import time
import hashlib
import base64
from pathlib import Path
import subprocess

sys.path.append(str((Path(__file__).parent / "venv" / "lib" / f"python{sys.version_info[0]}.{sys.version_info[1]}" / "site-packages").absolute()))

if not 'SUDO_USER' in os.environ:
    subprocess.run(["sudo", __file__] + sys.argv[1:])
    sys.exit()

# ------------------------------------------------------------------
#  1.  Load the password
# ------------------------------------------------------------------
try:
    from credentials import password  # `credentials.py` must define a variable called `password`
except ImportError as exc:
    sys.exit(f"Could not import `password` from credentials.py: {exc}")

# ------------------------------------------------------------------
# 2.  Build the data string from the command‑line arguments
# ------------------------------------------------------------------
if len(sys.argv) < 2:
    sys.exit("Usage: python script.py <data‑component‑1> <data‑component‑2> …")

# Join the supplied arguments with '/'  →  e.g.  "foo" "bar" "baz" →  "foo/bar/baz"
data = "/".join(sys.argv[1:])

timestamp = int(time.time())

# Append the current epoch (seconds) as a new component
data += f"/{timestamp}"

# ------------------------------------------------------------------
# 3.  Create output directory
# ------------------------------------------------------------------
os.makedirs("cells", exist_ok=True)

# ------------------------------------------------------------------
# 4.  Build the string that will be hashed
# ------------------------------------------------------------------
string_to_hash = f"{password}/ikiteikou_os_v0.0002_data_input/{data}"

# ------------------------------------------------------------------
# 5.  Compute SHA‑256 (binary) and Base64‑encode it
# ------------------------------------------------------------------
sha256_digest = hashlib.sha256(string_to_hash.encode()).digest()
signature = base64.b64encode(sha256_digest).decode().rstrip("\n")

# ------------------------------------------------------------------
# 6.  Print the resulting signature
# ------------------------------------------------------------------

# with open('/tmp/aaaaaaaa', 'w') as f:
qr_data = f"{" ".join(sys.argv[1:])} {timestamp} {signature}"

from escpos.printer import Usb

p = Usb(0x28e9, 0x0289, in_ep=0, out_ep=0x03)

p.image(Path(__file__).parent.absolute() / "jouhou_a.png")

p.qr(qr_data, size=10)

p.text('\n\n\n\n')

