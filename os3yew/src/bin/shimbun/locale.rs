use std::{collections::HashMap, sync::LazyLock};

const LOCALE_DATA_JA: LazyLock<HashMap<String, String>> = std::sync::LazyLock::new(|| {
    HashMap::from_iter(
        vec![
            ("keep_reading_button", "読み続ける"),
            ("start_again_button", "はじめから"),
            ("tips_scroll", "↓ スクロールしてみてください ↓"),
            ("tips_words", "文字たちをなぞってみてください"),
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
            ("tips_scroll", "↓ scroll ↓"),
            ("tips_words", "move mouse through the words"),
        ]
        .into_iter()
        .map(|x| (x.0.to_string(), x.1.to_string())),
    )
});

//     candidates.push((0, "本日".to_string()));
//     candidates.push((1, format!("本日{}日", self.day)));
//         candidates.push((2, "明日".to_string()));
//         candidates.push((3, format!("明日{}日", self.day)));
//         candidates.push((4, "昨日".to_string()));
//         candidates.push((5, format!("昨日{}日", self.day)));
//         candidates.push((6, format!("先月{}日", self.day)));
//         candidates.push((7, format!("来月{}日", self.day)));
//     candidates.push((8, format!("今月{}日", self.day)));
//     candidates.push((13, format!("{}日", self.day)));
//     candidates.push((9, format!("{}月{}日", new_month, self.day)));
//     candidates.push((10, format!("昨年{}月{}日", new_month, self.day)));
//     candidates.push((11, format!("来年{}月{}日", new_month, self.day)));
// candidates.push((12, format!("{}年{}月{}日", new_year, new_month, self.day)));

pub fn get_system_word(lang_code: &str, data_key: &str) -> String {
    if lang_code == "en" {
        return LOCALE_DATA_EN.get(data_key).unwrap().clone();
    }
    if lang_code == "ja" {
        return LOCALE_DATA_JA.get(data_key).unwrap().clone();
    }
    panic!("not implemented language");
}
