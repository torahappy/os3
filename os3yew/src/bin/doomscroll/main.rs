// ふむふむこういうのもあるのね

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gloo_timers::callback::Interval;
use rust_embed::RustEmbed;
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use web_sys::{HtmlElement, MessageEvent, WebSocket, console, js_sys::Function, window};
use yew::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Outgoing {
    #[serde(rename = "keepalive")]
    KeepAlive,
    #[serde(rename = "initialize_response")]
    InitializeResponse {
        channel: String,
        client_type: String, // "phone"
    },
    #[serde(rename = "scroll_y")]
    ScrollY { value: i32 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Incoming {
    #[serde(rename = "initialize")]
    Initialize { client_id: String },
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
}

impl WsClient {
    fn new() -> WsClient {
        let socket = Rc::new(WebSocket::new(&"ws://localhost:6543/ws").unwrap());

        let client_id_rc = Rc::new(RefCell::new(None));

        let onmessage = {
            let client_id_rc = client_id_rc.clone();
            Closure::new(Box::new(move |m: MessageEvent| {
                if let Some(data) = m.data().as_string() {
                    match serde_json::from_str::<Incoming>(&data) {
                        Ok(Incoming::Initialize { client_id }) => {
                            *client_id_rc.borrow_mut() = Some(client_id);
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
                        client_type: "phone".to_string(),
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
        };
    }
    fn scroll(&self, value: i32) {
        if let Err(e) = self
            .socket
            .send_with_str(&serde_json::to_string(&Outgoing::ScrollY { value: value }).unwrap())
        {
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

    let ws = use_ref(|| WsClient::new());

    let komagire_metrics = get_metrics("komagire");

    let scroll_handle = {
        let ws = ws.clone();
        move |e: html::onscroll::Event| {
            let elem: JsValue = e.target().unwrap().into();
            let elem: HtmlElement = elem.into();
            ws.scroll(elem.scroll_top());
        }
    };

    html! {
        <div class="root" onscroll={ scroll_handle }>
        <div class="stack">
            { (1..10).map(|_| get_komagire_html(&komagire_metrics, "tushin.webp")).collect::<Vec<_>>() }
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

#[component]
fn DesktopApp() -> Html {
    html! {
        <div>
        </div>
    }
}

fn main() {
    yew::Renderer::<PhoneApp>::new().render();
}
