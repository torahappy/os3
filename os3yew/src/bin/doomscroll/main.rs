// ふむふむこういうのもあるのね

use yew::prelude::*;

// 2867 2893

#[component]
fn PhoneApp() -> Html {
    html! {
        <div class="root">
            <img src="assets/komagire/tushin.webp" />
            <img src="assets/komagire/tushin.webp" />
            <img src="assets/komagire/tushin.webp" />
            <img src="assets/komagire/tushin.webp" />
            <img src="assets/komagire/tushin.webp" />
            <img src="assets/komagire/tushin.webp" />
            <div class="komagire-wrap">
                <img src="assets/komagire/bt1-1.webp" />
                <img src="assets/komagire/bt1-2.webp" />
                <img src="assets/komagire/bt1-3.webp" />
            </div>
            <img src="assets/komagire/tushin.webp" />
            <div class="komagire-wrap">
                <img src="assets/komagire/bt2-1.webp" />
                <img src="assets/komagire/bt2-2.webp" />
                <img src="assets/komagire/bt2-3.webp" />
            </div>
            <img src="assets/komagire/tushin.webp" />
            <div class="komagire-wrap">
                <img src="assets/komagire/s1.webp" />
                <img src="assets/komagire/s2.webp" />
                <img src="assets/komagire/s3.webp" />
            </div>
            <img src="assets/komagire/tushin.webp" />
            <div class="komagire-wrap">
                <img src="assets/komagire/ni1.webp" />
                <img src="assets/komagire/nkr11-1.webp" />
                <img src="assets/komagire/nkr11-2.webp" />
            </div>
            <div class="komagire-wrap">
                <img src="assets/komagire/ni11-3.webp" />
                <img src="assets/komagire/nkr14-2.webp" />
                <img src="assets/komagire/nkr16.webp" />
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
