use std::{collections::HashMap, sync::LazyLock};

const LOCALE_DATA_JA: LazyLock<HashMap<String, String>> = std::sync::LazyLock::new(|| {
    HashMap::from_iter(
        vec![
            ("keep_reading_button", "読み続ける"),
            ("start_again_button", "はじめから"),
        ]
        .into_iter()
        .map(|x| (x.0.to_string(), x.1.to_string())),
    )
});

const LOCALE_DATA_EN: LazyLock<HashMap<String, String>> = std::sync::LazyLock::new(|| {
    HashMap::from_iter(
        vec![
            ("keep_reading_button", "keep reading"),
            ("start_again_button", "start again"),
        ]
        .into_iter()
        .map(|x| (x.0.to_string(), x.1.to_string())),
    )
});

pub fn get_system_word(lang_code: &str, data_key: &str) -> String {
    if lang_code == "en" {
        return LOCALE_DATA_EN.get(data_key).unwrap().clone();
    }
    if lang_code == "ja" {
        return LOCALE_DATA_JA.get(data_key).unwrap().clone();
    }
    panic!("not implemented language");
}
