//use leptos::ev::TouchEvent;
use leptos::web_sys::TouchEvent;

use leptos::prelude::*;
use leptos::logging::log;

#[component]
pub fn Section(children: Children, image: &'static str, active: bool) -> impl IntoView {
    let _can_transition = signal(false);

    let (touch_start, set_touch_start) = signal(0);
    let on_touch_start = move |event: TouchEvent| {
        if let Some(touch) = event.touches().get(0) {
            set_touch_start.set(touch.client_y());
        }

    };

    let on_touch_end = move |event: TouchEvent| {
        if let Some(touch) = event.changed_touches().get(0) {
            if (touch.client_y() - touch_start.get()) < -100 {
                log!("Swipe down!")
            }
        }
    };

    let hidden = match active {
        true => "",
        false => "display: none;"
    };

    view!{
        <div
            class="section"
            style={format!("background-image: url('{}'); {}", image, hidden)}
            on:touchstart={on_touch_start}
            on:touchend={on_touch_end}
            on:touchcancel={on_touch_end}
        >
            <div class="section__blur-overlay"></div>
            <div class="section__content">
                {children()}
            </div>
        </div>
    }
}