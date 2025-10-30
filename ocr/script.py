from tempfile import tempdir
from PIL import Image

import pytesseract

import math

from jinja2 import Template, Environment, FileSystemLoader
import sys

import gi

from typing import NamedTuple
import os

gi.require_version('HarfBuzz', '0.0')

from gi.repository import HarfBuzz

import json

blob = HarfBuzz.blob_create_from_file('NotoSerifJP-VariableFont_wght.ttf')
face = HarfBuzz.face_create(blob, 0)
font = HarfBuzz.font_create(face)

class Extents(NamedTuple):
    width: int
    height: int
    x_bearing: int
    y_bearing: int

class Advance(NamedTuple):
    x: int
    y: int

class Shape(NamedTuple):
    x_advance: int
    y_advance: int
    x_offset: int
    y_offset: int

class OCR(NamedTuple):
    left: int
    top: int
    width: int
    height: int
    confidence: float
    text: str

class PageText(NamedTuple):
    font_size: float
    mult_x: float
    mult_y: float
    x: float
    y: float
    target_width: float
    target_height: float
    text: str

class Page(NamedTuple):
    # actual size (px, in, cm, etc.)
    width: str
    height: str
    # coordinate unit (x, y, width, height)
    viewbox: tuple[float, float, float, float]
    # text data
    texts: list[PageText]

def get_glyph(char: str) -> int:
    if len(char) != 1:
        raise ValueError('String length must be 1')
    success, ret = HarfBuzz.font_get_glyph(font,ord(char), 0)
    if not success:
        raise ValueError('Bad Character')
    return ret

def get_extents(char: str) -> Extents:
    e = HarfBuzz.font_get_glyph_extents(font, get_glyph(char))[1]
    return Extents(e.width, e.height, e.x_bearing, e.y_bearing)

def get_advance(char: str, direction: int = HarfBuzz.direction_t.LTR) -> Advance:
    if isinstance(direction, int):
        direction = HarfBuzz.direction_t(direction)
    elif not isinstance(direction, HarfBuzz.direction_t):
        raise ValueError('invalid argument')
    a = HarfBuzz.font_get_glyph_advance_for_direction(font, get_glyph(char), direction)
    return Advance(a.x, a.y)

def get_shape(chars: str, direction: int = HarfBuzz.direction_t.LTR) -> list[Shape]:
    if isinstance(direction, int):
        direction = HarfBuzz.direction_t(direction)
    elif not isinstance(direction, HarfBuzz.direction_t):
        raise ValueError('invalid argument')
    
    b = HarfBuzz.buffer_create()
    HarfBuzz.buffer_set_content_type(b, HarfBuzz.buffer_content_type_t.UNICODE)
    HarfBuzz.buffer_set_direction(b, direction)
    HarfBuzz.buffer_add_codepoints(b, [ord(x) for x in chars], 0, len(chars))
    HarfBuzz.shape(font, b)
    pos = HarfBuzz.buffer_get_glyph_positions(b)
    return [Shape(
        p.x_advance,
        p.y_advance,
        p.x_offset,
        p.y_offset
    ) for p in pos]

def get_shape_and_extents(chars: str, direction: int = HarfBuzz.direction_t.LTR) -> list[tuple[Extents, Shape]]:
    return list(zip([get_extents(x) for x in chars], get_shape(chars, direction)))

def run_ocr(path):
    ocr_result = []

    with Image.open(path) as f:
        for line in pytesseract.image_to_data(f).split('\n'):
            data = line.split('\t')
            if len(data) == 12 and data[0] != 'level' and data[-1] != '':
                left = data[6]
                top = data[7]
                width = data[8]
                height = data[9]
                confidence = data[10]
                text = data[11]
                o = OCR(int(left), int(top), int(width), int(height), float(confidence), text)
                ocr_result.append(o)
    
    return ocr_result

def get_page_info(path, layout_plan: int = 1):
    with Image.open(path) as f:
        image_width = f.width
        image_height = f.height
        if 'dpi' in f.info:
            dpi = f.info['dpi'][0]
            actual_width = (image_width / dpi, 'in')
        else:
            actual_width = (21, 'cm')
    actual_height = (actual_width[0] * (image_height / image_width), actual_width[1])

    text_data = []
    ocr_results = run_ocr(sys.argv[1])
    
    for result in ocr_results:
        target_height = result.height
        target_width = result.width
        se = get_shape_and_extents(result.text)
        # total text width and height when font size = 1024
        # (plan 1)
        # font_size = target_height
        # y-transform multiplier = 1024 / hb_height
        # x-transform multiplier = target_width / ((hb_width / 1024) * font_size)
        # (plan 2)
        # font_size = target_height * (1024 / hb_height)
        # y-transform multiplier = 1.00
        # x-transform multiplier = target_width / ((hb_width / 1024) * font_size)
        hb_width = sum([x[1].x_advance for x in se])
        hb_height = max([x[0].y_bearing for x in se])
        if hb_height == 0:
            continue
        if layout_plan == 1:
            font_size = target_height
            mult_y = 1024 / hb_height
            mult_x = target_width / ((hb_width / 1024) * font_size)
        elif layout_plan == 2:
            font_size = target_height * (1024 / hb_height)
            mult_y = 1.00
            mult_x = target_width / ((hb_width / 1024) * font_size)
        else:
            raise ValueError('invalid layout_plan value')
        text_data.append(PageText(font_size, mult_x, mult_y, result.left, result.top, target_width, target_height, result.text))
    
    return Page(
        str(actual_width[0]) + actual_width[1],
        str(actual_height[0]) + actual_height[1],
        (0, 0, image_width, image_height),
        text_data
    )

if __name__ == '__main__':
    page = get_page_info(sys.argv[1])
    env = Environment(loader=FileSystemLoader(os.path.abspath(os.path.dirname(__file__))))
    template = env.get_template('templates/main.jinja2')
    rendered = template.render({
        'page': page
    })
    print(rendered)