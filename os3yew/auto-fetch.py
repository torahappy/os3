import os
import regex
import re
import sqlite3
import argparse
from typing import Optional, Tuple, List
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
    cursor.execute("SELECT title, body, deleted_time, id FROM notes WHERE parent_id = ?;", (folder_id[0],))
    notes: List[Tuple[str, str, int, str]] = cursor.fetchall()

    conn.close()

    return [(title, body, note_id) for (title, body, deleted_time, note_id) in notes if deleted_time == 0]

assets_dir_texts = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'assets', 'texts')
assets_dir_images = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'assets', 'images')

def gen_files(notes: List[Tuple[str, str, str]]):
    "copy texts and images to the project directory"
    shutil.rmtree(assets_dir_texts, ignore_errors=True)
    os.makedirs(assets_dir_texts)
    shutil.rmtree(assets_dir_images, ignore_errors=True)
    os.makedirs(assets_dir_images)

    text_combined = ""

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
                # Replace the markdown image link
                note_body = re.sub(rf'!\[.+?\]\(:/{image_id}\)', f'![file{os.path.splitext(found_file)[1]}](assets/images/{os.path.basename(found_file)})', note_body)

        text_combined += "{% if title == \"" + note_title + "\" %}\n" + note_body + "\n{% endif %}\n"
    
    with open(os.path.join(assets_dir_texts, 'text_combined.txt'), 'w') as f:
        f.write(text_combined)

if __name__ == '__main__':
    notes = get_notes()
    gen_files(notes)