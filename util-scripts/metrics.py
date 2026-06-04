#!/usr/bin/env python3
"""
image_metrics.py

Usage:
    python image_metrics.py --source <directory> --output <file.json>

The script walks through every file in <directory>, tries to open it
with Pillow, extracts its width and height and writes a JSON file with
the mapping {filename: [width, height]}.

Dependencies:
    pip install Pillow
"""

import argparse
import json
import os
from pathlib import Path
from typing import Dict, List

try:
    from PIL import Image
except ImportError:
    raise ImportError("Pillow is required – install it with `pip install Pillow`")

# --------------------------------------------------------------------------- #
# Helper: try to open an image and return (width, height) or None
# --------------------------------------------------------------------------- #
def _size_of(file_path: Path) -> List[int] | None:
    try:
        with Image.open(file_path) as img:
            return [img.width, img.height]
    except Exception:
        # Anything that Pillow can't open (or is not an image) will be ignored
        return None

# --------------------------------------------------------------------------- #
# Main logic
# --------------------------------------------------------------------------- #
def main() -> None:
    parser = argparse.ArgumentParser(
        description="Collect width & height of every image in a folder."
    )
    parser.add_argument(
        "--source",
        required=True,
        type=str,
        help="Path to the folder that contains the images."
    )
    parser.add_argument(
        "--output",
        required=True,
        type=str,
        help="Path to the JSON file that will contain the results."
    )
    args = parser.parse_args()

    src = Path(args.source).resolve()
    out = Path(args.output).resolve()

    if not src.is_dir():
        parser.error(f"Source path {src} is not a directory")

    # Gather all files – we rely on Pillow to filter out non‑images
    metrics: Dict[str, List[int]] = {}

    for file in src.iterdir():
        if file.is_file():
            size = _size_of(file)
            if size:
                # Store the filename *only* (not the full path) as required
                metrics[file.name] = size

    # Write the JSON output
    out.parent.mkdir(parents=True, exist_ok=True)  # make sure the folder exists
    with out.open("w", encoding="utf-8") as fp:
        json.dump(metrics, fp, indent=4, sort_keys=True)

    print(f"Collected metrics for {len(metrics)} images → {out}")

# --------------------------------------------------------------------------- #
if __name__ == "__main__":
    main()

