use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::static_routes::StaticRoute;
use leptos_router::SsrMode;
use leptos_router_macro::path;

use crate::pages::{landing::Landing, not_found::NotFound, not_found::NotFoundPage};

/// Cache-buster for `/landing.js`.
///
/// Everything under `pkg/` carries a content hash in its filename, which is
/// what lets the CDN keep it forever. `landing.js` does not: it is copied
/// verbatim out of `assets-dir`, under the name it has on disk. Hashing its
/// bytes at compile time gives the URL something that changes exactly when the
/// file does, so the same `immutable` lifetime is safe for it too.
#[cfg(feature = "ssr")]
const LANDING_JS_VERSION: u64 = fnv1a(include_bytes!("../public/landing.js"));

/// FNV-1a, because it is four lines of `const fn` and nothing here needs a
/// cryptographic hash — only a value that differs when the input differs.
#[cfg(feature = "ssr")]
const fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    hash
}

/// The outer HTML shell — only used server-side for the initial SSR response.
#[cfg(feature = "ssr")]
pub fn shell(options: leptos::config::LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options.clone() islands=false />
                <MetaTags/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                // Not a plain <link>: with cargo-leptos `hash-files` on, the
                // stylesheet is `website.<hash>.css` and only this component
                // knows the hash. The same hash file is what `HydrationScripts`
                // above reads for the JS and the WASM.
                <HashedStylesheet options=options.clone() id="leptos"/>
                <script src="https://cdnjs.cloudflare.com/ajax/libs/gsap/3.12.5/gsap.min.js" defer></script>
                <script src="https://cdnjs.cloudflare.com/ajax/libs/gsap/3.12.5/ScrollTrigger.min.js" defer></script>
                <script src=format!("/landing.js?v={LANDING_JS_VERSION:016x}") defer></script>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Root application component — shared between SSR and WASM hydration.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="Save the Date · Cameron & Ryan · May 15, 2027"/>
        <Meta name="description" content="Save the date for Cameron and Ryan's wedding in Bay Head, New Jersey."/>

        <Router>
            <main>
                <Routes fallback=NotFound>
                    <Route path=path!("/") view=Landing ssr=SsrMode::Static(StaticRoute::new())/>
                    // Prerendered so there is a `404.html` on disk for nginx's
                    // `error_page` to return. Nothing links to `/404`; the
                    // route exists to produce the file. `NotFoundPage` rather
                    // than `NotFound` because the generator throws away any
                    // static route that renders a 404 status, which is the
                    // behaviour that keeps a typo from being cached — see
                    // `pages::not_found`.
                    <Route path=path!("/404") view=NotFoundPage ssr=SsrMode::Static(StaticRoute::new())/>
                </Routes>
            </main>
        </Router>
    }
}
