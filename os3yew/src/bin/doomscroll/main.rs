// ふむふむこういうのもあるのね

use std::{cell::RefCell, collections::{HashMap, HashSet}, hash::{DefaultHasher, Hash, Hasher}, rc::Rc, time::Instant};

use gloo_timers::callback::Interval;
use os3yew::{components::{ClockComponent, VideoWrapper}, util::md};
use rust_embed::RustEmbed;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{HtmlElement, Location, MessageEvent, WebSocket, Window, console, js_sys::{Function, Reflect}, window};
use yew::prelude::*;

use serde::{Deserialize, Serialize};

use crate::Curve::Daikei;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
enum ClientMode {
    #[serde(rename = "screen")]
    Screen,
    #[serde(rename = "phone")]
    Phone,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Outgoing {
    #[serde(rename = "keepalive")]
    KeepAlive,
    #[serde(rename = "initialize_response")]
    InitializeResponse {
        channel: String,
        client_type: ClientMode,
    },
    #[serde(rename = "scroll_y")]
    ScrollY { value: f64 },
    #[serde(rename = "list_clients")]
    ListClients,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Incoming {
    #[serde(rename = "initialize")]
    Initialize { client_id: String },

    #[serde(rename = "scroll_y")]
    ScrollY { client_id: String, value: f64 },
    #[serde(rename = "list_clients")]
    ListClients { clients: Vec<String> },
}

#[derive(RustEmbed)]
#[folder = "metadata/doomscroll"]
pub struct Asset;

fn scroll_to_top() {
    window().unwrap().scroll_to_with_x_and_y(0.0, 0.0);
}

fn get_metrics(category: &str) -> HashMap<String, (u32, u32)> {
    let blob = Asset::get(&(category.to_string() + &".json")).unwrap().data;
    let parsed: HashMap<String, (u32, u32)> = serde_json::from_slice(blob.as_ref()).unwrap();
    return parsed;
}

fn get_komagire_html(metrics: &HashMap<String, (u32, u32)>, src: &str) -> Html {
    let (w, h) = metrics.get(src).unwrap_or(&(1, 1));
    let ratio = (*w as f64) / (*h as f64);
    html! {
        <img src={ "assets/komagire/".to_string() + src } style={ "aspect-ratio: ".to_string() + &format!("{:.10}", ratio) } />
    }
}

fn get_video_html(src: &str) -> Html {
    html! {
        <video src={ "assets/inquire/".to_string() + src } autoplay={true} muted={true} playsinline={true} loop={true}></video>
    }
}

fn get_inquire_html(metrics: &HashMap<String, (u32, u32)>, src: &str) -> Html {
    let (w, h) = metrics.get(src).unwrap_or(&(1, 1));
    let ratio = (*w as f64) / (*h as f64);
    html! {
        <div class="inquire-image" style={ format!("aspect-ratio: {:.10}; background-image: url('assets/inquire/{}');", ratio, src) } />
    }
}

fn komagire_three(metrics: &HashMap<String, (u32, u32)>, images: (&str, &str, &str)) -> Html {
    html! {
        <div class="komagire-wrap">
            { get_komagire_html(metrics, &images.0) }
            { get_komagire_html(metrics, &images.1) }
            { get_komagire_html(metrics, &images.2) }
        </div>
    }
}

struct WsClient {
    socket: Rc<WebSocket>,
    onmessage: Closure<dyn FnMut(MessageEvent) -> ()>,
    onopen: Closure<dyn FnMut(JsValue) -> ()>,
    client_id: Rc<RefCell<Option<String>>>,
    interval: Interval,
    mode: ClientMode,
    scroll_listener: Rc<RefCell<Option<Box<dyn FnMut((String, f64)) -> ()>>>>,
    clients: Rc<RefCell<Vec<String>>>,
}

impl WsClient {
    fn listen_scroll<F>(&mut self, f: F)
    where
        F: FnMut((String, f64)) -> () + 'static,
    {
        if self.mode != ClientMode::Screen {
            return;
        }
        *self.scroll_listener.borrow_mut() = Some(Box::new(f));
    }
    fn new(mode: ClientMode) -> WsClient {
        let clients_rc = Rc::new(RefCell::new(Vec::new()));
        let scroll_listener: Rc<RefCell<Option<Box<dyn FnMut((String, f64)) -> ()>>>> =
            Rc::new(RefCell::new(None));
        let window: Window = web_sys::window().unwrap();
        let location: Location = window.location();
        let search = location.search().unwrap_or_default();
        let usp = web_sys::UrlSearchParams::new_with_str(&search).unwrap();
        let has_local = usp.get("local").is_some();
        let channel = usp.get("channel").unwrap_or("channel12345".to_string());

        let socket = if has_local {
            Rc::new(WebSocket::new(&"/doomscroll_web/ws").unwrap())
        } else {
            Rc::new(WebSocket::new(&"wss://os3.torahappy.org/doomscroll_web/ws").unwrap())
        };

        let client_id_rc = Rc::new(RefCell::new(None));

        let onmessage = {
            let scroll_listener = scroll_listener.clone();
            let client_id_rc = client_id_rc.clone();
            let clients_rc = clients_rc.clone();
            Closure::new(Box::new(move |m: MessageEvent| {
                if let Some(data) = m.data().as_string() {
                    match serde_json::from_str::<Incoming>(&data) {
                        Ok(Incoming::ListClients { clients }) => {
                            *clients_rc.borrow_mut() = clients;
                        }
                        Ok(Incoming::Initialize { client_id }) => {
                            *client_id_rc.borrow_mut() = Some(client_id);
                        }
                        Ok(Incoming::ScrollY { client_id, value }) => {
                            if mode == ClientMode::Screen {
                                if scroll_listener.borrow().is_some() {
                                    if let Some(x) = scroll_listener.borrow_mut().as_mut() {
                                        x((client_id, value));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            console::error_1(&format!("Message Parse Error: {:?}", e).into());
                        }
                    }
                }
            }))
        };
        socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        let onopen = {
            let socket = socket.clone();
            Closure::new(Box::new(move |_| {
                if let Err(e) = socket.send_with_str(
                    &serde_json::to_string(&Outgoing::InitializeResponse {
                        channel: channel.clone(),
                        client_type: mode,
                    })
                    .unwrap(),
                ) {
                    console::error_2(&"Message Send Error (Init): ".into(), &e);
                }
            }))
        };
        socket.set_onopen(Some(onopen.as_ref().unchecked_ref()));

        let interval = {
            let socket = socket.clone();
            Interval::new(1000, move || {
                if let Err(e) =
                    socket.send_with_str(&serde_json::to_string(&Outgoing::KeepAlive {}).unwrap())
                {
                    console::error_2(&"Message Send Error (KeepAlive): ".into(), &e);
                }
            })
        };

        return WsClient {
            socket,
            onmessage,
            onopen,
            interval,
            client_id: client_id_rc,
            mode,
            scroll_listener,
            clients: clients_rc,
        };
    }
    fn scroll(&self, value: f64, scroll_width: f64) {
        let coeff = if scroll_width >= 768.0 {
            1.0
        } else {
            768.0 / scroll_width
        };
        console::log_1(&format!("{}", coeff).into());
        if self.mode != ClientMode::Phone {
            return;
        }
        if let Err(e) = self.socket.send_with_str(
            &serde_json::to_string(&Outgoing::ScrollY {
                value: value * coeff,
            })
            .unwrap(),
        ) {
            console::error_2(&"Message Send Error (Scroll): ".into(), &e);
        }
    }
    fn query_list_clients(&self) {
        if self.mode != ClientMode::Screen {
            return;
        }
        if let Err(e) = self
            .socket
            .send_with_str(&serde_json::to_string(&Outgoing::ListClients).unwrap())
        {
            console::error_2(&"Message Send Error (ListClients): ".into(), &e);
        };
    }
}

impl Drop for WsClient {
    fn drop(&mut self) {
        if let Err(e) = self.socket.close() {
            console::error_2(&"Socket Close Error: ".into(), &e);
        }
    }
}

#[component]
fn PhoneApp() -> Html {
    use_effect_with((), |()| {
        scroll_to_top();
    });

    let ws = use_mut_ref(|| WsClient::new(ClientMode::Phone));

    let komagire_metrics = use_ref(|| get_metrics("komagire"));

    let scroll_handle = {
        let ws = ws.clone();
        move |e: html::onscroll::Event| {
            let elem: JsValue = e.target().unwrap().into();
            let elem: HtmlElement = elem.into();
            let client_height = elem.client_height() as f64;
            let client_width = elem.client_width() as f64;
            let scroll_top = elem.scroll_top() as f64;
            ws.borrow()
                .scroll(client_height + scroll_top as f64, client_width as f64);
        }
    };


    html! {
        <div class="root" onscroll={ scroll_handle }>
        <div class="stack">
            { (1..16).map(|_| get_komagire_html(&komagire_metrics, "tushin.webp")).collect::<Vec<_>>() }
            { komagire_three(&komagire_metrics, ("bt1-1.webp", "bt1-2.webp", "bt1-3.webp")) }
            { get_komagire_html(&komagire_metrics, "tushin.webp") }
            { komagire_three(&komagire_metrics, ("bt2-1.webp", "bt2-2.webp", "bt2-3.webp")) }
            { get_komagire_html(&komagire_metrics, &"tushin.webp") }
            { komagire_three(&komagire_metrics, ("s1.webp", "s2.webp", "s3.webp")) }
            { get_komagire_html(&komagire_metrics, &"tushin.webp") }
            { komagire_three(&komagire_metrics, ("ni1.webp", "nkr11-1.webp", "nkr11-2.webp")) }
            { get_komagire_html(&komagire_metrics, &"tushin.webp") }
            { komagire_three(&komagire_metrics, ("nkr11-3.webp", "nkr14-2.webp", "nkr16.webp")) }
            { (1..6).map(|_| get_komagire_html(&komagire_metrics, "tushin.webp")).collect::<Vec<_>>() }
        </div>
        </div>
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
enum Curve {
    #[default]
    Daikei,
}

#[derive(Debug, PartialEq, Clone, Default)]
enum Slideshow {
    Movie {
        src: &'static str,
    },
    Image {
        src: &'static str,
    },
    Markdown {
        text: &'static str,
    },
    #[default]
    Nothing,
}

#[derive(Debug, PartialEq, Clone, Default)]
struct Program {
    start: f64,
    end: f64,
    slideshow: Slideshow,
    curve: Curve,
}

impl Program {
    fn new(start: f64, end: f64, slideshow: Slideshow, curve: Curve) -> Program {
        return Program {
            start,
            end,
            slideshow,
            curve,
        };
    }
}

fn get_ranges_data() -> Vec<Program> {
    return vec![
        Program::new(2100.0, 7990.0, Slideshow::Image { src: "uchu.webp" }, Curve::Daikei),
        Program::new(8100.0, 11000.0, Slideshow::Markdown { text: "# 「内容証明アート」宣言

少しでも違和感を感じることがあったらそれを文書にしたためて、抗議文、陳情書、申立書を関係各所に送付しまくるのだ！

新聞の投書でも、政治家の事務所へのFAXでも、弁護士会への人権侵害申立でも、国連の人権委員会への報告書(Calls for Input)でも、、、

サブチャンネル、迂回路、社会が私たちにお情け的に用意してくれている回路を使いまくる！！

それらがすぐには何かにならなくとも、証拠として蓄積されていく。

いま最も美しいアートの形態は、将来の国際人権裁判に提出される証拠集になるであろう。

"}, Curve::Daikei),
Program::new(11111.0, 13000.0, Slideshow::Markdown { text: 
"## 「内容証明アート」の規格定義

(ISO / IKITEIKOU STANDARD OPERATIONS -0001)

★蓄積すること    

★公の文法を撹乱すること

真面目な訴えかけの間に、絵文字や叫び声、ゆるふわな言葉たち、あるいは感情的な叫びを散りばめよう。決して、ふざけているのではない。そもそも、真面目であるとかふざけているとか、そうした判断基準は私たちを黙らせるための道具でしかないのだから。

"}, Curve::Daikei),
Program::new(13000.0, 16000.0, Slideshow::Markdown { text: "
★マクロとミクロを接続すること

できるだけ、日々のちょっとした違和感、日常のなかの憤りの全てを拾い上げていく。最も洗練された監査請求は、小咄、独白、アネクドート、落語、の形式で行われる。

★自己検閲に抗うこと

★百億の名前で行うこと

私たちを規定して、縛り付ける名前なんて、もうおさらば。色々な名前を作り続け、色々な名前で署名しよう。
"}, Curve::Daikei),
Program::new(16000.0, 19000.0, Slideshow::Markdown { text: "
## わたしの　なまえ　一覧

〇〇　〇〇  
〇〇　〇〇  
〇〇　〇〇〇

などほか2穣4748𥝱9623垓385京3181兆1921億5412万189こ

## あなたの　なまえ　一覧

色紙　うごぬ  
ウタタネ　ゼネスト

などほか2穣4748𥝱9623垓385京3181兆1921億5412万190こ
"}, Curve::Daikei),
        Program::new(19000.0, 23200.0, Slideshow::Image { src: "IMG_3011.webp" }, Curve::Daikei),
        Program::new(23200.0, 26200.0, Slideshow::Image { src: "IMG_2891.webp" }, Curve::Daikei),
        Program::new(24145.0, 27800.0, Slideshow::Image { src: "IMG_2893.webp" }, Curve::Daikei),
        Program::new(28000.0, 30000.0, Slideshow::Image { src: "IMG_2956.webp" }, Curve::Daikei),
        Program::new(30000.0, 31250.0, Slideshow::Image { src: "IMG_2916.webp" }, Curve::Daikei),
        Program::new(30000.0, 33250.0, Slideshow::Image { src: "IMG_2917.webp" }, Curve::Daikei),
        Program::new(33250.0, 36250.0, Slideshow::Image { src: "IMG_2867.webp" }, Curve::Daikei),
        Program::new(36250.0, 43000.0, Slideshow::Movie { src: "KONOYOWAJIGOKU.webm" }, Curve::Daikei),
        Program::new(43000.0, 45000.0, Slideshow::Image { src: "IMG_3012.webp" }, Curve::Daikei),
        Program::new(45000.0, 46000.0, Slideshow::Image { src: "IMG_3037.webp" }, Curve::Daikei),
        Program::new(46000.0, 47000.0, Slideshow::Image { src: "IMG_3038.webp" }, Curve::Daikei),
        Program::new(47000.0, 50000.0, Slideshow::Image { src: "uchu.webp" }, Curve::Daikei),
    ];
}

// 47678
#[component]
fn DesktopApp() -> Html {
    let clients: UseStateHandle<HashMap<String, f64>> = use_state(|| HashMap::new());
    let ranges = use_ref(|| get_ranges_data());
    let inquire_metrics = use_ref(|| get_metrics("inquire"));
    let ws = use_ref(|| {
        let ws = RefCell::new(WsClient::new(ClientMode::Screen));
        ws
    });
    {
        let ws = ws.clone();
        let clients = clients.clone();
        use_effect(move || {
            ws.borrow_mut().listen_scroll(move |(id, scroll_y)| {
                let mut new_clients = (*clients).clone();
                new_clients.insert(id, scroll_y);
                clients.set(new_clients);
            });
        });
    };
    let enter_fullscreen = |e: MouseEvent| {
        window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .request_fullscreen();
    };

    let clock_callback = {
        let ws = ws.clone();
        let clients = clients.clone();
        move |(delta, culmative)| {
            ws.borrow().query_list_clients();
            let cl = ws.borrow().clients.borrow().clone();
            let mut clients_copy = (*clients).clone();

            // response from server
            let hs: HashSet<String> = HashSet::from_iter(cl.into_iter());

            // current set of client ids
            let hs2: HashSet<String> = HashSet::from_iter(clients_copy.keys().into_iter().cloned());

            // if there are changes in membership... Delete the clients that left the session.
            if hs != hs2 {
                clients_copy.retain(|k, _| hs.contains(k));
                clients.set(clients_copy);
            }
        }
    };

    let pictures = {
        clients
            .clone()
            .iter()
            .map(|(id, scr)| {
                let mut out = Vec::new();
                let segment = ranges.iter().enumerate().for_each(|(idx, pr)| {
                    let rotation = (idx as f64).sin() * 1.1;
                    if pr.start <= *scr && *scr <= pr.end {
                        let t = (scr - pr.start) / (pr.end - pr.start);
                        console::log_1(&format!("{}", t).into());
                        let tt = match pr.curve {
                            Daikei => {
                                if t < 1.0 / 3.0 {
                                    t * 3.0
                                } else if t < 2.0 / 3.0 {
                                    0.99
                                } else {
                                    ((1.0 - t) * 3.0)
                                }
                            }
                        };

                        let main_html = match pr.slideshow {
                            Slideshow::Movie { src } => {
                                get_video_html(src)
                            },
                            Slideshow::Image { src } => {
                                get_inquire_html(&inquire_metrics, src)
                            },
                            Slideshow::Markdown { text } => {
                                md(text.to_string())
                            },
                            Slideshow::Nothing => {
                                html!{}
                            },
                        };
                        let mut h =  DefaultHasher::new();
                        id.hash(&mut h);
                        out.push(html! {
                            <div key={id.clone()} class="picture-wrap" style={ format!("opacity: {}; color: white; z-index: {}; transform: rotate({:.4}deg);", tt, ((h.finish() % 10000) as f64) + tt, rotation) }>
                                <div class="picture">
                                    {main_html}
                                </div>
                            </div>
                        });
                    }
                });
                out
            })
            .flatten()
            .collect::<Vec<_>>()
    };

    let culmative:UseStateHandle<f64> = use_state(||0.0);

    let kokki_instances: UseStateHandle<Vec<f64>> = use_state(|| Vec::new());

    let keypress_handle = {
        let kokki_instances = kokki_instances.clone();
        let culmative = culmative.clone();
        Callback::from(move |e: KeyboardEvent| {
            let last_kokki = kokki_instances.last();
            if e.key() == "ArrowRight" && (last_kokki.is_none() || (*culmative - last_kokki.unwrap()) > 10.0) {
                let mut new_kokki_instances = (*kokki_instances).clone();
                new_kokki_instances.push(*culmative);
                kokki_instances.set(new_kokki_instances);

                let win = window().unwrap();
                let api_electron = Reflect::get(&win, &JsValue::from_str("api_electron"))
                    .ok();
                if let Some(api) = api_electron {
                    let do_print_val = Reflect::get(&api, &JsValue::from_str("do_print")).ok();
                    if let Some(do_print_val) = do_print_val {
                        if do_print_val.is_function() {
                            let do_print: Function = do_print_val.dyn_into::<Function>().expect("do_print is not a function");
                            let _ = do_print.call4(
                                &JsValue::NULL,
                                &JsValue::from_f64(3.0),   // first argument: 3
                                &JsValue::from_f64(0.0),   // second argument: 0
                                &JsValue::from_f64(0.0),   // third argument: 0
                                &JsValue::from_f64(0.0),   // third argument: 0
                            );
                        }
                    }
                }
            }
        })
    };

    let pictures_kokki = (*kokki_instances).clone().iter().enumerate().map(|(i, x)| {
        let x_key = x.to_string();

        html! {
                            <div data-key-debug={x_key} key={ x_key.clone() } class="picture-wrap" style={ format!("z-index: {}; transform: rotate({:.4}deg);", 50000.0 + (i as f32) / 100.0, i as f64 * 3.0 + x.fract() * 4.0) }>
                                <div class="picture">
                                    <VideoWrapper src="assets/inquire/rec20260720-crop.webm" current_seek={*culmative - x} />
                                </div>
                            </div>
        }
    }).collect::<Vec<_>>();

    use_effect_with((kokki_instances.clone(), culmative.clone()), move |(kokki_instances, culmative)| {
        let mut new_kokki_instances = (**kokki_instances).clone();

        new_kokki_instances.retain(|x| **culmative - x < 52.0);
        kokki_instances.set(new_kokki_instances);
    });

    let time_cb: Callback<((f64, f64))> = {
        let culmative = culmative.clone();
        Callback::from(move |(delta, cul)| {
            culmative.set(cul);
        })
    };

    let ref_key_listener: Rc<RefCell<Option<Closure<dyn FnMut(KeyboardEvent)>>>> = use_ref(|| RefCell::new(None));

    {
        let ref_key_listener = ref_key_listener.clone();
        let keypress_handle = keypress_handle.clone();
        use_effect(move || {
            let w: Window = window().unwrap();

            let key_listener = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
                (keypress_handle).emit(ev,);
            }) as Box<dyn FnMut(KeyboardEvent)>);
            let _ = w.add_event_listener_with_callback("keydown", key_listener.as_ref().unchecked_ref());

            let mut current_closure = ref_key_listener.borrow_mut();
            *current_closure = Some(key_listener);

            {
                let ref_key_listener =  ref_key_listener.clone();
                return move || {
                    let key_listener = ref_key_listener.borrow();
                    if let Some(key_listener) = key_listener.as_ref() {
                        let _ = w.remove_event_listener_with_callback("keydown", key_listener.as_ref().unchecked_ref());
                    }
                }
            }
        });
    };

    html! {
        <div class="root desktop">
            <ClockComponent interval={100} callback={clock_callback}/>
            { pictures }
            { pictures_kokki }
            <ClockComponent callback={time_cb} interval={16}/>
        </div>
    }
}

fn main() {
    let s = window().unwrap().location().search().unwrap();
    let usp = web_sys::UrlSearchParams::new_with_str(&s).unwrap();
    if usp.get("screen").is_some() {
        yew::Renderer::<DesktopApp>::new().render();
    } else {
        yew::Renderer::<PhoneApp>::new().render();
    }
}
