// The hydration path instantiates the same `into_any()` view type the server
// renders, so this needs the same headroom as the static site generator.
// Only the release build trips it: the extra layout work it does crosses the
// default limit that debug stays just inside of.
#![recursion_limit = "1024"]

pub mod app;
pub mod pages;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    //console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
