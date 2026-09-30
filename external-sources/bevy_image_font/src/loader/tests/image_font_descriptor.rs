use super::*;

#[test]
fn validation_accepts_valid_descriptor() {
    let valid_descriptor = ImageFontDescriptor::new(
        Utf8PathBuf::from("some/path"),
        ImageFontLayout::Automatic(String::from("A")),
    );

    assert!(valid_descriptor.is_ok());

    // For the sake of coverage
    println!("{:?}", valid_descriptor.unwrap().layout());
}

#[test]
fn validation_rejects_empty_path() {
    let invalid_descriptor = ImageFontDescriptor::new(
        Utf8PathBuf::from(""),
        ImageFontLayout::Automatic(String::from("A")),
    );

    assert!(matches!(
        invalid_descriptor,
        Err(ImageFontDescriptorValidationError::EmptyImagePath)
    ));
}

#[test]
fn validation_rejects_empty_automatic_layout() {
    let invalid_descriptor = ImageFontDescriptor::new(
        Utf8PathBuf::from("some/path"),
        ImageFontLayout::Automatic(String::new()),
    );

    assert!(matches!(
        invalid_descriptor,
        Err(ImageFontDescriptorValidationError::EmptyLayoutString)
    ));
}
