//! The only thing this service does at runtime: find an invitation, and accept
//! the reply to it.
//!
//! The rest of the site is generated once at start-up and served as flat files
//! from a CDN, so this module is deliberately the whole dynamic surface area.
//!
//! There is no route that lists the guest list. A caller has to know a last
//! name already on it before the API will name anybody, which is the whole
//! reason the card asks for one.

use axum::extract::State;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use shared::{
    checked_last_name, ApiError, Party, PartyLookupRequest, PartyLookupResponse, PartyMember,
    RsvpInvalid, RsvpRequest, RsvpResponse, SavedReply, PARTY_MATCH_MAX,
};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::state::AppState;

/// Routes mounted under `/api`.
///
/// Two layers wrap this, and the order each is applied in is the whole design
/// of the module.
///
/// The admin dashboard is merged in *after* `cors_layer()`: `Router::layer`
/// wraps only the routes already registered, so the two guest-facing routes get
/// the CORS headers a browser on the CDN's origin needs, and the dashboard —
/// which is only ever fetched by itself, and carries a session cookie — invites
/// no other origin to try.
///
/// `no_store()` goes on last, so it covers everything, the dashboard included.
/// See that function for why it is a layer over the whole prefix rather than a
/// header each handler remembers to set.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/lookup", post(lookup_party))
        .route("/rsvp", post(create_rsvp))
        .layer(cors_layer())
        .merge(crate::admin::router())
        .layer(no_store())
}

/// `Cache-Control: no-store` on every response under `/api`.
///
/// Everything this service answers is either a reply to one guest's question —
/// which invitation carries their name — or a page of the guest list itself.
/// None of it may be held by a shared cache, and a CDN sitting in front of the
/// site is exactly such a cache.
///
/// Applied to the whole prefix rather than to individual routes on purpose: a
/// route added later is covered without anyone having to remember it, which is
/// not true of a header set by hand in a handler. `overriding` rather than
/// `appending`, so the response carries one `Cache-Control` and not two.
///
/// nginx states the same thing again for `/api/` (see
/// `packaging/wedding-website.nginx.conf`). The duplication is deliberate: the
/// header the CDN acts on should not depend on which of the two is correct.
fn no_store() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    )
}

/// The static site is served from a different origin than this API, so the
/// browser preflights every submission.
///
/// `RSVP_ALLOWED_ORIGINS` is a comma-separated allow-list (for example
/// `https://cameronandryan.com`). It is unset in development, where any origin
/// is accepted so the generated site can be previewed from any local port.
fn cors_layer() -> CorsLayer {
    let layer = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    match std::env::var("RSVP_ALLOWED_ORIGINS") {
        Ok(raw) => {
            let origins = raw
                .split(',')
                .map(str::trim)
                .filter(|o| !o.is_empty())
                .filter_map(|o| match HeaderValue::from_str(o) {
                    Ok(value) => Some(value),
                    Err(_) => {
                        tracing::warn!("ignoring unparseable allowed origin: {o}");
                        None
                    }
                })
                .collect::<Vec<_>>();

            tracing::info!("CORS restricted to {} origin(s)", origins.len());
            layer.allow_origin(AllowOrigin::list(origins))
        }
        Err(_) => {
            tracing::warn!(
                "RSVP_ALLOWED_ORIGINS is unset — allowing any origin. Set it in production."
            );
            layer.allow_origin(AllowOrigin::any())
        }
    }
}

/// Finds the invitations carrying a given last name.
///
/// A `POST` rather than a `GET` with a query string: a lookup is a read, but
/// putting guests' names in URLs would write them into CDN access logs and
/// browser history, which is exactly what asking for a name instead of showing
/// a menu was meant to avoid.
///
/// Nothing matching is an empty list and a `200`, not an error.
async fn lookup_party(
    State(state): State<AppState>,
    Json(payload): Json<PartyLookupRequest>,
) -> Response {
    const FAILED: &str = "Could not look up your invitation.";

    let last_name = match checked_last_name(&payload.last_name) {
        Ok(name) => name,
        Err(invalid) => return error(StatusCode::BAD_REQUEST, invalid.message()),
    };

    // Two ways to match, both anchored to a whole word so a fragment cannot
    // fish for names:
    //
    //   * `last_name_key` is the final word of the name, which the schema
    //     derives (see the 0003 migration);
    //   * the `substr` comparison takes the same number of characters off the
    //     end of the name, plus the space before them, so a surname written in
    //     several words — "Van Buren" — matches too.
    //
    // `substr` with a negative start counts from the end. Deliberately not
    // `LIKE`: a guest typing `%` would otherwise match every name with a space
    // in it.
    let rows = sqlx::query(
        "SELECT id, party_id, name, plus_one
         FROM invitees
         WHERE party_id IN (
             SELECT DISTINCT party_id
             FROM invitees
             WHERE last_name_key = lower(?)
                OR lower(substr(name, -(length(?) + 1))) = lower(' ' || ?)
             ORDER BY party_id
             LIMIT ?
         )
         ORDER BY party_id, id",
    )
    .bind(last_name)
    .bind(last_name)
    .bind(last_name)
    .bind(PARTY_MATCH_MAX as i64)
    .fetch_all(&state.db)
    .await;

    let rows = match rows {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("party lookup failed: {e}");
            return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
        }
    };

    // Rows arrive grouped by party, so each change of id starts a new one.
    let mut parties: Vec<Party> = Vec::new();
    for row in rows {
        let (id, party_id, name, plus_one) = match decode_invitee(&row) {
            Some(fields) => fields,
            None => {
                tracing::error!("invitee row did not decode");
                return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
            }
        };

        if parties.last().map(|party| party.id) != Some(party_id) {
            parties.push(Party {
                id: party_id,
                members: Vec::new(),
            });
        }

        if let Some(party) = parties.last_mut() {
            party.members.push(PartyMember {
                id,
                name,
                plus_one,
            });
        }
    }

    tracing::info!("lookup matched {} invitation(s)", parties.len());

    (StatusCode::OK, Json(PartyLookupResponse { parties })).into_response()
}

/// Stores one answer per party member, replacing any earlier reply.
async fn create_rsvp(State(state): State<AppState>, Json(payload): Json<RsvpRequest>) -> Response {
    const FAILED: &str = "Could not save your reply.";

    let payload = payload.normalized();

    if let Err(invalid) = payload.validate() {
        return error(StatusCode::BAD_REQUEST, invalid.message());
    }

    // Who is on this invitation — and only if the last name the guest looked it
    // up under is still on it. That second condition is what keeps party and
    // invitee ids from being useful on their own: counting through them without
    // a matching name gets nothing back.
    let members = sqlx::query(
        "SELECT id, name, plus_one
         FROM invitees
         WHERE party_id = (
             SELECT party_id
             FROM invitees
             WHERE party_id = ?
               AND (
                   last_name_key = lower(?)
                   OR lower(substr(name, -(length(?) + 1))) = lower(' ' || ?)
               )
             LIMIT 1
         )
         ORDER BY id",
    )
    .bind(payload.party_id)
    .bind(&payload.last_name)
    .bind(&payload.last_name)
    .bind(&payload.last_name)
    .fetch_all(&state.db)
    .await;

    let members = match members {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("party lookup failed: {e}");
            return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
        }
    };

    if members.is_empty() {
        return error(StatusCode::BAD_REQUEST, RsvpInvalid::UnknownParty.message());
    }

    let mut named: Vec<(i64, String, bool)> = Vec::with_capacity(members.len());
    for row in members {
        match (
            row.try_get("id"),
            row.try_get("name"),
            row.try_get::<i64, _>("plus_one"),
        ) {
            (Ok(id), Ok(name), Ok(plus_one)) => named.push((id, name, plus_one != 0)),
            _ => {
                tracing::error!("invitee row did not decode");
                return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
            }
        }
    }

    let member_of = |id: i64| named.iter().find(|(member_id, _, _)| *member_id == id);
    let name_of = |id: i64| member_of(id).map(|(_, name, _)| name.clone());

    if payload
        .replies
        .iter()
        .any(|reply| member_of(reply.invitee_id).is_none())
    {
        return error(StatusCode::BAD_REQUEST, RsvpInvalid::NotInParty.message());
    }

    // Whether someone was invited with a guest is on their invitee row, so this
    // is the one part of a plus one the form cannot check for itself: a request
    // may name a guest for anybody, and only the list says who was offered one.
    if payload.replies.iter().any(|reply| {
        reply.plus_one_name.is_some()
            && !member_of(reply.invitee_id).is_some_and(|(_, _, plus_one)| *plus_one)
    }) {
        return error(
            StatusCode::BAD_REQUEST,
            RsvpInvalid::PlusOneNotInvited.message(),
        );
    }

    // Asked before the writes, because afterwards every member has a row and
    // the answer would always be "yes".
    let existing = sqlx::query(
        "SELECT 1 FROM rsvps
         JOIN invitees ON invitees.id = rsvps.invitee_id
         WHERE invitees.party_id = ?
         LIMIT 1",
    )
    .bind(payload.party_id)
    .fetch_optional(&state.db)
    .await;

    let updated = match existing {
        Ok(row) => row.is_some(),
        Err(e) => {
            tracing::error!("rsvp lookup failed: {e}");
            return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
        }
    };

    // One transaction so a family's answers land together or not at all —
    // half a party's reply is worse than none.
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("could not open transaction: {e}");
            return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
        }
    };

    for reply in &payload.replies {
        let written = sqlx::query(
            "INSERT INTO rsvps (invitee_id, attending)
             VALUES (?, ?)
             ON CONFLICT(invitee_id) DO UPDATE SET
                 attending  = excluded.attending,
                 updated_at = datetime('now')",
        )
        .bind(reply.invitee_id)
        .bind(reply.attending)
        .execute(&mut *tx)
        .await;

        if let Err(e) = written {
            tracing::error!("rsvp write failed: {e}");
            return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
        }

        // The name is the whole truth about the guest they are bringing, the
        // same way the note below is the whole truth about the note: a reply
        // that no longer names one withdraws the one it named before.
        let guest_written = match &reply.plus_one_name {
            Some(guest) => {
                sqlx::query(
                    "INSERT INTO plus_ones (invitee_id, name)
                     VALUES (?, ?)
                     ON CONFLICT(invitee_id) DO UPDATE SET
                         name       = excluded.name,
                         updated_at = datetime('now')",
                )
                .bind(reply.invitee_id)
                .bind(guest)
                .execute(&mut *tx)
                .await
            }
            None => {
                sqlx::query("DELETE FROM plus_ones WHERE invitee_id = ?")
                    .bind(reply.invitee_id)
                    .execute(&mut *tx)
                    .await
            }
        };

        if let Err(e) = guest_written {
            tracing::error!("plus-one write failed: {e}");
            return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
        }
    }

    // The form carries one note for the party, and it is the whole truth about
    // that note: clearing the box clears what we stored.
    let note_written = match &payload.note {
        Some(note) => {
            sqlx::query(
                "INSERT INTO party_notes (party_id, note)
                 VALUES (?, ?)
                 ON CONFLICT(party_id) DO UPDATE SET
                     note       = excluded.note,
                     updated_at = datetime('now')",
            )
            .bind(payload.party_id)
            .bind(note)
            .execute(&mut *tx)
            .await
        }
        None => {
            sqlx::query("DELETE FROM party_notes WHERE party_id = ?")
                .bind(payload.party_id)
                .execute(&mut *tx)
                .await
        }
    };

    if let Err(e) = note_written {
        tracing::error!("note write failed: {e}");
        return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("rsvp commit failed: {e}");
        return error(StatusCode::INTERNAL_SERVER_ERROR, FAILED);
    }

    let replies: Vec<SavedReply> = payload
        .replies
        .iter()
        .map(|reply| SavedReply {
            invitee_id: reply.invitee_id,
            name: name_of(reply.invitee_id).unwrap_or_default(),
            attending: reply.attending,
            plus_one: reply.plus_one_name.clone(),
        })
        .collect();

    tracing::info!(
        "rsvp {} for party {} ({} of {} attending, {} guest(s) named)",
        if updated { "updated" } else { "recorded" },
        payload.party_id,
        replies.iter().filter(|r| r.attending).count(),
        replies.len(),
        replies.iter().filter(|r| r.plus_one.is_some()).count(),
    );

    (
        StatusCode::OK,
        Json(RsvpResponse {
            party_id: payload.party_id,
            updated,
            replies,
        }),
    )
        .into_response()
}

/// `plus_one` is an INTEGER column, read as one and compared here rather than
/// decoded straight to `bool`, which is how the dashboard reads it too.
fn decode_invitee(row: &SqliteRow) -> Option<(i64, i64, String, bool)> {
    match (
        row.try_get("id"),
        row.try_get("party_id"),
        row.try_get("name"),
        row.try_get::<i64, _>("plus_one"),
    ) {
        (Ok(id), Ok(party_id), Ok(name), Ok(plus_one)) => {
            Some((id, party_id, name, plus_one != 0))
        }
        _ => None,
    }
}

fn error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(ApiError {
            error: message.to_owned(),
        }),
    )
        .into_response()
}
