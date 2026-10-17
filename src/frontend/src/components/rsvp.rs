//! The RSVP card — the one part of this static page that talks to a server.
//!
//! The page itself is prerendered to flat HTML and served from a CDN, so this
//! form cannot use a Leptos server function (there is no server rendering the
//! page to route one through). It posts JSON to the small standalone API
//! instead, which is the whole point of the hybrid split.
//!
//! Guests type the last name their invitation is under and the card finds it.
//! It deliberately does not offer a menu of everyone invited: the prerendered
//! HTML carries no names, and the API will not name anybody until it is given a
//! last name that is already on the list.

use leptos::prelude::*;
use shared::{Party, RsvpReply, RsvpRequest, NAME_MAX_LEN, NOTE_MAX_LEN};

/// How long to wait for a pause in typing before looking anything up. Long
/// enough that a surname is searched once, not once per letter.
#[cfg(feature = "hydrate")]
const LOOKUP_DEBOUNCE_MS: u64 = 350;

/// Where the API lives, baked in at build time.
///
/// Unset means same-origin, which is what local preview uses. A CDN deployment
/// sets `RSVP_API_BASE` to the API's origin (for example
/// `https://api.cameronandryan.com`) before building.
#[cfg(feature = "hydrate")]
fn api_url(path: &str) -> String {
    let base = option_env!("RSVP_API_BASE").unwrap_or("");
    format!("{base}{path}")
}

/// How the search for an invitation is going.
///
/// `Matched` with nothing in it is the ordinary "no such name" answer, not a
/// failure. Only the WASM build ever moves this past `Idle`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
enum Lookup {
    Idle,
    Searching,
    Matched(Vec<Party>),
    Failed,
}

/// What the card is currently showing below the button.
///
/// `Sent` is only ever constructed by the WASM submit path; under the
/// server-render build it exists purely so the view can match on it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
enum Status {
    Idle,
    Sending,
    Sent { updated: bool, attending: bool },
    Failed(String),
}

#[component]
pub fn RsvpForm() -> impl IntoView {
    let (last_name, set_last_name) = signal(String::new());
    let lookup = RwSignal::new(Lookup::Idle);

    // The invitation being replied to. Set on its own when one name matches,
    // and by the guest when several do.
    let party = RwSignal::new(None::<Party>);
    // One answer per member. A plain Vec rather than a map: a party is a
    // handful of people, and this keeps the signal cheap to compare.
    let answers = RwSignal::new(Vec::<(i64, bool)>::new());
    // One answer per member invited with a guest, kept apart from the answers
    // above because it is a different question with a different shape: no entry
    // means it has not been answered, `None` means coming alone, and `Some`
    // carries the name of whoever they are bringing.
    let guests = RwSignal::new(Vec::<(i64, Option<String>)>::new());

    let (note, set_note) = signal(String::new());
    let (status, set_status) = signal(Status::Idle);

    // The scheduled-but-not-yet-sent lookup, so the next keystroke can call it
    // off, and a ticket so a slow early reply cannot overwrite a later one.
    #[cfg(feature = "hydrate")]
    let pending = StoredValue::new(None::<TimeoutHandle>);
    #[cfg(feature = "hydrate")]
    let generation = StoredValue::new(0u32);

    let sending = move || status.get() == Status::Sending;

    let answer_for = move |invitee_id: i64| {
        answers.with(|answers| {
            answers
                .iter()
                .find(|(id, _)| *id == invitee_id)
                .map(|(_, attending)| *attending)
        })
    };

    let set_answer = move |invitee_id: i64, attending: bool| {
        answers.update(|answers| {
            match answers.iter_mut().find(|(id, _)| *id == invitee_id) {
                Some(entry) => entry.1 = attending,
                None => answers.push((invitee_id, attending)),
            }
        });
    };

    let guest_for = move |invitee_id: i64| {
        guests.with(|guests| {
            guests
                .iter()
                .find(|(id, _)| *id == invitee_id)
                .map(|(_, guest)| guest.clone())
        })
    };

    // The name typed for a guest so far, which is empty both before anything is
    // typed and when nobody is being brought.
    let guest_name = move |invitee_id: i64| guest_for(invitee_id).flatten().unwrap_or_default();

    let set_guest = move |invitee_id: i64, guest: Option<String>| {
        guests.update(|guests| {
            match guests.iter_mut().find(|(id, _)| *id == invitee_id) {
                Some(entry) => entry.1 = guest,
                None => guests.push((invitee_id, guest)),
            }
        });
    };

    let chosen_id = move || party.with(|chosen| chosen.as_ref().map(|chosen| chosen.id));

    // Editing the name abandons whatever the previous one found: a different
    // invitation means different people to answer for.
    let on_last_name = move |ev| {
        let typed = event_target_value(&ev);
        set_last_name.set(typed.clone());
        party.set(None);
        answers.set(Vec::new());
        guests.set(Vec::new());
        set_status.set(Status::Idle);

        #[cfg(feature = "hydrate")]
        {
            use shared::checked_last_name;
            use std::time::Duration;

            pending.update_value(|handle| {
                if let Some(handle) = handle.take() {
                    handle.clear();
                }
            });

            // Too short to search on yet — say nothing rather than "not found".
            let Ok(query) = checked_last_name(&typed) else {
                lookup.set(Lookup::Idle);
                return;
            };
            let query = query.to_owned();

            let ticket = generation.get_value() + 1;
            generation.set_value(ticket);

            lookup.set(Lookup::Searching);

            let scheduled = set_timeout_with_handle(
                move || {
                    leptos::task::spawn_local(async move {
                        let found = find_party(&query).await;

                        // Another keystroke has been typed since this went out.
                        if generation.get_value() != ticket {
                            return;
                        }

                        match found {
                            // Nothing is chosen here, not even a lone match:
                            // the guest picks their invitation by hand so the
                            // card reads the same however many names matched.
                            Ok(parties) => lookup.set(Lookup::Matched(parties)),
                            Err(()) => lookup.set(Lookup::Failed),
                        }
                    });
                },
                Duration::from_millis(LOOKUP_DEBOUNCE_MS),
            );

            if let Ok(handle) = scheduled {
                pending.set_value(Some(handle));
            }
        }
    };

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        if sending() {
            return;
        }

        let Some(chosen) = party.get_untracked() else {
            set_status.set(Status::Failed(
                "Please enter the last name on your invitation first.".into(),
            ));
            return;
        };

        let party_id = chosen.id;

        // Everyone on the invitation has to be answered for, so nobody is
        // silently left off the count.
        let mut replies = Vec::with_capacity(chosen.members.len());
        for member in &chosen.members {
            let Some(attending) = answer_for(member.id) else {
                set_status.set(Status::Failed(if chosen.members.len() == 1 {
                    "Please let us know whether you can make it.".into()
                } else {
                    format!("Please let us know whether {} can make it.", member.name)
                }));
                return;
            };
            // A guest is only asked about for someone who is coming, so a
            // declined answer carries nobody — whatever was typed before the
            // answer changed.
            let plus_one_name = if attending && member.plus_one {
                match guest_for(member.id) {
                    None => {
                        set_status.set(Status::Failed(if chosen.members.len() == 1 {
                            "Please let us know whether you are bringing a guest.".into()
                        } else {
                            format!(
                                "Please let us know whether {} is bringing a guest.",
                                member.name
                            )
                        }));
                        return;
                    }
                    Some(Some(guest)) if guest.trim().is_empty() => {
                        set_status.set(Status::Failed(if chosen.members.len() == 1 {
                            "Please give your guest's name.".into()
                        } else {
                            format!("Please give the name of {}'s guest.", member.name)
                        }));
                        return;
                    }
                    Some(guest) => guest,
                }
            } else {
                None
            };

            replies.push(RsvpReply {
                invitee_id: member.id,
                attending,
                plus_one_name,
            });
        }

        let payload = RsvpRequest {
            party_id,
            last_name: last_name.get_untracked(),
            replies,
            note: Some(note.get_untracked()),
        }
        .normalized();

        // Checked here as well as on the server so a mistake gets an instant
        // answer instead of a round trip.
        if let Err(invalid) = payload.validate() {
            set_status.set(Status::Failed(invalid.message().to_owned()));
            return;
        }

        set_status.set(Status::Sending);

        #[cfg(feature = "hydrate")]
        leptos::task::spawn_local(async move {
            set_status.set(send(payload).await);
        });
    };

    view! {
        <form class="cn-card cn-rsvp-card" on:submit=submit novalidate>
            <div class="cn-field">
                <label class="cn-field__lbl" for="rsvp-last-name">
                    "Enter the Last Name of a Member of Your Party"
                </label>
                <input
                    id="rsvp-last-name"
                    class="cn-field__inp"
                    type="text"
                    name="last_name"
                    autocomplete="family-name"
                    maxlength=NAME_MAX_LEN.to_string()
                    placeholder="Last name…"
                    prop:value=move || last_name.get()
                    on:input=on_last_name
                />

                {move || match lookup.get() {
                    Lookup::Idle => ().into_any(),
                    Lookup::Searching => view! {
                        <p class="cn-field__hint">"Looking for your invitation…"</p>
                    }
                    .into_any(),
                    Lookup::Failed => view! {
                        <p class="cn-field__hint cn-field__hint--err">
                            "We couldn't reach the server. Please try again, or email us directly."
                        </p>
                    }
                    .into_any(),
                    Lookup::Matched(parties) if parties.is_empty() => view! {
                        <p class="cn-field__hint cn-field__hint--err">
                            "We couldn't find an invitation under that name. Please check the spelling, or email us."
                        </p>
                    }
                    .into_any(),
                    // Every match is picked by hand, a lone one included. The
                    // guest confirming which invitation is theirs before
                    // answering for the people on it is worth the extra tap,
                    // and one path through the card beats two.
                    Lookup::Matched(parties) => {
                        let only_one = parties.len() == 1;

                        view! {
                            <p class="cn-field__hint">
                                {if only_one {
                                    "We found your invitation. Tap it to confirm."
                                } else {
                                    "More than one invitation matches that name. Which is yours?"
                                }}
                            </p>
                            <div class="cn-field__choice cn-field__choice--stack">
                                {parties
                                    .into_iter()
                                    .map(|option| {
                                        let id = option.id;
                                        let label = option.label();
                                        view! {
                                            <button
                                                type="button"
                                                class="cn-tab cn-tab--pill"
                                                class:active=move || chosen_id() == Some(id)
                                                aria-pressed=move || {
                                                    (chosen_id() == Some(id)).to_string()
                                                }
                                                on:click=move |_| party.set(Some(option.clone()))
                                            >
                                                {label}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                        .into_any()
                    }
                }}
            </div>

            // One question per person on the invitation, so a couple or a
            // family answers for everyone in a single reply.
            <div class="cn-field">
                {move || {
                    let Some(chosen) = party.get() else {
                        return view! {
                            <p class="cn-field__hint">
                                "Find your invitation above and we'll ask about everyone on it."
                            </p>
                        }
                        .into_any();
                    };

                    let named = chosen.members.len() > 1;

                    chosen
                        .members
                        .into_iter()
                        .map(|member| {
                            let id = member.id;
                            let label = if named {
                                format!("Will you attend? — {}", member.name)
                            } else {
                                "Will you attend?".to_owned()
                            };
                            let group_label = label.clone();

                            // Asked only where the invitation carries a guest,
                            // and only once this person has accepted: there is
                            // nobody to bring to a wedding you are not at.
                            let plus_one = member.plus_one;
                            let question = if named {
                                format!("Is {} bringing a guest?", member.name)
                            } else {
                                "Are you bringing a guest?".to_owned()
                            };

                            view! {
                                <div class="cn-field__guest">
                                    <span class="cn-field__lbl">{label}</span>
                                    <div
                                        class="cn-field__choice"
                                        role="group"
                                        aria-label=group_label
                                    >
                                        <button
                                            type="button"
                                            class="cn-tab cn-tab--pill"
                                            class:active=move || answer_for(id) == Some(true)
                                            aria-pressed=move || {
                                                (answer_for(id) == Some(true)).to_string()
                                            }
                                            on:click=move |_| set_answer(id, true)
                                        >
                                            "Joyfully Accepts"
                                        </button>
                                        <button
                                            type="button"
                                            class="cn-tab cn-tab--pill"
                                            class:active=move || answer_for(id) == Some(false)
                                            aria-pressed=move || {
                                                (answer_for(id) == Some(false)).to_string()
                                            }
                                            on:click=move |_| set_answer(id, false)
                                        >
                                            "Regretfully Declines"
                                        </button>
                                    </div>

                                    {move || {
                                        if !plus_one || answer_for(id) != Some(true) {
                                            return ().into_any();
                                        }

                                        let question = question.clone();
                                        let bringing =
                                            move || matches!(guest_for(id), Some(Some(_)));

                                        view! {
                                            <div class="cn-field__plus">
                                                <span class="cn-field__lbl">
                                                    {question.clone()}
                                                </span>
                                                <div
                                                    class="cn-field__choice"
                                                    role="group"
                                                    aria-label=question
                                                >
                                                    <button
                                                        type="button"
                                                        class="cn-tab cn-tab--pill"
                                                        class:active=move || bringing()
                                                        aria-pressed=move || bringing().to_string()
                                                        // Carries over whatever name was typed
                                                        // before, so answering twice does not
                                                        // empty the box.
                                                        on:click=move |_| {
                                                            set_guest(id, Some(guest_name(id)))
                                                        }
                                                    >
                                                        "Bringing A Guest"
                                                    </button>
                                                    <button
                                                        type="button"
                                                        class="cn-tab cn-tab--pill"
                                                        class:active=move || {
                                                            guest_for(id) == Some(None)
                                                        }
                                                        aria-pressed=move || {
                                                            (guest_for(id) == Some(None)).to_string()
                                                        }
                                                        on:click=move |_| set_guest(id, None)
                                                    >
                                                        "Coming Alone"
                                                    </button>
                                                </div>

                                                {move || {
                                                    bringing()
                                                        .then(|| {
                                                            view! {
                                                                <input
                                                                    class="cn-field__inp"
                                                                    type="text"
                                                                    maxlength=NAME_MAX_LEN
                                                                        .to_string()
                                                                    placeholder="Your guest's name…"
                                                                    aria-label="Your guest's name"
                                                                    prop:value=move || {
                                                                        guest_name(id)
                                                                    }
                                                                    on:input=move |ev| {
                                                                        set_guest(
                                                                            id,
                                                                            Some(event_target_value(&ev)),
                                                                        )
                                                                    }
                                                                />
                                                            }
                                                        })
                                                }}
                                            </div>
                                        }
                                            .into_any()
                                    }}
                                </div>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>

            <div class="cn-field">
                <label class="cn-field__lbl" for="rsvp-note">"A Note To The Couple"</label>
                <textarea
                    id="rsvp-note"
                    class="cn-field__area"
                    name="note"
                    maxlength=NOTE_MAX_LEN.to_string()
                    placeholder="A short message…"
                    prop:value=move || note.get()
                    on:input=move |ev| set_note.set(event_target_value(&ev))
                ></textarea>
            </div>

            <div class="row-end">
                <button type="submit" class="cn-btn cn-btn--accent" disabled=sending>
                    {move || if sending() { "Sending…" } else { "Submit Reply" }}
                </button>
            </div>

            // `aria-live` so a screen reader announces the outcome, which is
            // otherwise only visible as a colour change below the button.
            <div class="cn-rsvp-status" role="status" aria-live="polite">
                {move || match status.get() {
                    Status::Idle | Status::Sending => None,
                    Status::Sent { updated, attending } => Some(view! {
                        <p class="cn-rsvp-status__ok">
                            {match (updated, attending) {
                                (true, true) => "Reply updated — we can't wait to see you.",
                                (true, false) => "Reply updated — thank you for letting us know.",
                                (false, true) => "Thank you — we can't wait to see you.",
                                (false, false) => "Thank you for letting us know — you'll be missed.",
                            }}
                        </p>
                    }.into_any()),
                    Status::Failed(message) => Some(view! {
                        <p class="cn-rsvp-status__err">{message}</p>
                    }.into_any()),
                }}
            </div>
        </form>
    }
}

/// Asks which invitations carry a last name.
///
/// A POST, though it only reads: a query string would put guests' names into
/// CDN access logs and browser history.
#[cfg(feature = "hydrate")]
async fn find_party(last_name: &str) -> Result<Vec<Party>, ()> {
    use gloo_net::http::Request;
    use shared::{PartyLookupRequest, PartyLookupResponse};

    let body = PartyLookupRequest {
        last_name: last_name.to_owned(),
    };

    let request = Request::post(&api_url("/api/lookup"))
        .json(&body)
        .map_err(|e| leptos::logging::error!("could not encode lookup: {e}"))?;

    let response = request
        .send()
        .await
        .map_err(|e| leptos::logging::error!("lookup request failed: {e}"))?;

    if !response.ok() {
        leptos::logging::error!("lookup returned {}", response.status());
        return Err(());
    }

    response
        .json::<PartyLookupResponse>()
        .await
        .map(|found| found.parties)
        .map_err(|e| leptos::logging::error!("could not decode lookup: {e}"))
}

/// Posts the reply and turns whatever comes back into a [`Status`].
#[cfg(feature = "hydrate")]
async fn send(payload: RsvpRequest) -> Status {
    use gloo_net::http::Request;
    use shared::{ApiError, RsvpResponse};

    const UNREACHABLE: &str =
        "We couldn't reach the server. Please try again, or email us directly.";

    let request = match Request::post(&api_url("/api/rsvp")).json(&payload) {
        Ok(request) => request,
        Err(e) => {
            leptos::logging::error!("could not encode RSVP: {e}");
            return Status::Failed(UNREACHABLE.into());
        }
    };

    let response = match request.send().await {
        Ok(response) => response,
        Err(e) => {
            leptos::logging::error!("RSVP request failed: {e}");
            return Status::Failed(UNREACHABLE.into());
        }
    };

    if !response.ok() {
        // A validation rejection carries a human-readable reason; anything else
        // gets the generic message rather than leaking a status code.
        return Status::Failed(match response.json::<ApiError>().await {
            Ok(ApiError { error }) => error,
            Err(_) => UNREACHABLE.into(),
        });
    }

    match response.json::<RsvpResponse>().await {
        Ok(saved) => Status::Sent {
            updated: saved.updated,
            attending: saved.any_attending(),
        },
        Err(e) => {
            leptos::logging::error!("could not decode RSVP response: {e}");
            // The rows were written; only our reading of the reply failed.
            Status::Sent {
                updated: false,
                attending: payload.replies.iter().any(|reply| reply.attending),
            }
        }
    }
}
