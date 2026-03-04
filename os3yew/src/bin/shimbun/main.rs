// しんぶんからひろがっていくせかい

use askama::Template;
use comrak::{Options, markdown_to_html, options::Render};
use gloo_timers::callback::{Interval, Timeout};
// use gloo_net::http::Request;
use log::{info, warn};
use rand::{
    Rng, RngExt, SeedableRng, distr::slice::Choose, random_range, rng, rngs::StdRng,
    seq::IndexedRandom,
};
use regex::Regex;
use rust_embed::RustEmbed;
// use rustybuzz::{UnicodeBuffer, shape};
use std::{
    clone,
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::{BuildHasher, Hash, RandomState},
    ops::Add,
    rc::Rc,
};
use web_sys::{Document, Window, console, window};
use yew::{Html, html::IntoPropValue, prelude::*, virtual_dom::VNode};

#[derive(RustEmbed)]
#[folder = "metadata"]
struct Asset;

#[derive(Template, Debug, Clone, PartialEq)]
#[template(path = "text_combined.txt")]
struct ArticleTemplate {
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
    condition_seed: u64,
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
            condition_seed: rng().next_u64(),
        }
    }

    fn move_origin(&self) -> Date {
        // discard the original_date field in the original_date arg
        Date {
            year: self.year.clone(),
            month: self.month.clone(),
            day: self.day.clone(),
            original_date: None,
            condition_seed: rng().next_u64(),
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
            condition_seed: self.condition_seed,
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
            condition_seed: self.condition_seed,
        };

        new_date
    }

    fn month_day(&self) -> String {
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
struct Meta {
    window_width: f64,
    window_height: f64,
    global_pos_x: f64,
    global_pos_y: f64,
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
    fn get_instruction_manual(&self) -> String {
        return "下にあるボタンを押すと、今読んでいる文章が消えていきます。そうしてしばらくすると、色々な言葉の断片が浮かび上がっていきます。その言葉の断片の上にマウスカーソルを置いて、マウスを押し続けると、新たな文章が浮かび上がっていきます。".to_string();
    }
}

#[derive(PartialEq, Debug, Clone)]
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

fn get_availiable_titles(
    done_titles: &HashSet<String, RandomState>,
    all_titles: &HashSet<String, RandomState>,
) -> HashSet<String, RandomState> {
    if done_titles.len() == 0 {
        return HashSet::from_iter(vec!["注意".to_string()].into_iter());
    } else {
        let mut tmp = done_titles.clone();
        tmp.insert("「大大補償大会」開催決定".to_string());
        if tmp.len() == all_titles.len() {
            return HashSet::from_iter(vec!["「大大補償大会」開催決定".to_string()].into_iter());
        } else {
            let mut tmp2 = all_titles.clone();
            tmp2.remove("「大大補償大会」開催決定");
            return tmp2
                .difference(&done_titles)
                .into_iter()
                .map(|x| x.clone())
                .collect();
        }
    }
}

#[derive(Clone, PartialEq)]
struct RectMask {
    w: f64,
    h: f64,
    x: f64,
    y: f64,
}

#[derive(Clone, PartialEq)]
struct Article {
    template: ArticleTemplate,
    w: Option<f64>,
    h: Option<f64>,
    x: f64,
    y: f64,
    masks: Vec<RectMask>,
}

struct ParsedDomRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    top: f64,
    right: f64,
    bottom: f64,
    left: f64,
}

fn get_bounding_from_id(elem_id: &str) -> Option<ParsedDomRect> {
    let target = window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(elem_id);

    target.map(|x| {
        let bounding = x.get_bounding_client_rect();
        ParsedDomRect {
            x: bounding.x(),
            y: bounding.y(),
            width: bounding.width(),
            height: bounding.height(),
            left: bounding.left(),
            top: bounding.top(),
            bottom: bounding.bottom(),
            right: bounding.right(),
        }
    })
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum GameStage {
    ArticleView,
    ArticleHidden,
    ForecastStart,
    ElectionStart,
    Cleanup,
}

#[component]
fn App() -> Html {
    let render_number = use_state(|| 0);

    // current article
    let current_article = use_state(|| {
        Some(Article {
            template: ArticleTemplate {
                title: "注意".to_string(),
                mood: Mood {},
                meta: Default::default(),
                date: Box::new(Date::new(2026, 2, 13)),
            },
            w: Some(1000.0),
            h: None,
            x: 30.0,
            y: 30.0,
            masks: Vec::new(),
        })
    });

    // counter for keeping consistent element keys and ids
    let counter_keygen: UseStateHandle<u32> = use_state(|| 0);

    // candidates of next articles
    let forecasts: UseStateHandle<Vec<Option<Article>>> = use_state(|| Vec::new());

    // articles that have been read
    let done_titles: UseStateHandle<HashSet<String, RandomState>> = use_state(|| {
        let mut h = HashSet::new();
        h.insert("注意".to_string());
        h
    });

    // all titles (Actually, "title" in this regard is different from title (heading) shown on the
    // display. Rather, it acts as an internal ID, and has more strict naming rules than "title"
    // that will be displayed.)
    let all_titles: UseStateHandle<HashSet<String, RandomState>> = use_state(|| {
        let file = Asset::get("titles.json").expect("titles.json not found in static folder");
        let data = &file.data;
        let h = HashSet::from_iter(
            serde_json::from_slice::<Vec<String>>(data)
                .expect("JSON parse error")
                .into_iter(),
        );
        return h;
    });

    // ;; transition condition ;;       ;; function name ;;    ;; description ;;                      ;; GameStage ;;
    //                               => input_hide_article     (watch mouse input, current article    | ArticleView
    //                                                         starts to fade away)                   |
    //
    // (counter_hide_article filled) => advance_hide_article   (fading continues and cannot go back)  | ArticleHidden
    //
    // (timer)                       => advance_show_forecasts (several "forecasts" are created and   | ForecastStart
    //                                                          show up; can be parallel with         |
    //                                                          advance_hide_article)                 |
    //
    // (timer)                       => input_election         (player chooses which article to       | ElectionStart
    //                                                          go next)                              |
    //
    // (when a mask gets big enough) => advance_elect_article  (election completed; remove            | Cleanup
    //                                                          all forecasts; the biggest forecast   |
    //                                                          will be copied into current_article)  |
    let game_stage = use_state(|| GameStage::ArticleView);
    let transition_history: UseStateHandle<Vec<(f64, GameStage)>> =
        use_state(|| vec![(0.0, GameStage::ArticleView)]);

    let advance_hide_article: Callback<MouseEvent> =
        use_callback((game_stage.clone()), |_, (game_stage)| {
            game_stage.set(GameStage::ArticleHidden);
        });

    let advance_show_forecasts = use_callback(
        (
            current_article.clone(),
            done_titles.clone(),
            all_titles.clone(),
            counter_keygen.clone(),
        ),
        move |(), (current_article, done_titles, all_titles, counter)| {
            let done_titles_ref = (**done_titles).clone();
            let all_titles_ref = (**all_titles).clone();
            let availiable_titles = get_availiable_titles(&done_titles_ref, &all_titles_ref);

            counter.set(**counter + 1);
            if current_article.is_none() {
                return;
            }

            let mut rng = rand::rng();

            let days_skip: u32 = random_range(3..25);

            let chosen = availiable_titles
                .iter()
                .collect::<Vec<_>>()
                .choose(&mut rng)
                .expect("titles.json contained no titles")
                .as_str();

            let template = &current_article.as_ref().unwrap().template;

            let ht = ArticleTemplate {
                title: chosen.to_string(),
                mood: template.mood.clone(),
                meta: template.meta.clone(),
                date: Box::new(template.date.clone().after(days_skip).move_origin()),
            };

            current_article.set(Some(Article {
                template: ht,
                w: Some(1000.0),
                h: None,
                x: 30.0,
                y: 30.0,
                masks: Vec::new(),
            }));
            let mut dt = done_titles_ref.clone();
            dt.insert(chosen.to_string());
            done_titles.set(dt);

            //            if fonts.is_some() {
            //                let data = fonts.as_ref().unwrap().get("GenEiKoburiMin6-R").unwrap();
            //                let face = rustybuzz::Face::from_slice(data, 0).unwrap();
            //                let mut buf = UnicodeBuffer::new();
            //                buf.push_str("ああああああabcあいいabllm感じ感じ");
            //                let result = shape(&face, &[], buf);
            //                message.set(format!("{:?}", result));
            //            }
        },
    );

    let advance_elect_article = use_callback((), move |(), ()| {});

    let clock_callback = use_callback(
        (
            transition_history.clone(),
            game_stage.clone(),
            advance_show_forecasts.clone(),
        ),
        |(delta, culmative), (transition_history, game_stage, advance_show_forecasts)| {
            let last_gs = transition_history.last().unwrap().1;
            if **game_stage != last_gs {
                let mut new_th = (**transition_history).clone();
                new_th.push((culmative, **game_stage));
                transition_history.set(new_th);
                return;
            }

            if **game_stage == GameStage::ArticleHidden
                && culmative - transition_history.last().unwrap().0 > 10.0
            {
                game_stage.set(GameStage::ForecastStart);
                advance_show_forecasts.emit(());
                return;
            }

            console::log_1(&format!("{:?}", &**transition_history).into());
        },
    );

    //    use_effect_with((), move |_| {
    //        let fontname = "GenEiKoburiMin6-R";
    //        let font_url = "assets/".to_string() + fontname + ".ttf";
    //        if fonts.is_none() {
    //            wasm_bindgen_futures::spawn_local(async move {
    //                let data = Request::get(&font_url)
    //                    .send()
    //                    .await
    //                    .unwrap()
    //                    .binary()
    //                    .await
    //                    .unwrap();
    //                let mut new_hash = HashMap::new();
    //                new_hash.insert(fontname.to_string(), data);
    //                fonts.set(Some(new_hash));
    //            });
    //        }
    //    });
    //
    let data_table: Rc<Vec<Option<(bool, HashMap<String, String>, usize)>>> =
        use_memo((current_article.clone(), forecasts.clone()), |(ca, fc)| {
            let mut arr: Vec<Option<(bool, HashMap<String, String>, usize)>> = Vec::new();

            let insert_data = |is_current_article,
                               target: Option<&Article>,
                               arr: &mut Vec<Option<(bool, HashMap<String, String>, usize)>>,
                               idx: usize| {
                let dt = make_data_table(target.unwrap().template.clone().render().unwrap());

                arr.push(Some((is_current_article, dt, 0)))
            };

            if ca.is_some() {
                insert_data(true, ca.as_ref(), &mut arr, 0);
            } else {
                arr.push(None)
            }

            fc.iter().enumerate().for_each(|(i, x)| {
                if x.is_some() {
                    insert_data(false, x.as_ref(), &mut arr, i);
                } else {
                    arr.push(None)
                }
            });
            arr
        });

    let gen_article_id = |counter: u32, idx: usize, is_current: bool| {
        if is_current {
            format!("article-{}-current", counter)
        } else {
            format!("article-{}-forecast-{}", counter, idx)
        }
    };

    let upgrade_plan: UseStateHandle<Vec<(bool, usize, (f64, f64))>> = use_state(|| Vec::new());

    let upgrade_plan_check = use_callback::<bool, _, _, _>(
        (
            current_article.clone(),
            forecasts.clone(),
            counter_keygen.clone(),
            upgrade_plan.clone(),
            render_number.clone(),
        ),
        move |(first_render),
              (current_article, forecasts, counter, upgrade_plan_in, render_number)| {
            let mut all_articles = vec![(current_article.as_ref(), 0, true)];
            all_articles.append(
                &mut forecasts
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (x.as_ref(), i, false))
                    .collect(),
            );
            let no_need_upgrade = all_articles.iter().all(|(a, i, is_current)| {
                if let Some(a) = a {
                    if a.w.is_some() && a.h.is_some() {
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            });
            let mut not_yet_rendered = false;
            let upgrade_plan = if !no_need_upgrade {
                all_articles
                    .iter()
                    .map(|(a, i, is_current)| {
                        if a.is_some() {
                            let elem_id = gen_article_id(**counter, *i, *is_current);
                            console::log_1(&format!("{}", elem_id).into());
                            let rect = get_bounding_from_id(&elem_id);
                            if let Some(rect) = rect {
                                Some((*is_current, *i, (rect.width, rect.height)))
                            } else {
                                not_yet_rendered = true;
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            console::log_1(&format!("{:?} {:?}", upgrade_plan, no_need_upgrade).into());

            if not_yet_rendered {
                render_number.set(**render_number + 1);
            }

            upgrade_plan_in.set(upgrade_plan.into_iter().filter_map(|x| x).collect());
        },
    );

    {
        let current_article = current_article.clone();
        let forecasts = forecasts.clone();
        let upgrade_plan = upgrade_plan.clone();
        use_effect(move || {
            if upgrade_plan.is_empty() {
                return;
            }
            let mut forecasts_new = (&*forecasts).clone();
            upgrade_plan.iter().for_each(|(is_current, idx, (w, h))| {
                if *is_current {
                    if let Some(a) = current_article.as_ref() {
                        let mut a = a.clone();
                        a.w = Some(*w);
                        a.h = Some(*h);
                        current_article.set(Some(a));
                    }
                } else {
                    if let Some(Some(a)) = forecasts.get(*idx) {
                        let mut a = a.clone();

                        a.w = Some(*w);
                        a.h = Some(*h);
                        forecasts_new[*idx] = Some(a);
                    }
                }
            });
            forecasts.set(forecasts_new);
            upgrade_plan.set(Vec::new());
        });
    }

    let gen_article_style = |is_current_article, data, article_ref: Option<&Article>| {
        if let Some(article_ref) = article_ref {
            let mut style = "".to_string();
            if let Some(w) = article_ref.w {
                style += &format!("width: {:.4}px;", w);
            } else {
                style += "width: auto;"
            }
            if let Some(h) = article_ref.h {
                style += &format!("height: {:.4}px;", h);
            } else {
                style += "height: auto;"
            }
            style += "position: absolute;";
            style += &format!("top: {:.4}px;", article_ref.y);
            style += &format!("left: {:.4}px;", article_ref.x);
            return style;
        } else {
            return "display: none;".to_string();
        }
    };

    let gen_article_class = |is_current_article, data, article_ref: Option<&Article>| {
        let mut classes = Vec::new();
        classes.push("article".to_string());
        if (is_current_article) {
            classes.push("current-article".to_string());
        } else {
            classes.push("forecast".to_string());
        }
        return classes.join(" ");
    };

    // temp0.getBoundingClientRect()

    let html_articles: Vec<_> = data_table
        .iter()
        .filter(|x| x.is_some())
        .map(|x| {
            let (is_current, data, idx) = x.as_ref().unwrap();

            let article_ref = {
                if *is_current {
                    current_article.as_ref()
                } else {
                    forecasts.get(*idx).unwrap().as_ref()
                }
            };

            let elem_id = gen_article_id(*counter_keygen, *idx, *is_current);

            html! {
                <div class={gen_article_class(*is_current, data, article_ref)} id={elem_id.clone()} key={elem_id.clone()} style={gen_article_style(*is_current, data, article_ref)}>
                    <span>
                    { article_ref.unwrap().template.date.year.to_string()} {"年"}
                    { article_ref.unwrap().template.date.month.to_string() } {"月"}
                    { article_ref.unwrap().template.date.day.to_string()} {"日"}
                    </span>
                    if data.get("title").is_some() {
                    <h1>
                    {
                        data.get("title").unwrap()
                    }
                    </h1>
                    }
                    if data.get("text").is_some() {
                    <div>
                    {
                        md(data.get("text").unwrap().clone())
                    }
                    </div>
                    }
                    if data.get("images").is_some() {
                    <div>
                    {
                        md(data.get("images").unwrap().clone())
                    }
                    </div>
                    }
                    if data.get("image_caption").is_some() {
                    <div>
                    {
                        md(data.get("image_caption").unwrap().clone())
                    }
                    </div>
                    }
                    <button onclick={advance_hide_article.clone()}>{"読み続ける"}</button>
                </div>
            }
        })
        .collect();

    html! {
        <div class="app-wrapper">
            {html_articles}

            <RenderWatchComponent render_number={*render_number} callback={upgrade_plan_check}><></></RenderWatchComponent>
            <ClockComponent callback={clock_callback} interval={42} />
        </div>
    }
}

#[derive(PartialEq, Properties)]
pub struct RenderWatchProps {
    callback: Callback<bool, ()>,
    render_number: u64,
    children: Html,
}

pub struct RenderWatchComponent {
    timeout: Option<Timeout>,
}

impl Component for RenderWatchComponent {
    type Message = ();

    type Properties = RenderWatchProps;

    fn create(ctx: &Context<Self>) -> Self {
        RenderWatchComponent { timeout: None }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html!(<div data-render-number={ ctx.props().render_number.to_string() }> {ctx.props().children.clone()} </div>)
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        let c = ctx.props().callback.clone();
        self.timeout = Some(Timeout::new(10, move || {
            c.emit(first_render);
        }));
    }
}

#[derive(PartialEq, Properties)]
pub struct ClockProps {
    callback: Callback<(f64, f64), ()>,
    interval: u32,
}

pub struct ClockComponent {
    timeout: Option<Timeout>,
    culmative: f64,
}

#[derive(PartialEq)]
pub struct ClockMessage {
    delta: Option<f64>,
}

impl Component for ClockComponent {
    type Message = ClockMessage;

    type Properties = ClockProps;

    fn create(ctx: &Context<Self>) -> Self {
        let c = ctx.props().callback.clone();
        let i = ctx.props().interval;
        let l = ctx.link().clone();
        ClockComponent {
            timeout: Some(Timeout::new(i, move || {
                c.emit((i as f64 / 1000.0, l.get_component().unwrap().culmative)); // TODO: actual delta calc
                l.send_message(ClockMessage {
                    delta: Some(i as f64 / 1000.0),
                }); // TODO: actual delta calc
            })),
            culmative: 0.0,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        if let Some(delta) = msg.delta {
            self.culmative += delta;
            let c = ctx.props().callback.clone();
            let i = ctx.props().interval;
            let l = ctx.link().clone();
            self.timeout = Some(Timeout::new(i, move || {
                c.emit((i as f64 / 1000.0, l.get_component().unwrap().culmative)); // TODO: actual delta calc
                l.send_message(ClockMessage {
                    delta: Some(i as f64 / 1000.0),
                }); // TODO: actual delta calc
            }));
            return false;
        }
        return true;
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html!(<></>)
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
