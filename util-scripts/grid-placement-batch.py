#!/usr/bin/env python3

"""
grid-placement-svg-batch.py

This script batches a set of cell SVGs and feeds them into
`grid-placement-svg.py` so that each batch is written to a separate
output SVG file.

Usage:

    python grid-placement-svg-batch.py \
        --base  base_file.svg \
        --cells-dir  cells_dir \
        --out-dir  out_dir \
        --interval-w  10.0 \
        --interval-h  10.0 \
        --start-x   0.0 \
        --start-y   0.0 \
        --num-each   10

The above example will:

1.  Take the files *cells_dir/1.svg … cells_dir/10.svg*,
    pass them to `grid-placement-svg.py` together with the
    supplied base file and the spacing/starting coordinates,
    and write the resulting SVG to *out_dir/1.svg*.

2.  Then take the next ten files (*cells_dir/11.svg … cells_dir/20.svg*),
    write the result to *out_dir/2.svg*, and so on.

The script will create the output directory if it does not already
exist and will exit with a non‑zero status if any of the commands
fails.

Assisted by: gpt-oss-20b 
"""

import argparse
import subprocess
import sys
from pathlib import Path

__all__ = ["main"]


def _collect_svg_files(cells_dir: Path) -> list[Path]:
    """Return a sorted list of all *.svg files in *cells_dir*."""
    return sorted(cells_dir.glob("*.svg"))


def _run_batch(
    base_path: Path,
    batch: list[Path],
    out_path: Path,
    interval_w: float,
    interval_h: float,
    start_x: float,
    start_y: float,
) -> None:
    """Run a single batch of grid-placement-svg.py."""
    cmd = [
        sys.executable,
        "grid-placement-svg.py",
        "--base",
        str(base_path),
        "--cells",
    ] + [str(p) for p in batch] + [
        "--out",
        str(out_path),
        "--interval_w",
        str(interval_w),
        "--interval_h",
        str(interval_h),
        "--start_x",
        str(start_x),
        "--start_y",
        str(start_y),
    ]

    # Print the command for debugging purposes
    print(f"Running batch -> {out_path}")
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        print(f"[ERROR] grid-placement-svg.py failed for {out_path}", file=sys.stderr)
        print(result.stdout, file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        sys.exit(1)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Batch‑run grid-placement-svg.py over a directory of SVG cells."
    )
    parser.add_argument(
        "--base",
        required=True,
        help="Base SVG file to be used by grid-placement-svg.py",
    )
    parser.add_argument(
        "--cells-dir",
        required=True,
        help="Directory that contains all the cell SVG files",
    )
    parser.add_argument(
        "--out-dir",
        required=True,
        help="Directory where the output SVGs will be written",
    )
    parser.add_argument(
        "--interval-w",
        required=True,
        type=float,
        help="Horizontal spacing between cells",
    )
    parser.add_argument(
        "--interval-h",
        required=True,
        type=float,
        help="Vertical spacing between cells",
    )
    parser.add_argument(
        "--start-x",
        required=True,
        type=float,
        help="X coordinate of the first cell",
    )
    parser.add_argument(
        "--start-y",
        required=True,
        type=float,
        help="Y coordinate of the first cell",
    )
    parser.add_argument(
        "--num-each",
        required=True,
        type=int,
        help="Number of cells to process per batch",
    )

    args = parser.parse_args()

    base_path = Path(args.base).expanduser().resolve()
    cells_dir = Path(args.cells_dir).expanduser().resolve()
    out_dir = Path(args.out_dir).expanduser().resolve()

    # Sanity checks
    if not base_path.is_file():
        print(f"[ERROR] Base file '{base_path}' does not exist", file=sys.stderr)
        sys.exit(1)

    if not cells_dir.is_dir():
        print(f"[ERROR] Cells directory '{cells_dir}' does not exist", file=sys.stderr)
        sys.exit(1)

    out_dir.mkdir(parents=True, exist_ok=True)

    svg_files = _collect_svg_files(cells_dir)

    if not svg_files:
        print(f"[ERROR] No *.svg files found in '{cells_dir}'", file=sys.stderr)
        sys.exit(1)

    # Split into batches
    num_each = args.num_each
    batches = [
        svg_files[i : i + num_each]
        for i in range(0, len(svg_files), num_each)
    ]

    for idx, batch in enumerate(batches, start=1):
        out_path = out_dir / f"{idx}.svg"
        _run_batch(
            base_path=base_path,
            batch=batch,
            out_path=out_path,
            interval_w=args.interval_w,
            interval_h=args.interval_h,
            start_x=args.start_x,
            start_y=args.start_y,
        )


if __name__ == "__main__":
    main()

