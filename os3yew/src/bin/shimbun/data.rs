// しんぶんからひろがっていくせかい

pub mod util;

use askama::Template;
use os3yew::util::{CharacterMetric, RectMask};
// use gloo_net::http::Request;
use rand::{Rng, RngExt, SeedableRng, distr::slice::Choose, rng, rngs::StdRng};
use rust_embed::RustEmbed;
// use rustybuzz::{UnicodeBuffer, shape};
use chrono::{Datelike, NaiveDate};
use std::fmt::Display;
use web_sys::{js_sys::Atomics::add, window};

#[derive(PartialEq, Clone, Copy, Debug, PartialOrd)]
pub enum GameStage {
    ArticleView,
    ArticleFading,
    ForecastStart,
    Cleanup,
}

#[derive(RustEmbed)]
#[folder = "metadata/shimbun"]
pub struct Asset;

#[derive(Template, Debug, Clone, PartialEq)]
#[template(path = "text_combined.txt")]
pub struct ArticleTemplate {
    pub title: String,
    pub mood: Mood,
    pub meta: Meta,
    pub date: Box<Date>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub original_date: Option<Box<Date>>,
    pub condition_seed: u64,
    pub lang: String,

    /// Is first word in the sentence?
    pub is_first_word: bool,

    /// Is the preposition already provided (such as "since Jan 6th")? Then, use DoNotFill.
    /// If not, use PrepositionOn and such.
    /// (When Using PrepositionOn, we'll have "on Jan 6th", "today" (not "on today") etc.)
    pub preposition: PrepositionType
}

#[derive(Debug, Clone, PartialEq)]
/// Is the preposition already provided (such as "since Jan 6th")? Then, use DoNotFill.
/// If not, use PrepositionOn and such.
/// (When Using PrepositionOn, we'll have "on Jan 6th", "today" (not "on today") etc.)
pub enum PrepositionType {
    DoNotFill,
    PrepositionOn
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'a> Date {
    pub fn new(year: i32, month: u32, day: u32, lang: String) -> Self {
        Date {
            year,
            month,
            day,
            original_date: None,
            condition_seed: rng().next_u64(),
            lang,
            is_first_word: false,
            preposition: PrepositionType::PrepositionOn
        }
    }

    pub fn first_word(&self) -> Date {
        Date { is_first_word: true, ..self.clone() }
    }

    pub fn no_first_word(&self) -> Date {
        Date { is_first_word: false, ..self.clone() }
    }

    pub fn prep_on(&self) -> Date {
        Date { preposition: PrepositionType::PrepositionOn, ..self.clone() }
    }

    pub fn no_fill_prep(&self) -> Date {
        Date { preposition: PrepositionType::DoNotFill, ..self.clone() }
    }

    pub fn move_origin(&self) -> Date {
        // discard the original_date field in the original_date arg
        Date {
            original_date: None,
            condition_seed: rng().next_u64(),
            ..self.clone()
        }
    }

    pub fn after(&self, days: u32) -> Date {
        let naive_date = NaiveDate::from_ymd_opt(self.year, self.month, self.day)
            .expect("Invalid date")
            .checked_add_signed(chrono::Duration::days(days as i64))
            .expect("Date addition error");
        let new_date = Date {
            year: naive_date.year(),
            month: naive_date.month(),
            day: naive_date.day(),
            original_date: Some(Box::new(self.clone())),
            ..self.clone()
        };

        new_date
    }

    pub fn before(&self, days: u32) -> Date {
        let naive_date = NaiveDate::from_ymd_opt(self.year, self.month, self.day)
            .expect("Invalid date")
            .checked_sub_signed(chrono::Duration::days(days as i64))
            .expect("Date subtraction error");

        let new_date = Date {
            year: naive_date.year(),
            month: naive_date.month(),
            day: naive_date.day(),
            original_date: Some(Box::new(self.clone())),
            ..self.clone()
        };

        new_date
    }

    pub fn month_day(&self) -> String {
        let mut candidates: Vec<String> = Vec::new();

        let add_prop = |x: String| {
            if self.preposition == PrepositionType::PrepositionOn {
                "on ".to_string() + &x
            } else {
                x
            }
        };

        // If this is the original date, we call it "今日"
        if self.original_date.is_none() {
            if self.lang == "ja" {
                candidates.push(("本日".to_string()));
                candidates.push(("本日%-e日".to_string()));
            }
            if self.lang == "en" {
                candidates.push(("today".to_string()));
                candidates.push(("today %-e<th>".to_string()));
                candidates.push(("%-e<th> today".to_string()));
            }
        }

        let self_clone = Box::new(self.clone());

        let original = self.original_date.as_ref().unwrap_or(&self_clone);

        // Prepare naive dates for day‑difference calculation
        let naive_self =
            NaiveDate::from_ymd_opt(self.year, self.month, self.day).expect("Invalid date");
        let naive_orig = NaiveDate::from_ymd_opt(original.year, original.month, original.day)
            .expect("Invalid date");

        // Day difference in days (positive → self is later)
        let day_diff: i64 = (naive_self - naive_orig).num_days();

        // Yesterday / Tomorrow logic – check first
        match day_diff {
            1 => {
                if self.lang == "ja" {
                    candidates.push(("明日".to_string()));
                    candidates.push(("明日%-e日".to_string()));
                }
                if self.lang == "en" {
                    candidates.push(("tomorrow".to_string()));
                    candidates.push(("tomorrow %-e<th>".to_string()));
                }
            }
            -1 => {
                if self.lang == "ja" {
                    candidates.push(("昨日".to_string()));
                    candidates.push(("昨日%-e日".to_string()));
                }
                if self.lang == "en" {
                    candidates.push(("yesterday".to_string()));
                    candidates.push(("yesterday %-e<th>".to_string()));
                }
            }
            _ => {} // continue with month‑based logic
        }

        let orig_year = original.year;
        let orig_month = original.month as i32;

        let new_year = self.year;
        let new_month = self.month as i32;

        // Month difference taking year rollover into account
        let month_diff: i32 = if new_year == orig_year {
            new_month - orig_month
        } else if new_year == orig_year + 1 {
            (new_month - orig_month) + 12
        } else if new_year == orig_year - 1 {
            (new_month - orig_month) - 12
        } else {
            999
        };

        // “先月/来月”
        match month_diff {
            -1 => {
                if self.lang == "ja" {
                    candidates.push(("先月%-e日".to_string()));
                }
                if self.lang == "en" {
                    candidates.push(add_prop("%-e<th> last month".to_string()));
                }
            }
            1 => {
                if self.lang == "ja" {
                    candidates.push(("来月%-e日".to_string()));
                }
                if self.lang == "en" {
                    candidates.push(add_prop("%-e<th> next month".to_string()));
                }
            }
            _ => {} // fall‑through for other cases
        }

        // Same month & year – “今月…”
        if orig_month == new_month && orig_year == new_year {
            if self.lang == "ja" {
                candidates.push(("今月%-e日".to_string()));
                candidates.push(("%-e日".to_string()));
            }
            if self.lang == "en" {
                candidates.push(add_prop("%-e<th> this month".to_string()));
            }
        }

        // Same year but different month – “X月…”
        if orig_year == new_year {
            if self.lang == "ja" {
                candidates.push(("%-m月%-e日".to_string()));
            }
            if self.lang == "en" {
                candidates.push(add_prop("%B %-e<th>".to_string()));
                candidates.push(add_prop("%B %-e".to_string()));
            }
        }

        // Previous year – “昨年…”
        if new_year == orig_year - 1 {
            if self.lang == "ja" {
                candidates.push(("昨年%-m月%-e日".to_string()));
            }
            if self.lang == "en" {
                candidates.push(add_prop("%B %-e<th> last year".to_string()));
                candidates.push(add_prop("%B %-e last year".to_string()));
            }
        }

        // Next year – “来年…”
        if new_year == orig_year + 1 {
            if self.lang == "ja" {
                candidates.push(("来年%-m月%-e日".to_string()));
            }
            if self.lang == "en" {
                candidates.push(add_prop("%B %-e<th> next year".to_string()));
                candidates.push(add_prop("%B %-e next year".to_string()));
            }
        }

        // Fallback – fTIME_TILL_LOCK
        if self.lang == "ja" {
            candidates.push("%Y年%-m月%-e日".to_string());
        }
        if self.lang == "en" {
            candidates.push(add_prop("%B %-e<th>, %Y".to_string()));
            candidates.push(add_prop("%B %-e, %Y".to_string()));
        }

        // This will serialize the program execution tree. Inputs which share same execution tree
        // results in the same condition_hash.
        let condition_list = candidates.iter().cloned().collect::<Vec<_>>();
        let rs = ahash::RandomState::with_seed(42);
        let condition_hash = rs.hash_one(condition_list);

        // Combine with the current article's random seed. If we have the same article and the same
        // execution tree, then the same index will be chosen. (And if the execution tree is the
        // same, obviously the array length and its semantic structure are the same.)
        let mut r = StdRng::seed_from_u64(self.condition_seed ^ condition_hash);

        let string_list = candidates
            .iter()
            .map(|x| {
                let mut y = "".to_string();
                let _ = naive_self.format(x).write_to(&mut y);
                if self.lang == "en" {
                    y = y.replace("<th>", {
                        match self.day % 10 {
                            1 => "st",
                            2 => "nd",
                            3 => "rd",
                            _ => "th"
                        }
                    });
                    if self.is_first_word {
                        let upper_c = y.chars().nth(0).unwrap().to_ascii_uppercase();
                        y.replace_range(0..1, &upper_c.to_string());
                    }
                }
                y
            })
            .collect::<Vec<String>>();

        r.sample(Choose::new(&string_list).unwrap()).clone()
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Meta {
    pub window_width: f64,
    pub window_height: f64,
    pub global_pos_x: f64,
    pub global_pos_y: f64,
}

impl Default for Meta {
    fn default() -> Self {
        let w = window().unwrap().inner_width().unwrap().as_f64().unwrap();
        let h = window().unwrap().inner_height().unwrap().as_f64().unwrap();
        Meta {
            window_width: w,
            window_height: h,
            global_pos_x: 0.0,
            global_pos_y: 0.0,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Mood {}

impl Mood {
    pub fn is_subjective(&self) -> bool {
        true
    }
}

mod filters {
    use std::fmt::Display;
    #[askama::filter_fn]
    pub fn footnote(
        // Value that's piped into the filter within the jinja template.
        // This can be of any type. `impl Display` is just an example.
        value: impl Display,
        // This is askama's runtime values environment. Together with
        // values, these two arguments are always passed into a custom filter.
        env: &dyn askama::Values,
    ) -> askama::Result<String> {
        Ok(format!("<div class=\"footnote\">\n{value}\n</div>"))
    }
}


#[derive(Clone, PartialEq)]
pub struct Article {
    pub template: ArticleTemplate,
    pub w: Option<f64>,
    pub h: Option<f64>,
    pub x: f64,
    pub y: f64,
    pub masks: Vec<Option<RectMask>>,
    pub character_metrics: Option<Vec<CharacterMetric>>
}
