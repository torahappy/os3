use super::*;

/// Test `ImageFontLayout::Automatic` creates the correct character map.
#[test]
fn layout_automatic() {
    let layout = ImageFontLayout::Automatic("ABCD\nEFGH\nIJKL".to_string());

    // Each char is 100x100
    let image_size = UVec2::new(400, 300); // 4 characters wide, 3 rows
    let char_map = layout
        .into_character_rect_map(image_size)
        .expect("valid layout");

    assert_eq!(char_map.len(), 12);
    assert!(('A'..'L').all(|character| char_map.contains_key(&character)));
    assert_eq!(
        char_map[&'A'],
        // A is at 0 right, 0 down
        URect::new(0, 0, 100, 100),
        "Character 'A' has incorrect bounds."
    );
    assert_eq!(
        char_map[&'H'],
        // H is at 3 right, 1 down
        URect::new(300, 100, 400, 200),
        "Character 'H' has incorrect bounds."
    );
    assert_eq!(
        char_map[&'J'],
        // H is at 1 right, 2 down
        URect::new(100, 200, 200, 300),
        "Character 'J' has incorrect bounds."
    );
}

#[test]
fn layout_automatic_invalid_image_width() {
    let layout = ImageFontLayout::Automatic("AB\nCD".to_string());
    let image_size = UVec2::new(301, 200); // 301 / 2 is not a whole number
    let result = layout.into_character_rect_map(image_size);

    assert!(matches!(
        result,
        Err(ImageFontLayoutValidationError::InvalidImageWidth {
            width: 301,
            per_line_character_count: 2
        })
    ));
}

#[test]
fn layout_automatic_invalid_image_height() {
    let layout = ImageFontLayout::Automatic("AB\nCD".to_string());
    let image_size = UVec2::new(300, 201); // 201 / 2 is not a whole number
    let result = layout.into_character_rect_map(image_size);

    assert!(matches!(
        result,
        Err(ImageFontLayoutValidationError::InvalidImageHeight {
            height: 201,
            line_count: 2
        })
    ));
}

#[test]
fn layout_automatic_repeated_characters_error() {
    // Define a layout string with repeated characters
    let layout = ImageFontLayout::Automatic("AB\nAC".to_string());

    let image_size = UVec2::new(200, 100); // Each cell is 100x100 pixels
    let result = layout.into_character_rect_map(image_size);

    // Verify that an error is returned due to repeated characters
    assert!(
        matches!(
            result,
            Err(ImageFontLayoutValidationError::AutomaticRepeatedCharacter {
                row: 1,
                column: 0,
                character: 'A'
            })
        ),
        "{result:?}"
    );
}

#[test]
fn layout_manual_monospace() {
    let layout = ImageFontLayout::ManualMonospace {
        size: UVec2::new(10, 20),
        coords: HashMap::from([('a', UVec2::new(0, 0)), ('b', UVec2::new(10, 0))]),
    };

    // Arbitrary size; ManualMonospace supports custom positioning, so it's
    // independent of the actual size of the source texture
    let image_size = UVec2::new(100, 50);
    let char_map = layout
        .into_character_rect_map(image_size)
        .expect("valid layout");

    assert_eq!(char_map.len(), 2);
    assert_eq!(char_map[&'a'], URect::new(0, 0, 10, 20));
    assert_eq!(char_map[&'b'], URect::new(10, 0, 20, 20));
}

#[test]
fn test_image_font_layout_manual() {
    let layout = ImageFontLayout::Manual(HashMap::from([
        ('x', URect::new(0, 0, 5, 5)),
        ('y', URect::new(5, 5, 15, 15)),
    ]));

    // Arbitrary size; Manual supports custom positioning, so it's independent of
    // the actual size of the source texture
    let image_size = UVec2::new(100, 100);
    let char_map = layout
        .into_character_rect_map(image_size)
        .expect("valid layout");

    assert_eq!(char_map.len(), 2);
    assert_eq!(char_map[&'x'], URect::new(0, 0, 5, 5));
    assert_eq!(char_map[&'y'], URect::new(5, 5, 15, 15));
}

#[test]
fn layout_manual_empty_map() {
    let layout = ImageFontLayout::Manual(HashMap::new());
    let image_size = UVec2::new(100, 100);
    let char_map = layout
        .into_character_rect_map(image_size)
        .expect("valid layout");

    assert!(
        char_map.is_empty(),
        "Expected empty char map for empty input."
    );
}
