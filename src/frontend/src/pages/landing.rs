use leptos::prelude::*;

const PHOTO_BASE: &str = "https://wedding-site-photos-254325977215.s3.us-east-1.amazonaws.com";

const GALLERY: [(&str, u32, u32); 18] = [
    ("p01", 1100, 731),
    ("p02", 733, 1100),
    ("p03", 1100, 733),
    ("p04", 1100, 733),
    ("p05", 732, 1100),
    ("p06", 1100, 731),
    ("p07", 1100, 733),
    ("p08", 733, 1100),
    ("p09", 1100, 731),
    ("p10", 1100, 733),
    ("p11", 733, 1100),
    ("p12", 1100, 733),
    ("p13", 1100, 733),
    ("p14", 733, 1100),
    ("p15", 1100, 733),
    ("p16", 1100, 733),
    ("p17", 733, 1100),
    ("p18", 733, 1100),
];

#[component]
fn Hydrangeas() -> impl IntoView {
    view! {
        <div class="std-hydrangeas" aria-hidden="true">
            {(0..9).map(|_| view! { <i></i> }).collect_view()}
        </div>
    }
}

#[component]
pub fn Landing() -> impl IntoView {
    let (lightbox, set_lightbox) = signal(None::<usize>);

    let step_photo = move |delta: isize| {
        set_lightbox.update(|current| {
            if let Some(index) = *current {
                let count = GALLERY.len() as isize;
                *current = Some((index as isize + delta).rem_euclid(count) as usize);
            }
        });
    };

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

            let init = Closure::once_into_js(init_landing);
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    init.as_ref().unchecked_ref(),
                    0,
                );
            }
        }
    });

    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;
            use wasm_bindgen::JsCast;

            let on_key = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(
                move |event: web_sys::KeyboardEvent| {
                    if lightbox.get_untracked().is_none() {
                        return;
                    }
                    match event.key().as_str() {
                        "Escape" => set_lightbox.set(None),
                        "ArrowLeft" => step_photo(-1),
                        "ArrowRight" => step_photo(1),
                        _ => return,
                    }
                    event.prevent_default();
                },
            );

            if let Some(window) = web_sys::window() {
                let _ = window
                    .add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref());
            }
            on_key.forget();
        }
    });

    view! {
        <div class="std-site">
            <section id="top" class="std-opening" aria-label="Save the date invitation">
                <div class="std-opening__sticky">
                    <div class="std-wash std-wash--one"></div>
                    <div class="std-wash std-wash--two"></div>

                    <div class="std-invite">
                        <article class="std-card">
                            <div class="std-card__rule"><span></span></div>
                            <p class="std-kicker">"Save the date"</p>
                            <h1>"Cameron" <em>"&"</em> "Ryan"</h1>
                            <p class="std-date">"Saturday · May 15 · 2027"</p>
                            <p class="std-place">"Bay Head, New Jersey"</p>

                            <div class="std-details">
                                <div>
                                    <span>"Four o'clock"</span>
                                    <strong>"Ceremony"</strong>
                                    <p>"All Saints Episcopal Church"</p>
                                </div>
                                <div>
                                    <span>"Six o'clock"</span>
                                    <strong>"Reception"</strong>
                                    <p>"The Grenville"</p>
                                </div>
                            </div>

                            <a class="std-gallery-link" href="#gallery">"Gallary"</a>
                            <div class="std-card__rule std-card__rule--bottom"><span></span></div>
                        </article>

                        <div class="std-flap std-flap--left" aria-hidden="true"><Hydrangeas/></div>
                        <div class="std-flap std-flap--right" aria-hidden="true"><Hydrangeas/></div>
                        <div class="std-flap std-flap--top" aria-hidden="true"><Hydrangeas/></div>
                        <div class="std-flap std-flap--bottom" aria-hidden="true"><Hydrangeas/></div>

                        <div class="std-cover-copy" aria-hidden="true">
                            <span>"Save the date"</span>
                            <strong>"C & R"</strong>
                            <small>"Scroll to open"</small>
                        </div>
                    </div>
                </div>
            </section>

            <section id="gallery" class="std-gallery" aria-labelledby="gallery-title">
                <header class="std-gallery__header">
                    <p>"Cameron & Ryan"</p>
                    <h2 id="gallery-title">"Our gallery"</h2>
                    <span>"Engagement & before"</span>
                </header>

                <div class="std-gallery__grid">
                    {GALLERY
                        .iter()
                        .enumerate()
                        .map(|(index, &(key, width, height))| {
                            view! {
                                <button
                                    type="button"
                                    class="std-gallery__item"
                                    aria-label=format!("Open photograph {} of {}", index + 1, GALLERY.len())
                                    on:click=move |_| set_lightbox.set(Some(index))
                                >
                                    <img
                                        src=format!("{PHOTO_BASE}/photos/thumb/{key}.jpg")
                                        width=width
                                        height=height
                                        loading=if index < 4 { "eager" } else { "lazy" }
                                        decoding="async"
                                        alt=format!("Cameron and Ryan, photograph {} of {}", index + 1, GALLERY.len())
                                    />
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </section>

            <footer class="std-footer">
                <span>"Cameron & Ryan"</span>
                <span>"05 · 15 · 27"</span>
            </footer>

            <Show when=move || lightbox.get().is_some()>
                <div
                    class="std-lightbox"
                    role="dialog"
                    aria-modal="true"
                    aria-label="Photograph"
                    on:click=move |event| {
                        if event.target() == event.current_target() {
                            set_lightbox.set(None);
                        }
                    }
                >
                    <button class="std-lightbox__close" type="button" aria-label="Close" on:click=move |_| set_lightbox.set(None)>"×"</button>
                    <button class="std-lightbox__previous" type="button" aria-label="Previous photograph" on:click=move |_| step_photo(-1)>"‹"</button>
                    <img
                        src=move || lightbox.get().map(|index| format!("{PHOTO_BASE}/photos/full/{}.jpg", GALLERY[index].0)).unwrap_or_default()
                        alt=move || lightbox.get().map(|index| format!("Cameron and Ryan, photograph {} of {}", index + 1, GALLERY.len())).unwrap_or_default()
                    />
                    <button class="std-lightbox__next" type="button" aria-label="Next photograph" on:click=move |_| step_photo(1)>"›"</button>
                    <span>{move || lightbox.get().map(|index| format!("{} / {}", index + 1, GALLERY.len())).unwrap_or_default()}</span>
                </div>
            </Show>
        </div>
    }
}
