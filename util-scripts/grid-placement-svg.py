#!/usr/bin/env python3
"""
svg_grid.py – a tiny command‑line tool to tile “cell” SVGs onto a base SVG.

Usage
    python svg_grid.py --base base.svg --cells cell1.svg cell2.svg … \
           --out out.svg --interval_w 100 --interval_h 120 \
           --start_x 10 --start_y 20

or for a single repeating cell

    python svg_grid.py --base base.svg --single-cell cell.svg \
           --out out.svg --interval_w 100 --interval_h 120 \
           --start_x 10 --start_y 20

Return codes
    0  – success (exactly enough cells were placed)
    1  – file I/O or parsing error
    2  – more cells than the base image needed (output still produced)
    3  – fewer cells than the base image can contain (output still produced)
"""

import re
import argparse
import os
import sys
import xml.etree.ElementTree as ET
from typing import List, Tuple

# --------------------------------------------------------------------------- #
# Helper functions
# --------------------------------------------------------------------------- #

def _parse_float(value: str, name: str) -> float:
    """Parse a string to float, or raise a nice error."""
    try:
        return float(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"{name} must be a number, got '{value}'")

def _parse_path(value: str, name: str) -> str:
    """Return the path unchanged but raise an error if it doesn't exist."""
    p = os.path.abspath(value)
    if not os.path.exists(p):
        raise argparse.ArgumentTypeError(f"{name} file not found: {p}")
    return p

# --------------------------------------------------------------------------- #
# Core logic
# --------------------------------------------------------------------------- #

def load_svg(path: str) -> ET.ElementTree[ET.Element[str]]:
    """Load an SVG file and return its ElementTree."""
    try:
        tree = ET.parse(path)
        return tree
    except Exception as exc:
        raise IOError(f"Unable to read SVG '{path}': {exc}")

import copy

def copy_element(elem: ET.Element) -> ET.Element:
    """Return a deep copy of an ElementTree element."""
    c = copy.deepcopy(elem)
    if re.match(r'^\{.+?\}svg$', c.tag):
      c.tag = "svg"
    return c

def separate_unit(data: str) -> Tuple[float, str]:
    m = re.match(r'(\d*(\.\d*)?)\s*(\w*)', data)
    if m:
        return float(m[1]), m[3]
    else:
        raise ValueError("SVG Malformed Error: unit is Malformed")

def get_svg_dim(root: ET.Element) -> Tuple[float, float, float, float, float | None, float | None, str | None, str | None]:
    """
    Return (width, height) from the root element.
    Returns: (vb_width, vb_height, vb_x, vb_y, width, height, width_unit, height_unit)
    """

    vb_w, vb_h, vb_x, vb_y, width, height, w_unit, h_unit = [None] * 8

    attr_w = root.attrib.get("width")
    attr_h = root.attrib.get("height")
    if attr_h is not None and attr_w is not None:
        width, w_unit = separate_unit(attr_w)
        height, h_unit = separate_unit(attr_h)

    viewbox = root.attrib.get("viewBox")
    if viewbox:
        # viewBox is "minX minY width height"
        vb_x, vb_y, vb_w, vb_h = map(float, viewbox.split())
    else:
        if width is not None and height is not None:
            vb_x = 0.
            vb_y = 0.
            vb_w = width
            vb_h = height
        else:
            raise ValueError("SVG Parse error: No width/height metrics nor viewbox metrics!")
    return vb_w, vb_h, vb_x, vb_y, width, height, w_unit, h_unit

import math

def tile_cells(
    base_root: ET.Element,
    cell_elems: List[ET.Element],
    out_path: str,
    interval_w: float,
    interval_h: float,
    start_x: float,
    start_y: float,
    *,
    single_cell: bool = False,
) -> int:
    """
    Place cell_elems onto base_root at a grid defined by interval_w, interval_h,
    starting at (start_x, start_y).  Return the exit code:
        0 – exact fit
        2 – more cells than needed
        3 – fewer cells than needed
    The result is written to out_path.
    """
    # --------------------------------------------------------------------- #
    # Determine how many grid positions fit into the base image
    # --------------------------------------------------------------------- #
    try:
        base_w, base_h, base_x, base_y, base_svg_width, base_svg_height, base_svg_width_unit, base_svg_height_unit = get_svg_dim(base_root)
    except Exception as exc:
        raise ValueError(f"Unable to read base dimensions: {exc}")

    # How many columns and rows fit?
    ncols = math.ceil((base_w - start_x) / interval_w) if base_w >= start_x else 0
    nrows = math.ceil((base_h - start_y) / interval_h) if base_h >= start_y else 0
    total_grid = ncols * nrows

    # --------------------------------------------------------------------- #
    # If we have a single cell, duplicate it for all positions
    # --------------------------------------------------------------------- #
    if single_cell:
        if not cell_elems:
            raise ValueError("No cell provided for --single-cell")
        # duplicate the one element
        cell_elems = [copy_element(cell_elems[0]) for _ in range(total_grid)]

    # --------------------------------------------------------------------- #
    # Prepare the list of cells we will actually use
    # --------------------------------------------------------------------- #
    # If we have more cells than grid slots, we stop at total_grid
    # If fewer cells, we just run out of them (the grid will be partially filled)
    used_cells = cell_elems[:total_grid]
    used_count = len(used_cells)

    # --------------------------------------------------------------------- #
    # Build the output tree
    # --------------------------------------------------------------------- #
    # We'll wrap each cell in a <g transform="translate(...)" ...> so that
    # the original cell stays unchanged.
    for idx, (cx, cy) in enumerate(
        ((start_x + i * interval_w, start_y + j * interval_h)
         for j in range(nrows) for i in range(ncols))
    ):
        if idx >= used_count:
            break  # no more cells to place

        g = ET.Element("g")
        g.attrib["transform"] = f"translate({cx:.2f},{cy:.2f})"
        g.append(copy_element(used_cells[idx]))
        base_root.append(g)

    # --------------------------------------------------------------------- #
    # Write the file
    # --------------------------------------------------------------------- #
    try:
        ET.ElementTree(base_root).write(out_path, encoding="utf-8", xml_declaration=True)
    except Exception as exc:
        raise IOError(f"Unable to write output SVG '{out_path}': {exc}")

    # --------------------------------------------------------------------- #
    # Decide the return code
    # --------------------------------------------------------------------- #
    if used_count == total_grid:
        return 0
    elif used_count < total_grid:
        # not enough cells
        return 3
    else:  # used_count > total_grid
        return 2

# --------------------------------------------------------------------------- #
# CLI handling
# --------------------------------------------------------------------------- #

def parse_args(argv: List[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Tile cell SVGs onto a base SVG in a regular grid."
    )

    # Base SVG
    parser.add_argument(
        "--base",
        required=True,
        type=lambda v: _parse_path(v, "--base"),
        help="Path to the base SVG file.",
    )

    # Cell SVGs – either multiple or a single one repeated
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--cells",
        nargs="+",
        type=lambda v: _parse_path(v, "--cells"),
        help="Paths to the cell SVG files that will be tiled in order.",
    )
    group.add_argument(
        "--single-cell",
        metavar="CELL",
        type=lambda v: _parse_path(v, "--single-cell"),
        help="Path to a single cell SVG that will be repeated across the grid.",
    )

    # Output file
    parser.add_argument(
        "--out",
        required=True,
        type=lambda v: os.path.abspath(v),
        help="Path where the resulting SVG will be written.",
    )

    # Grid parameters
    parser.add_argument(
        "--interval_w",
        required=True,
        type=lambda v: _parse_float(v, "--interval_w"),
        help="Horizontal distance between cell centres.",
    )
    parser.add_argument(
        "--interval_h",
        required=True,
        type=lambda v: _parse_float(v, "--interval_h"),
        help="Vertical distance between cell centres.",
    )
    parser.add_argument(
        "--start_x",
        required=True,
        type=lambda v: _parse_float(v, "--start_x"),
        help="X‑coordinate of the first cell.",
    )
    parser.add_argument(
        "--start_y",
        required=True,
        type=lambda v: _parse_float(v, "--start_y"),
        help="Y‑coordinate of the first cell.",
    )

    return parser.parse_args(argv)

def main(argv: List[str]) -> int:
    try:
        args = parse_args(argv)

        # Load the base
        base_tree = load_svg(args.base)
        base_root = base_tree.getroot()
        base_root.tag = "svg"

        # Load the cells
        if args.cells:
            cell_paths = args.cells
            single = False
        else:
            cell_paths = [args.single_cell]
            single = True

        cell_elems = []
        for p in cell_paths:
            tree = load_svg(p)
            cell_root = tree.getroot()
            # We use the *whole* root element – it might contain <defs> etc.
            # But for the purpose of tiling we just embed it as-is.
            cell_elems.append(cell_root)

        # Tile them
        code = tile_cells(
            base_root=base_root,
            cell_elems=cell_elems,
            out_path=args.out,
            interval_w=args.interval_w,
            interval_h=args.interval_h,
            start_x=args.start_x,
            start_y=args.start_y,
            single_cell=single,
        )
        return code

    except Exception as exc:
        # Any error is a failure (exit code 1)
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

