// ふむふむこういうのもあるのね

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gloo_timers::callback::Interval;
use rust_embed::RustEmbed;
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use web_sys::{HtmlElement, MessageEvent, WebSocket, console, js_sys::Function, window};
use yew::prelude::*;

use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Incoming {
    #[serde(rename = "initialize")]
    Initialize { client_id: String },

    #[serde(rename = "scroll_y")]
    ScrollY { client_id: String, value: f64 },
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

fn get_inquire_html(metrics: &HashMap<String, (u32, u32)>, src: &str) -> Html {
    let (w, h) = metrics.get(src).unwrap_or(&(1, 1));
    let ratio = (*w as f64) / (*h as f64);
    html! {
        <img src={ "assets/inquire/".to_string() + src } style={ "aspect-ratio: ".to_string() + &format!("{:.10}", ratio) } />
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
        let scroll_listener: Rc<RefCell<Option<Box<dyn FnMut((String, f64)) -> ()>>>> =
            Rc::new(RefCell::new(None));
        let socket = Rc::new(WebSocket::new(&"ws://localhost:6543/ws").unwrap());

        let client_id_rc = Rc::new(RefCell::new(None));

        let onmessage = {
            let scroll_listener = scroll_listener.clone();
            let client_id_rc = client_id_rc.clone();
            Closure::new(Box::new(move |m: MessageEvent| {
                if let Some(data) = m.data().as_string() {
                    match serde_json::from_str::<Incoming>(&data) {
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
                        channel: "channel12345".to_string(),
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
        Program::new(0.0, 2100.0, Slideshow::Image { src: "uchu.webp" }, Curve::Daikei),
        Program::new(6000.0, 7000.0, Slideshow::Markdown { text: "# 「内容証明アート」宣言

少しでも違和感を感じることがあったらそれを文書にしたためて関係各所に送付しまくるのだ！

新聞の投書でも、政治家の事務所へのFAXでも、弁護士会への人権侵害申立でも、国連の人権委員会への報告書(Calls for Input)でも、、

サブチャンネル、迂回路を使いまくる！！

いま最も美しいアートの形態は、将来の国際人権裁判に提出される証拠集になるであろう。

## 「内容証明アート」の規格定義 (ISO-0001)

- 蓄積すること
    
- 公の文法を撹乱すること
    

真面目な訴えかけの間に、絵文字や叫び声、ゆるふわな言葉たちを散りばめよう。決して、ふざけているのではない。そもそも、真面目であるとかふざけているとか、そうした判断基準は私たちを黙らせるための道具でしかないのだから。

- マクロとミクロを接続すること

できるだけ、日々のちょっとした違和感、日常のなかの憤りの全てを拾い上げていく。最も洗練された監査請求は、小咄、独白、アネクドート、落語、の形式で行われる。

- 自己検閲に抗うこと
    
- 百億の名前で行うこと
    

私たちを規定して、縛り付ける名前なんて、もうおさらば。色々な名前を作り続け、色々な名前で署名しよう。

わたしの　なまえ　一覧

〇〇　〇〇  
〇〇　〇〇  
〇〇　〇〇〇

などほか2穣4748𥝱9623垓385京3181兆1921億5412万189こ

あなたの　なまえ　一覧

色紙　うごぬ  
ウタタネ　ゼネスト

などほか2穣4748𥝱9623垓385京3181兆1921億5412万190こ

\\* ISO = IKITEIKOU STANDARD OPERATIONS！

## 付録1: 送付する請願書の例"}, Curve::Daikei)
    ];
}

// 47678
#[component]
fn DesktopApp() -> Html {
    let inquire_metrics = use_ref(|| get_metrics("inquire"));
    let ws = use_ref(|| {
        let ws = RefCell::new(WsClient::new(ClientMode::Screen));
        ws.borrow_mut().listen_scroll(|(id, scroll_y)| {
        });
        ws
    });
    let enter_fullscreen = |e: MouseEvent| {
        window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .request_fullscreen();
    };
    use_effect_with((), |()| {});
    html! {
        <div class="root desktop" onclick={ enter_fullscreen }>
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
