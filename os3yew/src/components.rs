use gloo_timers::callback::Timeout;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{HtmlAudioElement, HtmlVideoElement, console};
use yew::prelude::*;
use yew::{Html, prelude::*};

use crate::components::_VideoWrapperProps::current_seek;

#[derive(PartialEq, Properties)]
pub struct RenderWatchProps {
    pub callback: Callback<bool, ()>,
    pub render_number: u64,
    pub children: Html,
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
    pub callback: Callback<(f64, f64), ()>,
    pub interval: u32,
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

/// The props that the component receives.
#[derive(Properties, PartialEq)]
pub struct AudioPlayerProps {
    /// URL of the audio file to play.
    pub src: Option<String>,

    /// A number that changes whenever we want to trigger a new play.
    pub iteration: u32,
}

/// The component that plays the audio automatically.
#[component]
pub fn AudioPlayer(props: &AudioPlayerProps) -> Html {
    let audio_ref = use_node_ref();
    let src = props.src.clone();
    let iteration = props.iteration;

    {
        let audio_ref = audio_ref.clone();
        use_effect_with((src.clone(), iteration), {
            move |(src, _iteration)| {
                if src.is_some() {
                    let audio_element: HtmlAudioElement = audio_ref
                        .cast::<HtmlAudioElement>()
                        .expect("audio element not found");

                    audio_element.set_src(&src.as_ref().unwrap());
                    if audio_element.ended() {
                        audio_element.set_current_time(0.0);
                        console::log_1(&"Play ended; Back to 0.0".into());
                    };

                    match audio_element.play() {
                        Ok(_) => {}
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!(
                                    "Error while calling play(): {}",
                                    e.as_string().unwrap_or_default()
                                )
                                .into(),
                            );
                        }
                    }
                }
            }
        });
    }

    html! {
        <audio ref={audio_ref} style="display:none" />
    }
}

#[derive(Properties, PartialEq)]
pub struct VideoWrapperProps {
    pub src: String,
    pub current_seek: f64,
}

/// Create a video wrapper which is immune to the seeking back to the beginning
/// effect on re-rendering.
#[function_component(VideoWrapper)]
pub fn video_wrapper(props: &VideoWrapperProps) -> Html {
    let video_ref = use_node_ref();

    {
        let video_ref = video_ref.clone();
        let current_seek_copy = props.current_seek;
        use_effect(move || {
            if let Some(v) = video_ref.cast::<HtmlVideoElement>() {
                if (v.current_time() - current_seek_copy).abs() > 0.5 {
                    v.set_current_time(current_seek_copy);
                }
            };
        });
    }

    html! {
        <video ref={video_ref} src={ props.src.clone() } autoplay={true} muted={true} playsinline={true} loop={true}></video>
    }
}
