// しんぶんからひろがっていくせかい

use yew::prelude::*;
use askama::Template;
use std::fmt::Display;

#[derive(Template)]
#[template(path = "text_combined.txt")]
struct HelloTemplate<'a> {
    title: &'a str,
    mood: Mood,
    meta: Meta,
    date: Date<'a>
}
use chrono::{NaiveDate, Datelike, NaiveDateTime};

#[derive(Debug, Clone, PartialEq)]
struct Date<'a> {
    year: i32,
    month: u32,
    day: u32,
    original_date: Option<&'a Date<'a>>,
}

impl<'a> Date<'a> {
    fn new(year: i32, month: u32, day: u32) -> Self {
        Date {
            year,
            month,
            day,
            original_date: None,
        }
    }

    fn with_original_date(original_date: Date) -> Date {
        // discard the original_date field in the original_date arg
        Date {
            year: original_date.year,
            month: original_date.month,
            day: original_date.day,
            original_date: None,
        }
    }

    fn after(&self, days: u32) -> Date {
        let naive_date = NaiveDate::from_ymd_opt(self.year, self.month, self.day)
            .expect("Invalid date")
            .checked_add_signed(chrono::Duration::days(days as i64))
            .expect("Date addition error");
        let new_date = Date {
            year: naive_date.year(),
            month: naive_date.month(),
            day: naive_date.day(),
            original_date: self.original_date.clone(),
        };

        new_date
    }

    fn before(&self, days: u32) -> Date {
        let naive_date = NaiveDate::from_ymd_opt(self.year, self.month, self.day)
            .expect("Invalid date")
            .checked_sub_signed(chrono::Duration::days(days as i64))
            .expect("Date subtraction error");

        let new_date = Date {
            year: naive_date.year(),
            month: naive_date.month(),
            day: naive_date.day(),
            original_date: self.original_date.clone(),
        };

        new_date
    }

    fn month_day(&self) -> String {
        if self.original_date.is_none() {
            return "今日".to_string();
        }

        let original = self.original_date.unwrap();

        let original_month = original.month;
        let original_year = original.year;
        let original_day = original.day;
        let new_month = self.month;
        let new_year = self.year;
        let new_day = self.day;

        if original_month == new_month && original_year == new_year {
            return format!("今月{}日", self.day);
        }

        if original_year == new_year {
            return format!("{}月{}日", new_month, new_day);
        }

        if new_year == original_year - 1 {
            return format!("昨年{}月{}日", new_month, new_day);
        }

        if new_year == original_year + 1 {
            return format!("来年{}月{}日", new_month, new_day);
        }

        format!("{}年{}月{}日", new_year, new_month, new_day)
    }
}

struct Meta {
}

impl Meta {
    fn get_instruction_manual(&self) -> String {
        return "".to_string();
    }
}

struct Mood {
}

impl Mood {
    fn is_subjective(&self) -> bool {
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
        env: &dyn askama::Values
    ) -> askama::Result<String> {
        Ok(format!("<div class=\"footnote\">{value}</div>"))
    }
}

#[component]
fn App() -> Html {
    let counter = use_state(|| 0);
    let onclick = {
        let counter = counter.clone();
        move |_| {
            let value = *counter + 1;
            counter.set(value);
        }
    };

    html! {
        <div>
            <button {onclick}>{ "+1" }</button>
            <p>{ *counter }</p>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
