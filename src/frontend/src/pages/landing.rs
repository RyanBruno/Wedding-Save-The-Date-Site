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

const FLORETS: [(i16, i16, i16, f32); 36] = [
    (0, 0, 0, 1.06),
    (-31, -14, -13, 1.04),
    (31, -17, 9, 1.02),
    (-8, -37, 18, 1.0),
    (10, 37, -8, 1.08),
    (-37, 23, 14, 1.03),
    (39, 20, -17, 1.0),
    (-57, -36, 5, 0.98),
    (-29, -54, -19, 1.06),
    (4, -66, 12, 0.96),
    (39, -51, 23, 1.02),
    (63, -28, -8, 1.05),
    (68, 7, 15, 0.97),
    (59, 42, -22, 1.04),
    (30, 61, 9, 1.0),
    (-5, 69, -12, 1.08),
    (-40, 59, 21, 0.96),
    (-63, 39, -7, 1.03),
    (-71, 4, 17, 1.0),
    (-87, -42, -20, 0.91),
    (-58, -72, 8, 0.95),
    (-20, -91, -8, 0.9),
    (20, -91, 19, 0.93),
    (58, -74, -13, 0.98),
    (87, -47, 12, 0.9),
    (96, -10, -18, 0.95),
    (93, 31, 5, 0.91),
    (72, 70, -15, 0.97),
    (34, 91, 18, 0.92),
    (-8, 99, -5, 0.96),
    (-48, 88, 14, 0.92),
    (-82, 63, -18, 0.96),
    (-99, 24, 7, 0.9),
    (-101, -13, 20, 0.94),
    (1, -19, -21, 0.9),
    (27, 14, 11, 0.91),
];

#[component]
fn HydrangeaFlap(side: &'static str) -> impl IntoView {
    let clip_id = format!("std-flap-clip-{side}");
    let floret_id = format!("std-floret-{side}");
    let head_id = format!("std-head-{side}");
    let petal_id = format!("std-petal-{side}");
    let leaf_id = format!("std-leaf-{side}");
    let leaf_fill_id = format!("std-leaf-fill-{side}");
    let paper_id = format!("std-paper-{side}");
    let clip = format!("url(#{clip_id})");
    let floret = format!("#{floret_id}");
    let head = format!("#{head_id}");
    let leaf = format!("#{leaf_id}");
    let petal = format!("url(#{petal_id})");
    let paper = format!("url(#{paper_id})");
    let silhouette = if side == "left" {
        "M0 0H307Q335 1 325 17Q355 12 344 33Q376 31 360 52Q394 54 374 72Q405 82 381 94Q405 111 378 119Q398 139 369 141Q387 164 357 160Q368 187 337 178Q347 207 318 192Q333 224 307 211Q322 238 342 235Q372 232 360 255Q397 249 382 276Q425 270 406 297Q455 294 433 320Q478 314 461 342Q507 338 484 362Q500 382 469 380Q487 405 455 397Q470 427 438 414Q448 444 416 430Q421 457 393 448Q409 477 380 464Q400 494 371 489Q400 519 379 533Q406 548 383 560Q404 579 378 586Q397 609 367 608Q380 632 348 627Q357 649 325 650H0Z"
    } else {
        "M0 0H318Q347 2 336 19Q365 15 354 37Q387 35 370 56Q402 59 381 78Q412 88 388 101Q410 119 382 126Q403 146 374 148Q390 172 360 167Q374 194 342 184Q351 211 323 199Q337 228 312 216Q328 242 347 241Q378 237 366 261Q401 256 387 282Q431 277 412 304Q461 301 439 327Q485 321 467 350Q507 346 485 368Q500 389 470 387Q487 413 455 404Q468 434 437 421Q447 451 414 437Q420 465 392 455Q407 483 379 472Q398 502 369 496Q397 526 376 540Q402 554 379 567Q399 586 373 592Q391 615 361 614Q374 637 342 633Q351 649 319 650H0Z"
    };
    let art_class = format!("std-flap-art std-flap-art--{side}");

    view! {
        <svg class=art_class viewBox="0 0 520 650" preserveAspectRatio="none" aria-hidden="true">
            <defs>
                <linearGradient id=paper_id x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0" stop-color="#f8fafc"/>
                    <stop offset=".58" stop-color="#edf1f7"/>
                    <stop offset="1" stop-color="#dce4ef"/>
                </linearGradient>
                <radialGradient id=petal_id cx="34%" cy="22%" r="84%">
                    <stop offset="0" stop-color="#ffffff"/>
                    <stop offset=".52" stop-color="#f5f7fa"/>
                    <stop offset=".82" stop-color="#e9edf4"/>
                    <stop offset="1" stop-color="#d2daea"/>
                </radialGradient>
                <linearGradient id=leaf_fill_id.clone() x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0" stop-color="#c4cfaa"/>
                    <stop offset=".46" stop-color="#98aa7c"/>
                    <stop offset="1" stop-color="#73865e"/>
                </linearGradient>
                <clipPath id=clip_id>
                    <path d=silhouette/>
                </clipPath>
                <g id=floret_id>
                    <g class="std-floret__shape">
                        <path class="std-floret__petal" fill=petal.clone() d="M0-1C-7-4-15-12-16-21C-17-29-12-35-5-35C-2-35 0-33 1-30C4-35 10-35 14-31C20-24 16-14 11-8C7-4 3-2 0-1Z"/>
                        <path class="std-floret__petal" fill=petal.clone() transform="rotate(88) scale(.97 1.02)" d="M0-1C-7-4-15-12-16-21C-17-29-12-35-5-35C-2-35 0-33 1-30C4-35 10-35 14-31C20-24 16-14 11-8C7-4 3-2 0-1Z"/>
                        <path class="std-floret__petal" fill=petal.clone() transform="rotate(181) scale(1.03 .96)" d="M0-1C-7-4-15-12-16-21C-17-29-12-35-5-35C-2-35 0-33 1-30C4-35 10-35 14-31C20-24 16-14 11-8C7-4 3-2 0-1Z"/>
                        <path class="std-floret__petal" fill=petal transform="rotate(272) scale(.95 1.04)" d="M0-1C-7-4-15-12-16-21C-17-29-12-35-5-35C-2-35 0-33 1-30C4-35 10-35 14-31C20-24 16-14 11-8C7-4 3-2 0-1Z"/>
                        <path class="std-floret__wash" d="M-10-3C-8-11-3-14 1-10C5-14 11-9 10-3C6 2-5 3-10-3Z"/>
                        <g class="std-floret__center">
                            <circle cx="-1.8" cy="-1.4" r="1.35"/>
                            <circle cx="1.7" cy="-1" r="1.1"/>
                            <circle cx=".4" cy="1.8" r="1.25"/>
                            <circle class="std-floret__center-highlight" cx="-2.1" cy="-1.8" r=".45"/>
                        </g>
                    </g>
                </g>
                <g id=head_id>
                    {FLORETS
                        .iter()
                        .enumerate()
                        .map(|(index, &(x, y, rotation, scale))| {
                            let class = format!("std-floret std-floret--{}", index % 4);
                            view! {
                                <use
                                    href=floret.clone()
                                    class=class
                                    transform=format!("translate({x} {y}) rotate({rotation}) scale({scale})")
                                />
                            }
                        })
                        .collect_view()}
                </g>
                <g id=leaf_id>
                    <g class="std-leaf__shape">
                        <path class="std-leaf__body" fill=format!("url(#{leaf_fill_id})") d="M0 0C12-14 28-24 43-27L49-31L55-28C70-27 84-19 97-6C85 5 70 15 54 19L48 22L42 19C25 18 11 10 0 0Z"/>
                        <path class="std-leaf__wash" d="M8 1C30-7 53-13 88-7C65 4 40 12 8 1Z"/>
                        <path class="std-leaf__vein" d="M6 1C32-2 60-5 90-7M24-1C30-7 36-13 43-19M38-3C44 3 50 8 57 13M51-5C58-11 63-16 69-19M65-6C71-1 75 3 80 6M76-7C80-11 83-14 86-15"/>
                    </g>
                </g>
            </defs>

            <g clip-path=clip>
                <path class="std-flap-art__paper" fill=paper d=silhouette/>
                <use href=head.clone() class="std-flower-head std-flower-head--top" transform="translate(183 104) scale(1.77)"/>
                <use href=head.clone() class="std-flower-head std-flower-head--middle" transform="translate(284 328) rotate(7) scale(1.84)"/>
                <use href=head class="std-flower-head std-flower-head--bottom" transform="translate(184 555) rotate(-5) scale(1.8)"/>
            </g>

            <g class="std-flap-leaves">
                <use href=leaf.clone() transform="translate(397 275) rotate(-48) scale(.76)"/>
                <use href=leaf.clone() transform="translate(378 393) rotate(35) scale(.72)"/>
                <use href=leaf transform="translate(61 586) rotate(154) scale(.78)"/>
            </g>
        </svg>
    }
}

#[component]
fn WaxSeal() -> impl IntoView {
    view! {
        <div class="std-wax-seal" aria-hidden="true">
            <svg viewBox="0 0 160 160">
                <defs>
                    <radialGradient id="std-wax-fill" cx="31%" cy="24%" r="78%">
                        <stop offset="0" stop-color="#b4c2ce"/>
                        <stop offset=".46" stop-color="#8e9fae"/>
                        <stop offset="1" stop-color="#6f8191"/>
                    </radialGradient>
                    <linearGradient id="std-wax-rim" x1="0" y1="0" x2="1" y2="1">
                        <stop offset="0" stop-color="#c0ccd6"/>
                        <stop offset=".42" stop-color="#91a2b1"/>
                        <stop offset="1" stop-color="#687b8b"/>
                    </linearGradient>
                    <filter id="std-wax-letter" x="-30%" y="-30%" width="160%" height="160%">
                        <feDropShadow dx="-1.4" dy="-1.4" stdDeviation=".7" flood-color="#cbd5dd" flood-opacity=".9"/>
                        <feDropShadow dx="1.5" dy="1.7" stdDeviation=".8" flood-color="#566a7b" flood-opacity=".8"/>
                    </filter>
                </defs>
                <path class="std-wax-seal__body" d="M82 7C94 10 101 14 112 16C124 19 126 30 135 38C143 47 139 58 146 69C151 80 144 90 143 101C141 113 130 117 123 127C116 137 104 135 93 143C82 151 72 143 60 144C48 144 42 134 31 129C20 124 20 111 14 101C8 91 14 81 12 69C10 57 20 50 24 39C28 27 41 26 51 18C60 10 71 13 82 7Z"/>
                <circle class="std-wax-seal__rim" cx="80" cy="78" r="57"/>
                <circle class="std-wax-seal__stamp" cx="80" cy="78" r="42"/>
                <path class="std-wax-seal__inner-shadow" d="M42 70A42 42 0 0 1 118 69"/>
                <text class="std-wax-seal__letter std-wax-seal__letter--c" x="65" y="93" text-anchor="middle">"C"</text>
                <text class="std-wax-seal__letter std-wax-seal__letter--r" x="94" y="109" text-anchor="middle">"R"</text>
                <path class="std-wax-seal__flourish" d="M51 112C65 118 88 116 104 105"/>
            </svg>
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

                        <div class="std-flap std-flap--left" aria-hidden="true">
                            <HydrangeaFlap side="left"/>
                            <WaxSeal/>
                        </div>
                        <div class="std-flap std-flap--right" aria-hidden="true">
                            <HydrangeaFlap side="right"/>
                        </div>

                        <div class="std-cover-copy" aria-hidden="true">
                            <span>"Save the date"</span>
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
