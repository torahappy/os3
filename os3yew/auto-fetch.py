import os
import regex
import sqlite3
import argparse
from typing import Optional, Tuple, List
import shutil

def get_notes() -> List[Tuple[str, str]]:
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
    cursor.execute("SELECT title, body, deleted_time FROM notes WHERE parent_id = ?;", (folder_id[0],))
    notes: List[Tuple[str, str, int]] = cursor.fetchall()

    conn.close()

    return [(a, b) for (a, b, c) in notes if c == 0]

if __name__ == '__main__':
    assets_dir_texts = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'assets', 'texts')
    shutil.rmtree(assets_dir_texts)
    os.makedirs(assets_dir_texts)
    assets_dir_images = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'assets', 'images')
    shutil.rmtree(assets_dir_images)
    os.makedirs(assets_dir_images)

    notes = get_notes()
    for note_title, note_body in notes:
        if note_title.startswith('!!!'):
            continue

        # Create a safe filename by replacing invalid characters
        safe_filename = "".join(c if regex.match(r'[\w\d\p{Han}\p{Hiragana}\p{Katakana}「」]', c) else '_' for c in note_title)
        if safe_filename == '':
            continue
        safe_filename += '.txt'

        # Write the note content to a file
        with open(os.path.join(assets_dir_texts, safe_filename), 'w', encoding='utf-8') as f:
            f.write(note_body)

        