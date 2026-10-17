//! Shared 404 body for nginx's prerendered error page and the client router.

use leptos::prelude::*;
use leptos_meta::Title;

/// What both of them show.
#[component]
fn NotFoundBody() -> impl IntoView {
    view! {
        <Title text="404 Not Found"/>
        <div class="not-found">
            <h1>"404"</h1>
            <p>"Page not found."</p>
        </div>
    }
}

/// The router's fallback for an unknown client-side route.
#[component]
pub fn NotFound() -> impl IntoView {
    view! { <NotFoundBody/> }
}

/// The `/404` route, prerendered to `404.html`. nginx supplies the HTTP status.
#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! { <NotFoundBody/> }
}
