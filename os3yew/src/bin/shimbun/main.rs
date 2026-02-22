// しんぶんからひろがっていくせかい

use askama::Template;
use comrak::{Options, markdown_to_html, options::Render};
use log::warn;
use rand::{random_range, seq::IndexedRandom};
use regex::Regex;
use rust_embed::RustEmbed;
use std::{
    collections::{HashMap, HashSet},
    default,
    fmt::Display,
    hash::RandomState,
};
use yew::{Html, html::IntoPropValue, prelude::*, virtual_dom::VNode};

#[derive(RustEmbed)]
#[folder = "metadata"]
struct Asset;

#[derive(Template, Debug, Clone)]
#[template(path = "text_combined.txt")]
struct HelloTemplate {
    title: String,
    mood: Mood,
    meta: Meta,
    date: Box<Date>,
}
use chrono::{Datelike, NaiveDate, NaiveDateTime};

#[derive(Debug, Clone, PartialEq)]
struct Date {
    year: i32,
    month: u32,
    day: u32,
    original_date: Option<Box<Date>>,
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'a> Date {
    fn new(year: i32, month: u32, day: u32) -> Self {
        Date {
            year,
            month,
            day,
            original_date: None,
        }
    }

    fn move_origin(&self) -> Date {
        // discard the original_date field in the original_date arg
        Date {
            year: self.year.clone(),
            month: self.month.clone(),
            day: self.day.clone(),
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
            original_date: Some(Box::new(self.clone())),
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
            original_date: Some(Box::new(self.clone())),
        };

        new_date
    }

    fn month_day(&self) -> String {
        // If this is the original date, we call it "今日"
        if self.original_date.is_none() {
            return "今日".to_string();
        }

        let original = self.original_date.as_ref().unwrap();

        // Prepare naive dates for day‑difference calculation
        let naive_self =
            NaiveDate::from_ymd_opt(self.year, self.month, self.day).expect("Invalid date");
        let naive_orig = NaiveDate::from_ymd_opt(original.year, original.month, original.day)
            .expect("Invalid date");

        // Day difference in days (positive → self is later)
        let day_diff: i64 = (naive_self - naive_orig).num_days();

        // Yesterday / Tomorrow logic – check first
        match day_diff {
            1 => return "明日".to_string(),
            -1 => return "昨日".to_string(),
            _ => (), // continue with month‑based logic
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
            -1 => return format!("先月{}日", self.day),
            1 => return format!("来月{}日", self.day),
            _ => (), // fall‑through for other cases
        }

        // Same month & year – “今月…”
        if orig_month == new_month && orig_year == new_year {
            return format!("今月{}日", self.day);
        }

        // Same year but different month – “X月…”
        if orig_year == new_year {
            return format!("{}月{}日", new_month, self.day);
        }

        // Previous year – “昨年…”
        if new_year == orig_year - 1 {
            return format!("昨年{}月{}日", new_month, self.day);
        }

        // Next year – “来年…”
        if new_year == orig_year + 1 {
            return format!("来年{}月{}日", new_month, self.day);
        }

        // Fallback – full date
        format!("{}年{}月{}日", new_year, new_month, self.day)
    }
}

#[derive(Debug, Clone)]
struct Meta {}

impl Meta {
    fn get_instruction_manual(&self) -> String {
        return "左上の+1ボタンをクリック".to_string();
    }
}

#[derive(Debug, Clone)]
struct Mood {}

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
        env: &dyn askama::Values,
    ) -> askama::Result<String> {
        Ok(format!("<div class=\"footnote\">{value}</div>"))
    }
}

fn nl2br(text: &str) -> Html {
    let mut nodes = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            nodes.push(html! { <br/> });
        }
        nodes.push(html! { {line} });
    }
    html! { {for nodes} }
}

fn dangerous_raw_html(raw_html_string: String) -> VNode {
    return Html::from_html_unchecked(AttrValue::from(raw_html_string));
}

fn md(md_str: String) -> VNode {
    dangerous_raw_html(markdown_to_html(&md_str, &Options{
        render: Render {
            r#unsafe: true,
            ..Default::default()
        },
        ..Default::default()
    }))
}

fn make_data_table(str_in: String) -> HashMap<String, String> {
    let key_re = Regex::new(r"^\s*\\?\[([^\]]+?)\\?\]\s*$").unwrap();

    let mut table: HashMap<String, String> = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut buffer: Vec<String> = Vec::new();

    for raw_line in str_in.lines() {
        let line = raw_line;

        if let Some(caps) = key_re.captures(line) {
            if let Some(k) = current_key.take() {
                let value = buffer.join("\n").trim().to_string();
                table.insert(k, value);
                buffer.clear();
            }

            current_key = Some(caps[1].to_string());
        } else if current_key.is_some() {
            buffer.push(line.to_string());
        }
    }

    if let Some(k) = current_key {
        let value = buffer.join("\n").trim().to_string();
        table.insert(k, value);
    }

    table
}

#[component]
fn App() -> Html {
    let template = use_state(|| None::<HelloTemplate>);
    let availiable_titles: UseStateHandle<HashSet<String, RandomState>> = use_state(|| {
        let file = Asset::get("titles.json").expect("titles.json not found in static folder");
        let data = &file.data;
        let mut h = HashSet::from_iter(
            serde_json::from_slice::<Vec<String>>(data)
                .expect("JSON parse error")
                .into_iter(),
        );
        h.remove("「大大補償大会」開催決定");
        return h;
    });
    let onclick = {
        let template = template.clone();
        let availiable_titles = availiable_titles.clone();
        move |_| {
            let mut rng = rand::rng();

            let days_skip: u32 = random_range(3..25);


            let chosen = if availiable_titles.len() == 0 {
                "「大大補償大会」開催決定"
            } else if template.is_none() {
                "注意"
            } else {
                availiable_titles
                    .iter()
                    .collect::<Vec<_>>()
                    .choose(&mut rng)
                    .expect("titles.json contained no titles")
                    .as_str()
            };

            let ht = if template.is_none() {
                HelloTemplate {
                    title: chosen.to_string(),
                    mood: Mood {},
                    meta: Meta {},
                    date: Box::new(Date::new(2026, 2, 13)),
                }
            } else {
                let u = template.as_ref().unwrap().clone();
                HelloTemplate {
                    title: chosen.to_string(),
                    mood: Mood {},
                    meta: Meta {},
                    date: Box::new(u.date.clone().after(days_skip).move_origin()),
                }
            };
            template.set(Some(ht.clone()));
            availiable_titles.set(
                availiable_titles
                    .difference(&HashSet::from([chosen.to_string()]))
                    .map(|x| x.clone())
                    .collect(),
            );
        }
    };

    let data_table = if template.is_some() {
        Some(make_data_table(
            template.clone().as_ref().unwrap().render().unwrap(),
        ))
    } else {
        None
    };

    html! {
        <div>
            <button {onclick}>{ "+1" }</button>
            if template.is_some() && data_table.is_some(){
                <div>
                <span>
                { template.as_ref().unwrap().date.year } {"年"}
                { template.as_ref().unwrap().date.month } {"月"}
                { template.as_ref().unwrap().date.day } {"日"}
                </span>
                if data_table.as_ref().unwrap().get("title").is_some() {
                <h1>
                {
                    data_table.as_ref().unwrap().get("title").unwrap()
                }
                </h1>
                }
                if data_table.as_ref().unwrap().get("text").is_some() {
                <div>
                {
                    md(data_table.as_ref().unwrap().get("text").unwrap().clone())
                }
                </div>
                }
                if data_table.as_ref().unwrap().get("images").is_some() {
                <div>
                {
                    md(data_table.as_ref().unwrap().get("images").unwrap().clone())
                }
                </div>
                }
                if data_table.as_ref().unwrap().get("image_caption").is_some() {
                <div>
                {
                    md(data_table.as_ref().unwrap().get("image_caption").unwrap().clone())
                }
                </div>
                }
                </div>
            }
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
