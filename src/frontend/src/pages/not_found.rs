//! The 404 page, in two wrappers around one body.
//!
//! nginx answers an unknown path with `error_page 404 /404.html`, and that file
//! comes from the prerendered `/404` route. The split is forced by the static
//! generator: `leptos_axum` deliberately refuses to write a static route whose
//! render set a 404 status, so that a site cannot end up with a cached file per
//! typo. A page that announced itself as a 404 would therefore never reach
//! disk.
//!
//! So [`NotFoundPage`] renders the body and says nothing about the status —
//! nginx supplies that — while [`NotFound`], the router's fallback, still sets
//! it, because there the response is being built rather than written to a file.

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

/// The router's fallback: a path the app itself does not know. Sets the status,
/// since this is a live response.
#[component]
pub fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        use leptos_axum::ResponseOptions;
        let resp = expect_context::<ResponseOptions>();
        resp.set_status(axum::http::StatusCode::NOT_FOUND);
    }

    view! { <NotFoundBody/> }
}

/// The `/404` route, prerendered to `404.html`. Sets no status — see the module
/// comment for why doing so would stop the file being written at all.
#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! { <NotFoundBody/> }
}
