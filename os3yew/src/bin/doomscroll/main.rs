// ふむふむこういうのもあるのね

use std::collections::HashMap;

use rust_embed::RustEmbed;
use yew::prelude::*;

#[derive(RustEmbed)]
#[folder = "metadata/doomscroll"]
pub struct Asset;

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

#[component]
fn PhoneApp() -> Html {
    let komagire_metrics = get_metrics("komagire");
    html! {
        <div class="root">
            { get_komagire_html(&komagire_metrics, "tushin.webp") }
            { get_komagire_html(&komagire_metrics, "tushin.webp") }
            { get_komagire_html(&komagire_metrics, "tushin.webp") }
            { get_komagire_html(&komagire_metrics, "tushin.webp") }
            { get_komagire_html(&komagire_metrics, "tushin.webp") }
            { get_komagire_html(&komagire_metrics, "tushin.webp") }
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
