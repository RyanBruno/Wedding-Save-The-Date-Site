//! The admin dashboard: the one place the whole guest list is visible.
//!
//! Three things are deliberate about where this lives.
//!
//! **It is not a page of the site.** Every route in the Leptos app is
//! prerendered to flat HTML and packaged for nginx to serve, so anything
//! defined there would end up as a world-readable file the CDN would happily
//! copy to every edge. The dashboard is served by this process, from bytes
//! compiled into the binary, and appears nowhere in `target/site`.
//!
//! **It sits under `/api`.** That prefix answers `Cache-Control: no-store` on
//! every response — see [`crate::api::router`] — so no shared cache may keep a
//! page of the guest list, which is exactly what a signed-in page needs.
//! Nothing else has to be configured.
//!
//! **It carries no CORS layer.** The guest-facing routes allow a browser on the
//! CDN's origin to post a reply; the dashboard is only ever fetched by itself,
//! so no other origin is invited to try. See [`crate::api::router`] for why
//! merging after the layer is what leaves these routes uncovered.

mod auth;
mod data;

use axum::extract::{Path, State};
use axum::http::header::{self, HeaderName, HeaderValue};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use shared::ApiError;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::state::AppState;
use auth::{AdminSession, LoginRejected};
pub use auth::AdminAuth;

/// The dashboard, as three files compiled into the binary.
///
/// Served as separate responses rather than one inlined blob: nothing under
/// `/api` is cached by anyone, so these three are re-fetched every time the
/// page loads, but keeping the markup, the styling and the behaviour in their
/// own files is worth more than the two extra requests cost.
const HTML: &str = include_str!("assets/admin.html");
const CSS: &str = include_str!("assets/admin.css");
const JS: &str = include_str!("assets/admin.js");

/// Routes for the dashboard. Merged into the `/api` router, so every path
/// here ends up under `/api/admin`.
pub fn router() -> Router<AppState> {
    // Spelled out rather than nested under `/admin`, so that both `/api/admin`
    // and `/api/admin/` land on the page. A nest answers only one of the two,
    // and the one it answers is the one nobody types.
    Router::new()
        // The page itself, plus the sign-in exchange. Open by necessity —
        // there has to be somewhere to type the password.
        .route("/admin", get(page))
        .route("/admin/", get(page))
        .route("/admin/admin.css", get(stylesheet))
        .route("/admin/admin.js", get(script))
        .route("/admin/session", post(sign_in).delete(sign_out))
        // Everything below takes an `AdminSession`, so a request without a live
        // cookie never reaches the handler.
        .route("/admin/overview", get(overview))
        .route("/admin/export.csv", get(export_csv))
        .route("/admin/parties", post(create_party))
        .route("/admin/parties/{id}", delete(delete_party))
        .route("/admin/parties/{id}/invitees", post(add_invitee))
        .route("/admin/parties/{id}/note", put(set_note))
        .route("/admin/invitees/{id}", put(update_invitee).delete(delete_invitee))
        .route("/admin/invitees/{id}/rsvp", put(set_rsvp))
        .route("/admin/invitees/{id}/plus-one", put(set_plus_one))
        // Belt and braces: the path is not linked from anywhere and no cache
        // is allowed to hold it, but a guest list is not something to leave to
        // a crawler's good manners. `Cache-Control` is not set here —
        // [`crate::api::router`] puts `no-store` on the whole prefix.
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow"),
        ))
}

// ── The page ─────────────────────────────────────────────────────────────────

/// The dashboard shell.
///
/// Always 200, and always the same HTML: the page decides for itself whether to
/// show the sign-in card or the board, from a flag this handler writes into it.
/// Serving a different document to a signed-out visitor would make the browser
/// cache the wrong one at exactly the moment the session changes.
async fn page(State(state): State<AppState>, parts: Parts) -> Response {
    if !state.admin.enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            DISABLED_HTML,
        )
            .into_response();
    }

    let signed_in = auth::signed_in(&state.admin, &parts);
    let html = HTML.replace("{{SIGNED_IN}}", if signed_in { "true" } else { "false" });

    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

async fn stylesheet() -> Response {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], CSS).into_response()
}

async fn script() -> Response {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        JS,
    )
        .into_response()
}

/// Shown when `ADMIN_PASSWORD` is unset. Explains the fix rather than 404ing,
/// because the only person who can reach this is the one who owns the box.
const DISABLED_HTML: &str = concat!(
    "<!doctype html><meta charset=\"utf-8\">",
    "<title>Admin unavailable</title>",
    "<style>body{font:15px/1.6 system-ui,sans-serif;max-width:34rem;margin:12vh auto;padding:0 1.5rem;color:#1b1f24}",
    "code{background:#f1f3f5;padding:.15em .4em;border-radius:4px}</style>",
    "<h1>Admin dashboard is not configured</h1>",
    "<p>Set <code>ADMIN_PASSWORD</code> in the API's environment and restart it.</p>"
);

// ── Sign in, sign out ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SignInRequest {
    password: String,
}

#[derive(Serialize)]
struct SignedIn {
    ok: bool,
}

async fn sign_in(State(state): State<AppState>, Json(body): Json<SignInRequest>) -> Response {
    match state.admin.sign_in(&body.password) {
        Ok(token) => (
            StatusCode::OK,
            [(header::SET_COOKIE, state.admin.cookie(&token))],
            Json(SignedIn { ok: true }),
        )
            .into_response(),

        Err(LoginRejected::Disabled) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "The admin dashboard is not configured.".into(),
        ),

        Err(LoginRejected::Wrong) => {
            error(StatusCode::UNAUTHORIZED, "That password is not right.".into())
        }

        Err(LoginRejected::LockedOut(wait)) => error(
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "Too many attempts. Try again in {} seconds.",
                wait.as_secs().max(1)
            ),
        ),
    }
}

/// Ends this browser's session. Takes no session extractor — signing out of an
/// expired session should succeed quietly, not 401.
async fn sign_out(State(state): State<AppState>, parts: Parts) -> Response {
    if let Some(token) = auth::token_from(&parts) {
        state.admin.sign_out(&token);
    }

    (
        StatusCode::OK,
        [(header::SET_COOKIE, state.admin.clearing_cookie())],
        Json(SignedIn { ok: true }),
    )
        .into_response()
}

// ── Reads ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Overview {
    parties: Vec<data::AdminParty>,
}

/// The whole board in one response. Counts, filtering and sorting all happen in
/// the browser, so every edit is followed by one refetch and the screen can
/// never show a stat that disagrees with the table under it.
async fn overview(_: AdminSession, State(state): State<AppState>) -> Response {
    match data::overview(&state.db).await {
        Ok(parties) => (StatusCode::OK, Json(Overview { parties })).into_response(),
        Err(e) => failed(e),
    }
}

/// The guest list as a spreadsheet — one row per guest, the invitation's note
/// repeated alongside each of its members so the file sorts and filters on its
/// own.
async fn export_csv(_: AdminSession, State(state): State<AppState>) -> Response {
    let parties = match data::overview(&state.db).await {
        Ok(parties) => parties,
        Err(e) => return failed(e),
    };

    let mut csv = String::from("Invitation,Guest,Reply,Replied at,Plus one,Note\n");

    for party in &parties {
        let note = party.note.clone().unwrap_or_default();

        for member in &party.members {
            let reply = match member.attending {
                Some(true) => "Attending",
                Some(false) => "Not attending",
                None => "Awaiting reply",
            };

            // One column carries both halves of a plus one: empty for a guest
            // who was never offered one, the guest's name once it is known, and
            // the allowance itself in between, so a head count off this file
            // never has to join two columns to know who is coming.
            let plus_one = match (member.plus_one, member.plus_one_name.as_deref()) {
                (_, Some(name)) => name,
                (true, None) => "Invited with a guest",
                (false, None) => "",
            };

            csv.push_str(&[
                quote(&party.name),
                quote(&member.name),
                quote(reply),
                quote(member.replied_at.as_deref().unwrap_or("")),
                quote(plus_one),
                quote(&note),
            ]
            .join(","));
            csv.push('\n');
        }
    }

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"guest-list.csv\"",
            ),
        ],
        csv,
    )
        .into_response()
}

/// Every field is quoted, so a name with a comma in it — or a note with a
/// newline — cannot shift the columns.
fn quote(field: &str) -> String {
    format!("\"{}\"", field.replace('"', "\"\""))
}

// ── Writes ───────────────────────────────────────────────────────────────────

/// What every mutation returns. The dashboard refetches the overview rather
/// than patching its own copy, so there is nothing else worth sending back.
#[derive(Serialize)]
struct Done {
    ok: bool,
}

fn done() -> Response {
    (StatusCode::OK, Json(Done { ok: true })).into_response()
}

#[derive(Deserialize)]
struct NewParty {
    names: Vec<String>,
}

async fn create_party(
    _: AdminSession,
    State(state): State<AppState>,
    Json(body): Json<NewParty>,
) -> Response {
    match data::create_party(&state.db, &body.names).await {
        Ok(_) => done(),
        Err(e) => failed(e),
    }
}

async fn delete_party(
    _: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match data::delete_party(&state.db, id).await {
        Ok(()) => done(),
        Err(e) => failed(e),
    }
}

async fn add_invitee(
    _: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<data::NewInvitee>,
) -> Response {
    match data::add_invitee(&state.db, id, &body.name).await {
        Ok(_) => done(),
        Err(e) => failed(e),
    }
}

#[derive(Deserialize)]
struct InviteeEdit {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    party_id: Option<i64>,
    /// Whether they may bring a guest. Omitted leaves the allowance alone;
    /// `false` also removes whoever was named under it.
    #[serde(default)]
    plus_one: Option<bool>,
}

async fn update_invitee(
    _: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<InviteeEdit>,
) -> Response {
    match data::update_invitee(
        &state.db,
        id,
        body.name.as_deref(),
        body.party_id,
        body.plus_one,
    )
    .await
    {
        Ok(()) => done(),
        Err(e) => failed(e),
    }
}

async fn delete_invitee(
    _: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match data::delete_invitee(&state.db, id).await {
        Ok(()) => done(),
        Err(e) => failed(e),
    }
}

#[derive(Deserialize)]
struct RsvpEdit {
    /// `null` clears the reply, which is not the same as answering "no".
    #[serde(default)]
    attending: Option<bool>,
}

async fn set_rsvp(
    _: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<RsvpEdit>,
) -> Response {
    match data::set_rsvp(&state.db, id, body.attending).await {
        Ok(()) => done(),
        Err(e) => failed(e),
    }
}

#[derive(Deserialize)]
struct PlusOneEdit {
    /// `null` clears the guest, which is how a plus one is withdrawn — the same
    /// thing an emptied box does on the RSVP card.
    #[serde(default)]
    name: Option<String>,
}

async fn set_plus_one(
    _: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<PlusOneEdit>,
) -> Response {
    match data::set_plus_one(&state.db, id, body.name.as_deref()).await {
        Ok(()) => done(),
        Err(e) => failed(e),
    }
}

#[derive(Deserialize)]
struct NoteEdit {
    #[serde(default)]
    note: Option<String>,
}

async fn set_note(
    _: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<NoteEdit>,
) -> Response {
    match data::set_note(&state.db, id, body.note.as_deref()).await {
        Ok(()) => done(),
        Err(e) => failed(e),
    }
}

// ── Errors ───────────────────────────────────────────────────────────────────

/// The wire shape for a refusal, shared with the guest-facing API so the
/// dashboard's fetch wrapper reads errors the same way the RSVP card does.
pub(crate) fn error_body(message: &str) -> ApiError {
    ApiError {
        error: message.to_owned(),
    }
}

fn error(status: StatusCode, message: String) -> Response {
    (status, Json(error_body(&message))).into_response()
}

/// Turns a data-layer failure into a response: a bad edit comes back with its
/// own wording, a database failure comes back as a generic 500 and goes to the
/// log with the detail.
fn failed(e: data::AdminError) -> Response {
    match e {
        data::AdminError::Invalid(invalid) => error(StatusCode::BAD_REQUEST, invalid.message()),
        data::AdminError::Db(e) => {
            tracing::error!("admin query failed: {e}");
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong saving that.".into(),
            )
        }
    }
}
