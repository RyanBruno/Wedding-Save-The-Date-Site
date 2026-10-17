use leptos::prelude::*;
use leptos_meta::Title;

/// The attire guide — a page of its own rather than a longer card on the
/// landing page, because "formal" means different things to different guests
/// and the answer wants pictures.
///
/// The mood board is deliberately empty for now: the tiles are captioned with
/// what belongs in each one, so the layout is already the real layout and
/// dropping the photographs in later is a swap rather than a redesign.
#[component]
pub fn Attire() -> impl IntoView {
    // Same GSAP boot as the landing page. `initLanding` looks up every element
    // it touches and skips whatever is absent, so the parallax and timeline
    // blocks simply find nothing here and the card reveals still run.
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;
            use wasm_bindgen::JsCast;

            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_name = initLanding)]
                fn init_landing();
            }

            let cb = Closure::once_into_js(move || {
                init_landing();
            });
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    0,
                );
            }
        }
    });

    view! {
        <Title text="Attire · Cameron & Ryan"/>

        <div class="cinematic">
            // ── Nav ──────────────────────────────────────────────────────────────
            // Solid from the first pixel: there is no hero photograph here for a
            // transparent bar to sit over.
            <nav class="cn-nav cn-nav--solid">
                <div class="wrap cn-nav__row">
                    <a class="cn-nav__crest" href="/">"Cameron & Ryan"</a>
                    <div class="cn-nav__links">
                        <a href="/">"The Invitation"</a>
                    </div>
                    <div class="cn-nav__cta">
                        <a class="cn-btn cn-btn--accent" href="/#rsvp">"RSVP"</a>
                    </div>
                </div>
            </nav>

            // ── Heading ──────────────────────────────────────────────────────────
            <section id="top" class="cn-section cn-page-head">
                <div class="wrap">
                    <a class="cn-back" href="/">
                        <span class="cn-back__arrow">"←"</span>
                        "Back To The Invitation"
                    </a>
                    <div class="cn-heading cn-heading--center cn-reveal">
                        <div class="cn-eyebrow">"May 15, 2027 · Bay Head, New Jersey"</div>
                        <h1 class="cn-h2">"What To Wear"</h1>
                        <p class="cn-lede">
                            "The dress code is formal. A church at four o'clock, then The Grenville
                             by the water until late — dressed up, and warm enough to stand outside
                             after dark."
                        </p>
                    </div>
                </div>
            </section>

            // ── The short version ────────────────────────────────────────────────
            <section class="cn-section cn-section--alt">
                <div class="wrap">
                    <div class="cn-heading cn-heading--center cn-reveal">
                        <div class="cn-eyebrow">"If You Read Nothing Else"</div>
                        <h2 class="cn-h2">"The Short Version"</h2>
                    </div>
                    <div class="cn-details-grid cn-details-grid--trio">
                        <div class="cn-card">
                            <div class="cn-eyebrow">"Suits"</div>
                            <div class="cn-card__ttl">"A Dark Suit And Tie"</div>
                            <p>"Navy, charcoal or black, with a tie. A tuxedo is welcome and is not
                                expected — roughly a third of the room will be in one."</p>
                        </div>
                        <div class="cn-card">
                            <div class="cn-eyebrow">"Dresses"</div>
                            <div class="cn-card__ttl">"Floor Length Or Cocktail"</div>
                            <p>"A gown, a midi or a dressy cocktail length all read correctly.
                                A jumpsuit does too."</p>
                        </div>
                        <div class="cn-card">
                            <div class="cn-eyebrow">"Shoes"</div>
                            <div class="cn-card__ttl">"Think About The Grass"</div>
                            <p>"Cocktail hour is on a lawn. A block heel or a flat will thank you
                                by seven o'clock."</p>
                        </div>
                    </div>
                </div>
            </section>

            // ── Mood board ───────────────────────────────────────────────────────
            <section id="mood" class="cn-section cn-section--full">
                <div class="wrap">
                    <div class="cn-heading cn-heading--center cn-reveal">
                        <div class="cn-eyebrow">"For The Avoidance Of Doubt"</div>
                        <h2 class="cn-h2">"The Mood Board"</h2>
                        <p class="cn-lede">"Images to come — the captions are what each frame will hold."</p>
                    </div>
                    <div class="cn-mood">
                        {[
                            ("I",    "Black tie optional",  "A dark suit, a tuxedo, and where the line between them sits."),
                            ("II",   "Gowns",               "Floor length, in the palette below."),
                            ("III",  "Cocktail length",     "Midi and knee length, dressed up rather than down."),
                            ("IV",   "Seaside formal",      "What the light actually looks like at six o'clock in May."),
                            ("V",    "Layers",              "The wrap, shawl or jacket that makes the evening comfortable."),
                        ].iter().map(|&(numeral, title, caption)| view! {
                            <figure class="cn-mood__item">
                                <div class="cn-mood__ph">{numeral}</div>
                                <figcaption class="cn-mood__cap">
                                    <span class="cn-mood__ttl">{title}</span>
                                    <span class="cn-mood__desc">{caption}</span>
                                </figcaption>
                            </figure>
                        }).collect_view()}
                    </div>
                </div>
            </section>

            // ── Palette ──────────────────────────────────────────────────────────
            <section class="cn-section cn-section--alt">
                <div class="wrap">
                    <div class="cn-heading cn-heading--center cn-reveal">
                        <div class="cn-eyebrow">"A Suggestion, Never A Rule"</div>
                        <h2 class="cn-h2">"The Palette"</h2>
                        <p class="cn-lede">
                            "Swatches to come. Wear the colour you feel best in — this is here for
                             anyone who would rather be told."
                        </p>
                    </div>
                    <div class="cn-swatches">
                        {[
                            "Deep navy", "Slate", "Sage", "Champagne", "Dusty rose", "Bone",
                        ].iter().map(|&label| view! {
                            <div class="cn-swatch">
                                <div class="cn-swatch__chip"></div>
                                <div class="cn-swatch__lbl">{label}</div>
                            </div>
                        }).collect_view()}
                    </div>
                </div>
            </section>

            // ── Two small lists ──────────────────────────────────────────────────
            <section class="cn-section">
                <div class="wrap">
                    <div class="cn-guide-cols">
                        <div class="cn-card">
                            <div class="cn-eyebrow">"Worth Bringing"</div>
                            <div class="cn-card__ttl">"A Short List"</div>
                            <ul class="cn-list">
                                <li>"A wrap or a jacket. The reception runs outdoors into the evening and the wind comes off the water."</li>
                                <li>"Shoes you can stand on grass in, or a second pair in the car."</li>
                                <li>"The church is not air conditioned. A layer you can take off helps."</li>
                            </ul>
                        </div>
                        <div class="cn-card">
                            <div class="cn-eyebrow">"Best Avoided"</div>
                            <div class="cn-card__ttl">"Two Requests"</div>
                            <ul class="cn-list">
                                <li>"White, ivory and cream — those are spoken for on the day."</li>
                                <li>"Stiletto heels, for the lawn's sake as much as your own."</li>
                            </ul>
                            <p class="cn-guide-cols__note">
                                "Beyond that, there is nothing you can wear that will bother us.
                                 Come as you are comfortable."
                            </p>
                        </div>
                    </div>
                </div>
            </section>

            // ── Footer ───────────────────────────────────────────────────────────
            <footer class="cn-footer">
                <div class="wrap">
                    <div class="cn-footer__mono">"C & R"</div>
                    <div class="cn-footer__name">"Still Not Sure?"</div>
                    <div class="cn-footer__meta">"Ask us — we would far rather answer than have you guess"</div>
                    <div class="row" style="justify-content:center; margin-top:40px; gap:16px; flex-wrap:wrap;">
                        <a class="cn-btn" href="/">"Back To The Invitation"</a>
                        <a class="cn-btn cn-btn--accent" href="/#rsvp">"RSVP"</a>
                    </div>
                </div>
            </footer>
        </div>
    }
}
