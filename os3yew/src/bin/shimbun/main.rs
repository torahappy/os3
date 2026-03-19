// しんぶんからひろがっていくせかい

mod data;
mod locale;

use crate::{data::*, locale::get_system_word};
use askama::Template;
use ordered_float::OrderedFloat;
use os3yew::{
    components::{AudioPlayer, ClockComponent, RenderWatchComponent},
    util::*,
};
use rand::{random_range, seq::IndexedRandom};
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::{console, window};
// use rustybuzz::{UnicodeBuffer, shape};
use std::{
    collections::{HashMap, HashSet},
    hash::RandomState,
    rc::Rc,
    sync::LazyLock,
};
use yew::{Html, prelude::*};

const BTN_HEIGHT: f64 = 90.0;
const LOCK_SIZE: f64 = 110.0;
const TIME_TILL_LOCK: f64 = 7.0;
const LANGUAGES: LazyLock<HashSet<String, RandomState>> = std::sync::LazyLock::new(|| {
    HashSet::from_iter(vec!["en".to_string(), "ja".to_string()].into_iter())
});
const DEFAULT_LANGUAGE: &str = "ja";
const LANGUAGE_OVERWRITE_KEYS: LazyLock<HashSet<String>> = std::sync::LazyLock::new(|| {
    HashSet::from_iter(
        vec![
            "title".to_string(),
            "text".to_string(),
            "ending".to_string(),
            "image_caption_work_title".to_string(),
            "image_caption".to_string(),
        ]
        .into_iter(),
    )
});

fn get_exp_coeff() -> f64 {
    return (LOCK_SIZE.ln() - TIME_TILL_LOCK).exp();
}

fn apply_sizemod(r: &RectMask, sizemod: f64) -> RectMask {
    let mut r_clone = r.clone();
    r_clone.x -= sizemod / 2.;
    r_clone.y -= sizemod / 2.;
    r_clone.w += sizemod;
    r_clone.h += sizemod;
    return r_clone;
}

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

fn get_sizemod_from_time(x: f64) -> f64 {
    let lin = x * (LOCK_SIZE / TIME_TILL_LOCK);
    let exp = x.exp() * get_exp_coeff();
    if x <= TIME_TILL_LOCK {
        return lin;
    } else {
        return exp;
    }
}

/// get (w, h, x, y) data of current article
fn get_article_metrics(meta: &Meta) -> (f64, f64, f64, f64) {
    return (800.0, 800.0, meta.window_width / 2.0 - 400.0, 30.0);
}

#[component]
fn App() -> Html {
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

    let current_language = use_state(|| {
        let s = window().unwrap().location().search().unwrap();
        let usp = web_sys::UrlSearchParams::new_with_str(&s).unwrap();
        if let Some(lang) = usp.get(&"lang") {
            if LANGUAGES.contains(&lang) {
                return lang;
            }
        }
        DEFAULT_LANGUAGE.to_string()
    });

    // current article
    let current_article = use_state(|| {
        let s = window().unwrap().location().search().unwrap();
        let usp = web_sys::UrlSearchParams::new_with_str(&s).unwrap();

        let title = {
            if let Some(current) = usp.get(&"current") {
                if all_titles.contains(&current) {
                    current
                } else {
                    "注意".to_string()
                }
            } else {
                "注意".to_string()
            }
        };
        let am = get_article_metrics(&Default::default());
        Some(Article {
            template: ArticleTemplate {
                title,
                mood: Mood {},
                meta: Default::default(),
                date: Box::new(Date::new(2026, 2, 13, (*current_language).clone())),
            },
            w: Some(am.0),
            h: None,
            x: am.2,
            y: am.3,
            masks: Vec::new(),
            character_metrics: None,
        })
    });

    let render_number = use_state(|| 0);

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

    // ## legend
    // transition condition =>
    // prominent function name
    // (function desctiption)
    // GameStage
    //
    // ## desctiptions
    // clickevt_fade_article =>
    // noop
    // (just changing className)
    // ArticleFading
    //
    // timer =>
    // input_election, advance_show_forecasts
    // (several "forecasts" are created and show up)
    // ForecastStart
    //
    // when a mask gets big enough =>
    // noop
    // (just classname change)
    // Cleanup
    //
    // wait for the animation to be completed =>
    // advance_elect_article
    // (election completed; remove all forecasts;
    //  the biggest forecast will be copied into current_article)
    // set back to ArticleView

    // Game stage
    let game_stage = use_state(|| GameStage::ArticleView);

    // Game State change history with elapsed time
    let transition_history: UseStateHandle<Vec<(f64, GameStage)>> =
        use_state(|| vec![(0.0, GameStage::ArticleView)]);

    // precalc for "mask" rects; keyed by forecast index & mask index
    let precalc_for_rects: UseStateHandle<Option<QuadNode<(usize, usize)>>> = use_state(|| None);

    let precalc_for_characters: UseStateHandle<Option<QuadNode<(usize, String)>>> =
        use_state(|| None);

    let track_iteration_1 = use_state(|| 0 as u32);
    let track_src_1 = use_state(|| "".to_string());

    // when forecasts change, if there are no masks, try generate initial masks. if the metrics
    // aren't filled yet, do nothing for the article.
    use_effect_with(
        (
            precalc_for_rects.clone(),
            forecasts.clone(),
            precalc_for_characters.clone(),
        ),
        |(precalc_for_rects, forecasts, precalc_for_characters)| {
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

            // get possible range of all masks coordinates
            let bound = next_forecasts
                .iter()
                .filter_map(|x| x.as_ref())
                .map(|x| {
                    if x.w.is_some() && x.h.is_some() {
                        Some(RectMask {
                            w: x.w.unwrap(),
                            h: x.h.unwrap(),
                            x: x.x,
                            y: x.y,
                        })
                    } else {
                        None
                    }
                })
                .filter_map(|x| x)
                .collect::<Vec<_>>();

            if bound.len() != 0 {
                let bound_flattened = RectMask::bounding_rect(bound).unwrap();
                let mut quad_data: QuadNode<(usize, usize)> =
                    QuadNode::new(bound_flattened.clone());

                for (i, a) in forecasts.iter().enumerate() {
                    if let Some(a) = a {
                        for (j, x) in a.masks.iter().enumerate() {
                            if let Some(r) = x {
                                let mut r_clone = r.clone();
                                r_clone.x += a.x;
                                r_clone.y += a.y;
                                quad_data.insert((i, j), r_clone, 4);
                            }
                        }
                    }
                }

                let all_chars = forecasts
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        a.as_ref()
                            .map(|a| a.character_metrics.as_ref().map(|m| (i, m, a.x, a.y)))
                            .flatten()
                    })
                    .filter_map(|m| m)
                    .map(|(i, m, x, y)| m.iter().map(|m| (i, m, x, y)).collect::<Vec<_>>())
                    .flatten()
                    .collect::<Vec<_>>();

                let mut char_quad: QuadNode<(usize, String)> =
                    QuadNode::new(bound_flattened.clone());

                all_chars.iter().for_each(|(i, m, x, y)| {
                    let r = RectMask {
                        w: m.width,
                        h: m.height,
                        x: m.left,
                        y: m.top,
                    };
                    if (!bound_flattened.intersects(&r)) {
                        return;
                    }
                    char_quad.insert((*i, m.character.clone()), r, 4);
                });

                precalc_for_rects.set(Some(quad_data));
                precalc_for_characters.set(Some(char_quad));
                forecasts.set(next_forecasts);
            }
        },
    );

    let to_be_enlarged: UseStateHandle<Option<(usize, usize)>> = use_state(|| None);
    let to_be_enlarged_lock = use_state(|| false);

    let mouse_move_evt = use_callback(
        (
            precalc_for_rects.clone(),
            to_be_enlarged.clone(),
            to_be_enlarged_lock.clone(),
        ),
        move |me: MouseEvent, (precalc_for_rects, to_be_enlarged, to_be_enlarged_lock)| {
            let x = me.page_x();
            let y = me.page_y();
            if let Some(precalc_for_rects) = precalc_for_rects.as_ref() {
                let mut r = vec![];
                precalc_for_rects.query(&RectMask::new(x as f64, y as f64, 1.0, 1.0), &mut r);
                let mut cands = r.iter().map(|x| &x.0).collect::<Vec<_>>();
                cands.sort(); // TODO: use more funny selection algo

                let t = cands.get(0).map(|&x| x.clone());

                if (!**to_be_enlarged_lock) || to_be_enlarged.is_none() {
                    to_be_enlarged.set(t);
                }
            }
        },
    );

    use_effect_with(
        (
            to_be_enlarged.clone(),
            forecasts.clone(),
            current_language.clone(),
        ),
        move |(to_be_enlarged, forecasts, current_language)| {
            if let Some(precalc_for_characters) = precalc_for_characters.as_ref() {
                if let Some((idx_a, idx_b)) = **to_be_enlarged {
                    if let Some(forecast) = forecasts.get(idx_a).unwrap() {
                        if let Some(Some(mask)) = forecast.masks.get(idx_b) {
                            let mut r2 = vec![];
                            precalc_for_characters.query(
                                &RectMask {
                                    w: mask.w,
                                    h: mask.h,
                                    x: mask.x + forecast.x,
                                    y: mask.y + forecast.y,
                                },
                                &mut r2,
                            );
                            r2.sort_by_key(|x| {
                                (
                                    x.0.0.clone(),
                                    OrderedFloat::from(x.1.y),
                                    OrderedFloat::from(x.1.x),
                                )
                            });
                            r2.dedup(); // very important!!!!!!
                            let joined = r2
                                .iter()
                                .map(|x| x.0.1.clone())
                                .collect::<Vec<_>>()
                                .join("");
                            do_speech(&joined, &**current_language);
                        }
                    }
                }
            }
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
                    let am = get_article_metrics(&template.meta);
                    Some(Article {
                        template: t,
                        w: Some(am.0),
                        h: None,
                        x: am.2,
                        y: am.3,
                        masks: Vec::new(),
                        character_metrics: None,
                    })
                })
                .collect::<Vec<_>>();

            forecasts.set(chosen);
        },
    );

    let to_be_enlarged_elapesed_time = use_state(|| 0.0 as f64);

    let advance_elect_article = use_callback(
        (
            done_titles.clone(),
            to_be_enlarged.clone(),
            to_be_enlarged_lock.clone(),
            to_be_enlarged_elapesed_time.clone(),
            forecasts.clone(),
            game_stage.clone(),
            current_article.clone(),
        ),
        |base_article: Article,
         (
            done_titles,
            to_be_enlarged,
            to_be_enlarged_lock,
            to_be_enlarged_elapesed_time,
            forecasts,
            game_stage,
            current_article,
        )| {
            // add new title
            let mut dt = (**done_titles).clone();
            dt.insert(base_article.template.title.clone());
            done_titles.set(dt);

            // reset states
            game_stage.set(GameStage::ArticleView);
            to_be_enlarged.set(None);
            to_be_enlarged_lock.set(false);
            to_be_enlarged_elapesed_time.set(0.0);
            forecasts.set(Vec::new());

            // set article
            current_article.set(Some(base_article.clone()));
        },
    );

    // ticking funciton. Most of the "timeout" funcitons and the watching funcitons for GameStage
    // should be put here.
    let clock_callback = use_callback(
        (
            transition_history.clone(),
            game_stage.clone(),
            advance_show_forecasts.clone(),
            to_be_enlarged.clone(),
            to_be_enlarged_elapesed_time.clone(),
            to_be_enlarged_lock.clone(),
            forecasts.clone(),
            advance_elect_article.clone(),
            track_iteration_1.clone(),
            track_src_1.clone(),
        ),
        |(delta, culmative),
         (
            transition_history,
            game_stage,
            advance_show_forecasts,
            to_be_enlarged,
            to_be_enlarged_elapesed_time,
            to_be_enlarged_lock,
            forecasts,
            advance_elect_article,
            track_iteration_1,
            track_src_1,
        )| {
            let last_gs = transition_history.last().unwrap().1;

            // immediately after GameStage changes
            if **game_stage != last_gs {
                if last_gs == GameStage::ArticleView {
                    track_iteration_1.set(**track_iteration_1 + 1);
                    track_src_1.set("assets/next_v.wav".to_string());
                }
                let mut new_th = (**transition_history).clone();
                new_th.push((culmative, **game_stage));
                transition_history.set(new_th);
                return;
            }

            // timeout functions
            if **game_stage == GameStage::ArticleFading
                && culmative - transition_history.last().unwrap().0 > 10.0
            {
                advance_show_forecasts.emit(());
                game_stage.set(GameStage::ForecastStart);
                return;
            }

            if GameStage::ForecastStart <= **game_stage && **game_stage <= GameStage::Cleanup {
                if to_be_enlarged.is_some() {
                    to_be_enlarged_elapesed_time.set(**to_be_enlarged_elapesed_time + delta);
                    if **game_stage == GameStage::ForecastStart {
                        let sizemod = get_sizemod_from_time(**to_be_enlarged_elapesed_time);

                        if sizemod > LOCK_SIZE {
                            to_be_enlarged_lock.set(true);
                            track_iteration_1.set(**track_iteration_1 + 1);
                            track_src_1.set("assets/after_election_v.wav".to_string());
                            game_stage.set(GameStage::Cleanup);
                        }
                    }
                }
            }

            if **game_stage == GameStage::Cleanup
                && culmative - transition_history.last().unwrap().0 > 16.0
                && to_be_enlarged.is_some()
            {
                let a_o = forecasts.get(to_be_enlarged.unwrap().0).unwrap();
                if let Some(a) = a_o {
                    advance_elect_article.emit(a.clone());
                }
            }
        },
    );

    // obtain the article text data
    let data_table: Rc<Vec<Option<(bool, HashMap<String, String>, usize)>>> = use_memo(
        (
            current_article.clone(),
            forecasts.clone(),
            current_language.clone(),
        ),
        |(ca, fc, current_language)| {
            let mut arr: Vec<Option<(bool, HashMap<String, String>, usize)>> = Vec::new();

            let insert_data = |is_current_article,
                               target: Option<&Article>,
                               arr: &mut Vec<Option<(bool, HashMap<String, String>, usize)>>,
                               idx: usize| {
                let mut dt = make_data_table(target.unwrap().template.clone().render().unwrap());
                dt = apply_language_to_data_table(
                    &dt,
                    &**current_language,
                    &LANGUAGE_OVERWRITE_KEYS,
                );

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
        },
    );

    let ending: UseStateHandle<Option<String>> = use_state(|| None);
    // transit ArticleView -> ArticleFading instantly
    let click_evt_fade_article: Callback<MouseEvent> = use_callback(
        (game_stage.clone(), data_table.clone(), ending.clone()),
        |_, (game_stage, data_table, ending)| {
            let c = data_table
                .iter()
                .filter(|x| x.is_some() && x.as_ref().unwrap().0)
                .collect::<Vec<_>>();
            if let Some(Some(c)) = c.get(0) {
                if let Some(e) = c.1.get("ending") {
                    ending.set(Some(e.clone()));
                    return;
                }
            }
            if **game_stage == GameStage::ArticleView {
                game_stage.set(GameStage::ArticleFading);
            }
        },
    );

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
    let upgrade_plan_whole: UseStateHandle<Vec<(bool, usize, (f64, f64))>> =
        use_state(|| Vec::new());

    let upgrade_plan_characters: UseStateHandle<Vec<(bool, usize, Vec<CharacterMetric>)>> =
        use_state(|| Vec::new());

    // invokes getBoundingClientRect for each elements with unfetched metrics. Also, if there are
    // any inconsistencies between the component state and the actual rendered DOM, then query
    // re-fetching through some Timeout.
    let upgrade_plan_check = use_callback::<bool, _, _, _>(
        (
            current_article.clone(),
            forecasts.clone(),
            counter_keygen.clone(),
            upgrade_plan_whole.clone(),
            render_number.clone(),
            upgrade_plan_characters.clone(),
        ),
        move |_,
              (
            current_article,
            forecasts,
            counter,
            upgrade_plan_whole,
            render_number,
            upgrade_plan_characters,
        )| {
            // 1. common setup
            let mut all_articles = vec![(current_article.as_ref(), 0, true)];
            all_articles.append(
                &mut forecasts
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (x.as_ref(), i, false))
                    .collect(),
            );

            let mut something_wrong = false;

            // 2. WHOLE metrics

            let no_need_upgrade_whole = all_articles.iter().all(|(a, _, _)| {
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

            let upgrade_plan_whole_new = if !no_need_upgrade_whole {
                all_articles
                    .iter()
                    .map(|(a, i, is_current)| {
                        if a.is_some() {
                            let elem_id = gen_article_id(**counter, *i, *is_current);
                            let rect = get_bounding_from_id(&elem_id);
                            if let Some(rect) = rect {
                                Some((*is_current, *i, (rect.width, rect.height)))
                            } else {
                                something_wrong = true;
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

            // 3. Character metrics

            let no_need_upgrade_char = all_articles.iter().all(|(a, _, _)| {
                if let Some(a) = a {
                    if a.character_metrics.is_some() {
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            });

            let upgrade_plan_char_new = if !no_need_upgrade_char {
                all_articles
                    .iter()
                    .map(|(a, i, is_current)| {
                        if a.is_some() && a.unwrap().w.is_some() && a.unwrap().h.is_some() {
                            let elem_id = gen_article_id(**counter, *i, *is_current);
                            if let Ok(data) = get_span_metrics(&elem_id) {
                                Some((*is_current, *i, data))
                            } else {
                                something_wrong = true;
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

            // 4. postprocess

            if something_wrong {
                render_number.set(**render_number + 1);
            }

            upgrade_plan_whole.set(
                upgrade_plan_whole_new
                    .into_iter()
                    .filter_map(|x| x)
                    .collect(),
            );

            upgrade_plan_characters.set(
                upgrade_plan_char_new
                    .into_iter()
                    .filter_map(|x| x)
                    .collect(),
            );
        },
    );

    // Apply fetched upgrade plan to the component state.
    {
        let current_article = current_article.clone();
        let forecasts = forecasts.clone();
        let upgrade_plan_whole = upgrade_plan_whole.clone();
        let upgrade_plan_char = upgrade_plan_characters.clone();
        use_effect(move || {
            if upgrade_plan_whole.is_empty() && upgrade_plan_char.is_empty() {
                return;
            }
            let mut forecasts_new = (&*forecasts).clone();
            upgrade_plan_whole
                .iter()
                .for_each(|(is_current, idx, (w, h))| {
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
            upgrade_plan_char
                .iter()
                .for_each(|(is_current, idx, data)| {
                    if *is_current {
                        if let Some(a) = current_article.as_ref() {
                            let mut a = a.clone();
                            a.character_metrics = Some(data.clone());
                            current_article.set(Some(a));
                        }
                    } else {
                        if let Some(Some(a)) = forecasts.get(*idx) {
                            let mut a = a.clone();
                            a.character_metrics = Some(data.clone());
                            forecasts_new[*idx] = Some(a);
                        }
                    }
                });
            forecasts.set(forecasts_new);
            upgrade_plan_whole.set(Vec::new());
            upgrade_plan_char.set(Vec::new());
        });
    }

    // get style attr for each article.
    let gen_article_style =
        |is_current_article: bool, _, article_ref: Option<&Article>, idx: usize| {
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
    let gen_article_class = |is_current_article, _, _: Option<&Article>, idx: usize| {
        let mut classes = Vec::new();
        classes.push("article".to_string());
        if is_current_article {
            classes.push("current-article".to_string());
        } else {
            classes.push("forecast".to_string());
            if *game_stage == GameStage::Cleanup {
                if to_be_enlarged.is_some() && idx == to_be_enlarged.unwrap().0 {
                    classes.push("chosen".to_string());
                } else {
                    classes.push("fading-cleanup".to_string());
                }
            }
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
        classes.push("link-deactivated".to_string());
        return classes.join(" ");
    };

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
            for (i, r) in article_ref.unwrap().masks.clone().into_iter().filter_map(|x|x).enumerate().map(|(i, r)| {
                if *to_be_enlarged == Some((*idx, i)) {
                    let sizemod = get_sizemod_from_time(*to_be_enlarged_elapesed_time);
                    (i, apply_sizemod(&r, sizemod))
                } else {
                    (i, r)
                }
            }) {
                <rect key={i} x={format!("{:.4}", r.x)} y={format!("{:.4}", r.y)} width={format!("{:.4}", r.w)} height={format!("{:.4}", r.h)} fill="white" />
            }
        </mask>
    </svg>
    <div class={gen_article_class(*is_current, data, article_ref, *idx)} id={elem_id.clone()} key={elem_id.clone()} style={gen_article_style(*is_current, data, article_ref, *idx)}>
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
        <div class="images">
        {
            md(data.get("images").unwrap().clone())
        }
        </div>
        }
        if data.get("image_caption_work_title").is_some() {
        <div class="work-title">
        {
            md(data.get("image_caption_work_title").unwrap().clone())
        }
        </div>
        }
        if data.get("image_caption").is_some() {
        <div class="image-caption">
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
    </div>
</>
            }
        })
        .collect();

    let get_btn_style = use_callback((current_article.clone()), move |(), ca| {
        if ca.is_some() && ca.as_ref().unwrap().w.is_some() && ca.as_ref().unwrap().h.is_some() {
            let mut s = "".to_string();
            s += "position: absolute;";
            s += &format!("left: {}px;", ca.as_ref().unwrap().x);
            s += &format!(
                "top: {}px;",
                ca.as_ref().unwrap().y + ca.as_ref().unwrap().h.unwrap()
            );
            return s;
        }
        return "".to_string();
    });

    let tips_scroll_shown = use_memo(
        (
            current_article.clone(),
            transition_history.clone(),
            game_stage.clone(),
        ),
        |(ca, th, gs)| {
            if let Some(ca) = &**ca {
                if let Some(h) = ca.h {
                    if **gs == GameStage::ArticleView
                        && ca.y + h + BTN_HEIGHT > ca.template.meta.window_height
                    {
                        return true;
                    }
                }
                return false;
            } else {
                return false;
            }
        },
    );

    let tips_words_shown = use_memo(
        (transition_history.clone(), game_stage.clone()),
        |(th, gs)| {
            if **gs == GameStage::ForecastStart {
                let fs_cnt = th
                    .iter()
                    .filter(|(_, g)| *g == GameStage::ForecastStart)
                    .count();
                if fs_cnt <= 3 {
                    return true;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        },
    );

    let get_btn_class = use_callback((game_stage.clone()), move |(), gs| {
        if **gs == GameStage::ArticleFading {
            return "btn-fading btn-wrapper".to_string();
        }
        return "btn-wrapper".to_string();
    });

    // the final virtual dom
    html! {
        <>
        if *game_stage >= GameStage::ForecastStart {
        <div id="mouse-evt-overlay" onmousemove={mouse_move_evt}></div>
        }
        <div class="app-wrapper">
            {html_articles}

            if *game_stage <= GameStage::ArticleFading {
            <div class={get_btn_class.emit(())} style={get_btn_style.emit(())}>
                if ending.is_none() {
                <button onclick={click_evt_fade_article.clone()}>{get_system_word(&*current_language, "keep_reading_button")}</button>
                } else {
                { md(ending.as_ref().unwrap().clone()) }
                <button onclick={|_|{window().unwrap().location().set_href("select.html");}}>{get_system_word(&*current_language, "start_again_button")}</button>
                }
            </div>
            }

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
        <a href="select.html" class="lang-select">
            {"Back to Title"}<br/>
            {"タイトルに戻る"}
        </a>
        if *tips_scroll_shown {
            <div class="tips-scroll-wrap">
                <p class="tips-scroll">{ get_system_word(&*current_language, "tips_scroll") }</p>
            </div>
        }
        if *tips_words_shown {
            <p class="tips-words">{ get_system_word(&*current_language, "tips_words") }</p>
        }
        <AudioPlayer src={(*track_src_1).clone()} iteration={*track_iteration_1}/>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
