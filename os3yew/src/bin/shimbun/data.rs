// しんぶんからひろがっていくせかい

pub mod util;

use os3yew::util::RectMask;
use askama::Template;
// use gloo_net::http::Request;
use rand::{
    Rng, RngExt, SeedableRng,
    distr::slice::Choose,
    rng,
    rngs::StdRng,
};
use rust_embed::RustEmbed;
// use rustybuzz::{UnicodeBuffer, shape};
use std::
    fmt::Display
;
use web_sys::window;
use chrono::{Datelike, NaiveDate};


#[derive(PartialEq, Clone, Copy, Debug, PartialOrd)]
pub enum GameStage {
    ArticleView,
    ArticleFading,
    ForecastStart,
}

#[derive(RustEmbed)]
#[folder = "metadata"]
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
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'a> Date {
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Date {
            year,
            month,
            day,
            original_date: None,
            condition_seed: rng().next_u64(),
        }
    }

    pub fn move_origin(&self) -> Date {
        // discard the original_date field in the original_date arg
        Date {
            year: self.year.clone(),
            month: self.month.clone(),
            day: self.day.clone(),
            original_date: None,
            condition_seed: rng().next_u64(),
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
            condition_seed: self.condition_seed,
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
            condition_seed: self.condition_seed,
        };

        new_date
    }

    pub fn month_day(&self) -> String {
        let mut candidates: Vec<(u32, String)> = Vec::new();

        // If this is the original date, we call it "今日"
        if self.original_date.is_none() {
            candidates.push((0, "本日".to_string()));
            candidates.push((1, format!("本日{}日", self.day)));
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
                candidates.push((2, "明日".to_string()));
                candidates.push((3, format!("明日{}日", self.day)));
            }
            -1 => {
                candidates.push((4, "昨日".to_string()));
                candidates.push((5, format!("昨日{}日", self.day)));
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
                candidates.push((6, format!("先月{}日", self.day)));
            }
            1 => {
                candidates.push((7, format!("来月{}日", self.day)));
            }
            _ => {} // fall‑through for other cases
        }

        // Same month & year – “今月…”
        if orig_month == new_month && orig_year == new_year {
            candidates.push((8, format!("今月{}日", self.day)));
            candidates.push((13, format!("{}日", self.day)));
        }

        // Same year but different month – “X月…”
        if orig_year == new_year {
            candidates.push((9, format!("{}月{}日", new_month, self.day)));
        }

        // Previous year – “昨年…”
        if new_year == orig_year - 1 {
            candidates.push((10, format!("昨年{}月{}日", new_month, self.day)));
        }

        // Next year – “来年…”
        if new_year == orig_year + 1 {
            candidates.push((11, format!("来年{}月{}日", new_month, self.day)));
        }

        // Fallback – full date
        candidates.push((12, format!("{}年{}月{}日", new_year, new_month, self.day)));

        // This will serialize the program execution tree. Inputs which share same execution tree
        // results in the same condition_hash.
        let condition_list = candidates.iter().map(|x| x.0.clone()).collect::<Vec<u32>>();
        let rs = ahash::RandomState::with_seed(42);
        let condition_hash = rs.hash_one(condition_list);

        // Combine with the current article's random seed. If we have the same article and the same
        // execution tree, then the same index will be chosen. (And if the execution tree is the
        // same, obviously the array length and its semantic structure are the same.)
        let mut r = StdRng::seed_from_u64(self.condition_seed ^ condition_hash);

        let string_list = candidates
            .iter()
            .map(|x| x.1.clone())
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

impl Meta {
    pub fn get_instruction_manual(&self) -> String {
        return "下にあるボタンを押すと、今読んでいる文章が消えていきます。そうしてしばらくすると、色々な言葉の断片が浮かび上がっていきます。その言葉の断片の上にマウスカーソルを置いて、マウスを押し続けると、新たな文章が浮かび上がっていきます。".to_string();
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
        Ok(format!("<div class=\"footnote\">{value}</div>"))
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
}

