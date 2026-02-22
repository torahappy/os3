import os
from pathlib import Path
import regex
import re
import sqlite3
import argparse
from typing import Optional, Tuple, List
from PIL import Image, ImageOps
import shutil
import json

def get_notes() -> List[Tuple[str, str, str]]:
    """Main function to query Joplin notes by folder title."""
    parser = argparse.ArgumentParser(description='Query Joplin notes by folder title.')
    parser.add_argument('title', type=str, help='Title of the folder to search for')
    args = parser.parse_args()

    db_path: str = os.path.expanduser('~/.config/joplin-desktop/database.sqlite')
    conn: sqlite3.Connection = sqlite3.connect(db_path)
    cursor: sqlite3.Cursor = conn.cursor()

    # Get folder ID
    cursor.execute("SELECT id FROM folders WHERE title = ?;", (args.title,))
    folder_id: Optional[Tuple[str]] = cursor.fetchone()

    if folder_id is None:
        raise ValueError(f"No folder found with title '{args.title}'")

    # Get notes in the folder
    cursor.execute("SELECT title, body, deleted_time, id, is_conflict FROM notes WHERE parent_id = ?;", (folder_id[0],))
    notes: List[Tuple[str, str, int, str, bool]] = cursor.fetchall()

    conn.close()

    return [(title, body, note_id) for (title, body, deleted_time, note_id, is_conflict) in notes if deleted_time == 0 and not is_conflict]

assets_dir_texts = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'templates')
assets_dir_images = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'assets', 'images')
assets_dir_metadata = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'metadata')

def gen_files(notes: List[Tuple[str, str, str]], dimensions: dict[str, list[int]] | None):
    "copy texts and images to the project directory"
    os.makedirs(assets_dir_texts, exist_ok=True)
    os.makedirs(assets_dir_images, exist_ok=True)
    os.makedirs(assets_dir_metadata, exist_ok=True)

    if dimensions:
        text_combined = ""

    list_titles = []

    for note_title, note_body, note_id in notes:
        if note_title.startswith('!!!'):
            continue

        # Process image references in the note body
        image_refs = re.findall(r'!\[.+?\]\(:/([0-9a-f]{32})\)', note_body)
        for image_id in image_refs:
            # Look for the image file in Joplin's resources directory
            resources_dir = os.path.expanduser('~/.config/joplin-desktop/resources')
            found_file = None
            for root, _, files in os.walk(resources_dir):
                for file in files:
                    if file.startswith(image_id) and not file.endswith('.crypted'):
                        found_file = os.path.join(root, file)
                        break
                if found_file:
                    break

            if found_file:
                # Copy the image to the assets directory
                if not os.path.exists(os.path.join(assets_dir_images, os.path.basename(found_file))):
                    shutil.copy2(found_file, os.path.join(assets_dir_images, os.path.basename(found_file)))
                if dimensions:
                    dim = dimensions[os.path.basename(found_file)]
                # Replace the markdown image link
                    note_body = re.sub(rf'!\[.+?\]\(:/{image_id}\)', f'<img src="assets/images/{os.path.basename(found_file)}" style="aspect-ratio: {dim[0] / dim[1]};" />', note_body)

        if dimensions:
            text_combined += "{% if title == \"" + note_title + "\" %}\n" + note_body + "\n{% endif %}\n"

        list_titles.append(note_title)
    
    if dimensions:
        with open(os.path.join(assets_dir_texts, 'text_combined.txt'), 'w') as f:
            f.write(text_combined)
    
    with open(os.path.join(assets_dir_metadata, 'titles.json'), 'w') as f:
        json.dump(list_titles, f, indent=4, ensure_ascii=False, sort_keys=True)

def exif_corrected(img: Image.Image) -> Image.Image:
    """
    Return a copy of `img` with the correct orientation applied
    (if the image contains an EXIF orientation tag).
    """
    return ImageOps.exif_transpose(img)

def collect_dimensions() -> dict[str, list[int]]:
    """
    Scan `assets_dir_images` for files with a common image extension,
    open them, correct the orientation and record the width & height.

    Returns:
        mapping:  filename (not full path) → [width, height]
    """
    # Common image extensions – feel free to add more if you need
    IMAGE_EXTS = {".jpg", ".jpeg", ".png", ".webp"}

    mapping: dict[str, list[int]] = {}

    for img_path in Path(assets_dir_images).iterdir():
        if not img_path.is_file():
            continue

        if img_path.suffix.lower() not in IMAGE_EXTS:
            # Skip non‑image files
            continue

        try:
            with Image.open(img_path) as im:
                im = exif_corrected(im)          # ← correct orientation
                w, h = im.size                      # ← (width, height)
                mapping[img_path.name] = [w, h]
        except Exception as exc:
            print(f"[WARN] Could not process {img_path.name}: {exc}")

    return mapping

def write_images_json(mapping: dict[str, list[int]]) -> None:
    """
    Write ``mapping`` to ``assets_dir_metadata/images.json``.
    """
    out_file = Path(assets_dir_metadata) / "images.json"

    # Pretty‑print the JSON for easier human inspection
    with out_file.open("w", encoding="utf-8") as fp:
        json.dump(mapping, fp, indent=4, sort_keys=True)

if __name__ == '__main__':
    shutil.rmtree(assets_dir_images)
    notes = get_notes()
    gen_files(notes, None)
    dimensions = collect_dimensions()
    gen_files(notes, dimensions)
    write_images_json(dimensions)

