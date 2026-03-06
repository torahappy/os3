use comrak::{Options, markdown_to_html, options::Render};
use regex::Regex;
// use rustybuzz::{UnicodeBuffer, shape};
use std::
    collections::HashMap
;
use yew::{Html, prelude::*, virtual_dom::VNode};

pub fn nl2br(text: &str) -> Html {
    let mut nodes = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            nodes.push(html! { <br/> });
        }
        nodes.push(html! { {line} });
    }
    html! { {for nodes} }
}

pub fn dangerous_raw_html(raw_html_string: String) -> VNode {
    return Html::from_html_unchecked(AttrValue::from(raw_html_string));
}

pub fn md(md_str: String) -> VNode {
    dangerous_raw_html(markdown_to_html(
        &md_str,
        &Options {
            render: Render {
                r#unsafe: true,
                ..Default::default()
            },
            ..Default::default()
        },
    ))
}

pub fn make_data_table(str_in: String) -> HashMap<String, String> {
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



