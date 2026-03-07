// しんぶんからひろがっていくせかい

mod data;

use crate::data::util::*;
use crate::data::*;
use askama::Template;
use gloo_timers::callback::Timeout;
use os3yew::{
    components::{ClockComponent, RenderWatchComponent},
    util::*,
};
use rand::{
    distr::{Distribution, Uniform},
    random_range, rng,
    seq::IndexedRandom,
};
// use rustybuzz::{UnicodeBuffer, shape};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    hash::RandomState,
    rc::Rc,
};
use web_sys::{console, window};
use yew::{Html, prelude::*};

pub fn get_availiable_titles(
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

    // ## legend
    // transition condition => prominent function name (function desctiption) GameStage
    //
    // ## desctiptions
    // clickevt_fade_article => noop (just changing className) ArticleFading
    //
    // timer => input_election, advance_show_forecasts (several "forecasts" are created and show up)
    // ForecastStart
    //
    // when a mask gets big enough => advance_elect_article (election completed; remove
    // all forecasts; the biggest forecast will be copied into current_article)

    // Game stage
    let game_stage = use_state(|| GameStage::ArticleView);

    // Game State change history with elapsed time
    let transition_history: UseStateHandle<Vec<(f64, GameStage)>> =
        use_state(|| vec![(0.0, GameStage::ArticleView)]);

    // transit ArticleView -> ArticleFading instantly
    let clickevt_fade_article: Callback<MouseEvent> =
        use_callback((game_stage.clone()), |_, (game_stage)| {
            if **game_stage == GameStage::ArticleView {
                game_stage.set(GameStage::ArticleFading);
            }
        });
    let precalc_for_rects: UseStateHandle<Option<QuadNode<(usize, usize)>>> = use_state(|| None);

    // when forecasts change, if there are no masks, try generate initial masks. if the metrics
    // aren't filled yet, do nothing for the article.
    use_effect_with(
        (precalc_for_rects.clone(), forecasts.clone()),
        |(precalc_for_rects, forecasts)| {
            let next_forecasts = forecasts
                .iter()
                .map(|x| {
                    if let Some(article) = x {
                        if article.w.is_some()
                            && article.h.is_some()
                            && article.masks.iter().all(|x| x.is_none())
                        {
                            let n = (article.w.unwrap() * article.h.unwrap() / 5445.0).max(10.0)
                                as usize;
                            let mut next_article = article.clone();
                            next_article.masks = gen_random_masks(
                                article.w.unwrap(),
                                article.h.unwrap(),
                                n,
                                40.0,
                                40.0,
                            )
                            .into_iter()
                            .map(|x| Some(x))
                            .collect::<Vec<_>>();

                            return Some(next_article);
                        }
                    }
                    return x.clone();
                })
                .collect::<Vec<_>>();

            let bound = next_forecasts
                .iter()
                .filter_map(|x| x.as_ref())
                .map(|x| x.masks.iter().filter_map(|x| x.clone()))
                .flatten()
                .collect::<Vec<_>>();

            if bound.len() != 0 {
                let mut quad_data: QuadNode<(usize, usize)> =
                    QuadNode::new(RectMask::bounding_rect(bound).unwrap());

                for (i, a) in forecasts.iter().enumerate() {
                    if let Some(a) = a {
                        for (j, x) in a.masks.iter().enumerate() {
                            if let Some(x) = x {
                                quad_data.insert((i, j), x.clone(), 4);
                            }
                        }
                    }
                }

                precalc_for_rects.set(Some(quad_data));
            }
            forecasts.set(next_forecasts);
        },
    );

    // transit ArticleFading -> ForecastStart instantly;
    let advance_show_forecasts = use_callback(
        (
            current_article.clone(),
            done_titles.clone(),
            all_titles.clone(),
            counter_keygen.clone(),
            forecasts.clone(),
        ),
        move |(), (current_article, done_titles, all_titles, counter, forecasts)| {
            let done_titles_ref = (**done_titles).clone();
            let all_titles_ref = (**all_titles).clone();
            let availiable_titles = get_availiable_titles(&done_titles_ref, &all_titles_ref);

            counter.set(**counter + 1);
            if current_article.is_none() {
                return;
            }

            let mut rng = rand::rng();

            let days_skip: u32 = random_range(3..25);

            let sample_size = availiable_titles.len().min(3);

            if sample_size == 0 {
                return;
            } // TODO: implement the game ending

            let template = &current_article.as_ref().unwrap().template;

            let chosen = availiable_titles
                .iter()
                .collect::<Vec<_>>()
                .sample(&mut rng, sample_size)
                .map(|&x| {
                    let t = ArticleTemplate {
                        title: x.clone(),
                        mood: template.mood.clone(),
                        meta: template.meta.clone(),
                        date: Box::new(template.date.clone().after(days_skip).move_origin()),
                    };
                    Some(Article {
                        template: t,
                        w: Some(1000.0),
                        h: None,
                        x: 30.0,
                        y: 30.0,
                        masks: Vec::new(),
                    })
                })
                .collect::<Vec<_>>();

            forecasts.set(chosen);
        },
    );

    // ticking funciton. Most of the "timeout" funcitons and the watching funcitons for GameStage
    // should be put here.
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

            if **game_stage == GameStage::ArticleFading
                && culmative - transition_history.last().unwrap().0 > 10.0
                && **game_stage != GameStage::ForecastStart
            {
                advance_show_forecasts.emit(());
                game_stage.set(GameStage::ForecastStart);
                return;
            }

            console::log_1(&format!("{:?}", &**transition_history).into());
        },
    );

    // obtain the article text data
    let data_table: Rc<Vec<Option<(bool, HashMap<String, String>, usize)>>> =
        use_memo((current_article.clone(), forecasts.clone()), |(ca, fc)| {
            let mut arr: Vec<Option<(bool, HashMap<String, String>, usize)>> = Vec::new();

            let insert_data = |is_current_article,
                               target: Option<&Article>,
                               arr: &mut Vec<Option<(bool, HashMap<String, String>, usize)>>,
                               idx: usize| {
                let dt = make_data_table(target.unwrap().template.clone().render().unwrap());

                arr.push(Some((is_current_article, dt, idx)))
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

    // get article id
    let gen_article_id = |counter: u32, idx: usize, is_current: bool| {
        if is_current {
            format!("article-{}-current", counter)
        } else {
            format!("article-{}-forecast-{}", counter, idx)
        }
    };

    // "upgrade" Article structure. If there are width/height field that are not filled yet, try
    // invoking getBoundingClientRect API and fetch the metrics.
    let upgrade_plan: UseStateHandle<Vec<(bool, usize, (f64, f64))>> = use_state(|| Vec::new());

    // invokes getBoundingClientRect for each elements with unfetched metrics. Also, if there are
    // any inconsistencies between the component state and the actual rendered DOM, then query
    // re-fetching through some Timeout.
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

    // Apply fetched upgrade plan to the component state.
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

    // get style attr for each article.
    let gen_article_style =
        |is_current_article: bool, data, article_ref: Option<&Article>, idx: usize| {
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
                if !is_current_article {
                    style += &format!(
                        "mask: url(\"#{}-mask\");",
                        gen_article_id(*counter_keygen, idx, is_current_article)
                    );
                }
                return style;
            } else {
                return "display: none;".to_string();
            }
        };

    // gen class attr for each article
    let gen_article_class = |is_current_article, data, article_ref: Option<&Article>| {
        let mut classes = Vec::new();
        classes.push("article".to_string());
        if is_current_article {
            classes.push("current-article".to_string());
        } else {
            classes.push("forecast".to_string());
        }
        if is_current_article {
            if GameStage::ArticleView <= *game_stage && *game_stage <= GameStage::ArticleFading {
                classes.push("visible".to_string());
                if *game_stage == GameStage::ArticleFading {
                    classes.push("fading".to_string());
                }
            } else {
                classes.push("hidden".to_string());
            }
        }
        return classes.join(" ");
    };

    let mouse_move_evt = use_callback((precalc_for_rects.clone()), |me: MouseEvent, precalc_for_rects| {
        let x = me.x();
        let y = me.y();
        if let Some(precalc_for_rects) = precalc_for_rects.as_ref() {
            console::log_1(&format!("aaaaaa").into());
            let mut r = vec![];
            precalc_for_rects.query(&RectMask::new(x as f64, y as f64, 1.0, 1.0), &mut r);
            console::log_1(&format!("{:?}", r).into());
        }
    });

    // virtual dom for articles.
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

            if article_ref.is_none() { return html!(<></>) };

            html! {
<>
    <svg height="0" xmlns="http://www.w3.org/2000/svg">
        <mask id={elem_id.clone() + "-mask"} mask-type="alpha">
            for (i, x) in article_ref.unwrap().masks.clone().into_iter().enumerate() {
                if let Some(x) = x {
                    <rect key={i} x={format!("{:.4}", x.x)} y={format!("{:.4}", x.y)} width={format!("{:.4}", x.w)} height={format!("{:.4}", x.h)} fill="white" />
                }
            }
        </mask>
    </svg>
    <div class={gen_article_class(*is_current, data, article_ref)} id={elem_id.clone()} key={elem_id.clone()} style={gen_article_style(*is_current, data, article_ref, *idx)}>
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
        if data.get("images").is_some() {
        <div>
        {
            md(data.get("images").unwrap().clone())
        }
        </div>
        }
        if data.get("image_caption_work_title").is_some() {
        <div>
        {
            md(data.get("image_caption_work_title").unwrap().clone())
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
        if data.get("text").is_some() {
        <div>
        {
            md(data.get("text").unwrap().clone())
        }
        </div>
        }
        <button onclick={clickevt_fade_article.clone()}>{"読み続ける"}</button>
    </div>
</>
            }
        })
        .collect();


    // the final virtual dom
    html! {
    <div class="app-wrapper" onmousemove={mouse_move_evt}>
        {html_articles}

        <RenderWatchComponent render_number={*render_number} callback={upgrade_plan_check}><></></RenderWatchComponent>
        <ClockComponent callback={clock_callback} interval={42} />
        <svg height="0" xmlns="http://www.w3.org/2000/svg">
            <filter id="forecast-filter">
                <feComponentTransfer>
                    <feFuncR type="gamma" amplitude="3" exponent="9" offset="0"></feFuncR>
                    <feFuncG type="gamma" amplitude="3" exponent="9" offset="0"></feFuncG>
                    <feFuncB type="gamma" amplitude="3" exponent="9" offset="0"></feFuncB>
                </feComponentTransfer>
            </filter>
        </svg>
    </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
