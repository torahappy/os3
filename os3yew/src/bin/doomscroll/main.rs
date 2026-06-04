// ふむふむこういうのもあるのね

use yew::prelude::*;

#[component]
fn App() -> Html {
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
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
