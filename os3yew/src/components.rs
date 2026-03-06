use gloo_timers::callback::Timeout;
use yew::{Html, prelude::*};

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
