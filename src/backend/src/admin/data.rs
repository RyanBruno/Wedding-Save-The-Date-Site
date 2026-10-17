//! Everything the dashboard reads and writes, kept apart from the HTTP layer.
//!
//! The guest-facing handlers in [`crate::api`] go out of their way never to
//! return a name that was not asked for by surname. Nothing in this module does
//! that, on purpose: the whole point of the dashboard is the list. The check
//! that keeps them apart is the session cookie, not the query.

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// A guest list is dozens of rows, so the dashboard fetches the whole thing in
/// one request and filters in the browser. Past this, it should page instead —
/// and something has gone wrong with the wedding.
pub const OVERVIEW_MAX_PARTIES: i64 = 2_000;

/// One invitation, with everyone on it and everything they have told us.
#[derive(Debug, Clone, Serialize)]
pub struct AdminParty {
    pub id: i64,
    /// The invitation's label — the members' names joined. Derived, never typed.
    pub name: String,
    pub created_at: String,
    pub members: Vec<AdminInvitee>,
    pub note: Option<String>,
    pub note_updated_at: Option<String>,
    /// Most recent reply from anyone on the invitation.
    pub last_reply_at: Option<String>,
}

/// One person on an invitation, and their answer if they have given one.
#[derive(Debug, Clone, Serialize)]
pub struct AdminInvitee {
    pub id: i64,
    pub name: String,
    /// `None` means nobody has answered for them yet — which is different from
    /// an answer of "not coming", and the dashboard shows them differently.
    pub attending: Option<bool>,
    pub replied_at: Option<String>,
    /// Whether they were invited with a guest of their own choosing.
    pub plus_one: bool,
    /// The guest they are bringing, once they have named one. `None` with
    /// `plus_one` set is an allowance nobody has used yet.
    pub plus_one_name: Option<String>,
}

/// A name being added to the list. A trailing `+1` gives them a guest, the same
/// marker the seeding script reads from a CSV field.
#[derive(Debug, Clone, Deserialize)]
pub struct NewInvitee {
    pub name: String,
}

/// Why a write was refused. Everything here is something the person at the
/// keyboard can fix, so each one carries wording meant to be shown verbatim.
#[derive(Debug)]
pub enum AdminInvalid {
    NameEmpty,
    NameTooLong,
    NameTaken(String),
    NoMembers,
    PartyTooLarge,
    UnknownParty,
    UnknownInvitee,
    NoteTooLong,
    PlusOneNotInvited(String),
}

impl AdminInvalid {
    pub fn message(&self) -> String {
        match self {
            Self::NameEmpty => "Please give the guest's full name.".into(),
            Self::NameTooLong => {
                format!("That name is longer than {} characters.", shared::NAME_MAX_LEN)
            }
            Self::NameTaken(name) => format!(
                "{name} is already on the guest list. A guest belongs to one invitation, so \
                 rename them or move the existing entry instead."
            ),
            Self::NoMembers => "An invitation needs at least one guest on it.".into(),
            Self::PartyTooLarge => format!(
                "One invitation holds at most {} guests.",
                shared::PARTY_MAX_MEMBERS
            ),
            Self::UnknownParty => "That invitation no longer exists.".into(),
            Self::UnknownInvitee => "That guest no longer exists.".into(),
            Self::NoteTooLong => {
                format!("That note is longer than {} characters.", shared::NOTE_MAX_LEN)
            }
            Self::PlusOneNotInvited(name) => format!(
                "{name} was not invited with a guest, so there is nobody to name. Give them a \
                 plus one first."
            ),
        }
    }
}

/// Either a bad request from the dashboard, or the database failing.
#[derive(Debug)]
pub enum AdminError {
    Invalid(AdminInvalid),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for AdminError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

impl From<AdminInvalid> for AdminError {
    fn from(e: AdminInvalid) -> Self {
        Self::Invalid(e)
    }
}

type Result<T> = std::result::Result<T, AdminError>;

// ── Reads ────────────────────────────────────────────────────────────────────

/// The whole board: every invitation, its guests, their answers and the note.
///
/// Three flat queries stitched together rather than one join, because the note
/// hangs off the party and a join would repeat it once per guest — then the
/// assembling code has to un-repeat it anyway.
pub async fn overview(db: &SqlitePool) -> Result<Vec<AdminParty>> {
    let party_rows = sqlx::query(
        "SELECT id, name, created_at
         FROM parties
         ORDER BY id
         LIMIT ?",
    )
    .bind(OVERVIEW_MAX_PARTIES)
    .fetch_all(db)
    .await?;

    let mut parties: Vec<AdminParty> = Vec::with_capacity(party_rows.len());
    for row in party_rows {
        parties.push(AdminParty {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            created_at: row.try_get("created_at")?,
            members: Vec::new(),
            note: None,
            note_updated_at: None,
            last_reply_at: None,
        });
    }

    // Position of each party in the vector, so the two passes below can drop
    // their rows straight onto the right one.
    let index: std::collections::HashMap<i64, usize> = parties
        .iter()
        .enumerate()
        .map(|(at, party)| (party.id, at))
        .collect();

    // The plus one joins in here rather than being fetched on its own: it hangs
    // off the invitee one-to-one, so it repeats nothing the way a party's note
    // would.
    let invitee_rows = sqlx::query(
        "SELECT i.id, i.party_id, i.name, i.plus_one,
                r.attending, r.updated_at AS replied_at,
                g.name AS plus_one_name
         FROM invitees i
         LEFT JOIN rsvps r ON r.invitee_id = i.id
         LEFT JOIN plus_ones g ON g.invitee_id = i.id
         ORDER BY i.party_id, i.id",
    )
    .fetch_all(db)
    .await?;

    for row in invitee_rows {
        let party_id: i64 = row.try_get("party_id")?;
        let Some(&at) = index.get(&party_id) else {
            continue;
        };

        // `attending` is NULL for a guest with no row in `rsvps` — the LEFT
        // JOIN's doing, and exactly the distinction the dashboard needs.
        let attending: Option<bool> = row.try_get::<Option<i64>, _>("attending")?.map(|a| a != 0);
        let replied_at: Option<String> = row.try_get("replied_at")?;

        // Timestamps are `datetime('now')` strings, so lexical order is
        // chronological order and no parsing is needed to find the latest.
        if let Some(latest) = replied_at.as_deref() {
            let party = &mut parties[at];
            if party.last_reply_at.as_deref().is_none_or(|seen| seen < latest) {
                party.last_reply_at = Some(latest.to_owned());
            }
        }

        parties[at].members.push(AdminInvitee {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            attending,
            replied_at,
            plus_one: row.try_get::<i64, _>("plus_one")? != 0,
            plus_one_name: row.try_get("plus_one_name")?,
        });
    }

    let note_rows = sqlx::query("SELECT party_id, note, updated_at FROM party_notes")
        .fetch_all(db)
        .await?;

    for row in note_rows {
        let party_id: i64 = row.try_get("party_id")?;
        if let Some(&at) = index.get(&party_id) {
            parties[at].note = row.try_get("note")?;
            parties[at].note_updated_at = row.try_get("updated_at")?;
        }
    }

    Ok(parties)
}

// ── Writes ───────────────────────────────────────────────────────────────────

/// Creates an invitation from a list of names — the same unit the seeding
/// script works in, so the two cannot drift apart.
pub async fn create_party(db: &SqlitePool, names: &[String]) -> Result<i64> {
    let guests = clean_names(names)?;
    let labels: Vec<String> = guests.iter().map(|(name, _)| name.clone()).collect();

    let mut tx = db.begin().await?;

    for (name, _) in &guests {
        if let Some(taken) = find_name(&mut tx, name).await? {
            return Err(AdminInvalid::NameTaken(taken).into());
        }
    }

    let party_id: i64 = sqlx::query("INSERT INTO parties (name) VALUES (?) RETURNING id")
        .bind(party_label(&labels))
        .fetch_one(&mut *tx)
        .await?
        .try_get("id")?;

    for (name, plus_one) in &guests {
        insert_invitee(&mut tx, party_id, name, *plus_one).await?;
    }

    tx.commit().await?;
    tracing::info!("admin created invitation {party_id} with {} guest(s)", guests.len());
    Ok(party_id)
}

/// Adds one guest to an existing invitation.
pub async fn add_invitee(db: &SqlitePool, party_id: i64, raw: &str) -> Result<i64> {
    let (raw, plus_one) = split_plus_one(raw);
    let name = clean_name(raw)?;

    let mut tx = db.begin().await?;

    if !party_exists(&mut tx, party_id).await? {
        return Err(AdminInvalid::UnknownParty.into());
    }
    if let Some(taken) = find_name(&mut tx, &name).await? {
        return Err(AdminInvalid::NameTaken(taken).into());
    }

    let existing: i64 = sqlx::query("SELECT count(*) AS n FROM invitees WHERE party_id = ?")
        .bind(party_id)
        .fetch_one(&mut *tx)
        .await?
        .try_get("n")?;

    if existing as usize >= shared::PARTY_MAX_MEMBERS {
        return Err(AdminInvalid::PartyTooLarge.into());
    }

    let id = insert_invitee(&mut tx, party_id, &name, plus_one).await?;
    relabel_party(&mut tx, party_id).await?;

    tx.commit().await?;
    tracing::info!("admin added {name} to invitation {party_id}");
    Ok(id)
}

/// Renames a guest, moves them to another invitation, or both.
///
/// Moving is a real operation here rather than a hand-written UPDATE, because
/// it changes who answers for whom — and doing it in the open, with the reply
/// carried along, beats doing it in `sqlite3` at midnight.
pub async fn update_invitee(
    db: &SqlitePool,
    invitee_id: i64,
    new_name: Option<&str>,
    new_party: Option<i64>,
    new_plus_one: Option<bool>,
) -> Result<()> {
    let mut tx = db.begin().await?;

    let current = sqlx::query("SELECT party_id, name FROM invitees WHERE id = ?")
        .bind(invitee_id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(current) = current else {
        return Err(AdminInvalid::UnknownInvitee.into());
    };
    let old_party: i64 = current.try_get("party_id")?;
    let old_name: String = current.try_get("name")?;

    if let Some(raw) = new_name {
        let name = clean_name(raw)?;
        // A guest keeping their own name is not a clash with themselves.
        let clash = sqlx::query("SELECT name FROM invitees WHERE name_key = lower(?) AND id <> ?")
            .bind(&name)
            .bind(invitee_id)
            .fetch_optional(&mut *tx)
            .await?;

        if let Some(row) = clash {
            return Err(AdminInvalid::NameTaken(row.try_get("name")?).into());
        }

        sqlx::query("UPDATE invitees SET name = ?, name_key = lower(?) WHERE id = ?")
            .bind(&name)
            .bind(&name)
            .bind(invitee_id)
            .execute(&mut *tx)
            .await?;
    }

    if let Some(plus_one) = new_plus_one {
        sqlx::query("UPDATE invitees SET plus_one = ? WHERE id = ?")
            .bind(plus_one)
            .bind(invitee_id)
            .execute(&mut *tx)
            .await?;

        // Taking the allowance away takes the guest with it. A name left behind
        // would go on being counted for an invitation that no longer carries
        // anybody, and nothing on the card could clear it.
        if !plus_one {
            sqlx::query("DELETE FROM plus_ones WHERE invitee_id = ?")
                .bind(invitee_id)
                .execute(&mut *tx)
                .await?;
        }

        tracing::info!("admin set plus-one allowance for {old_name} to {plus_one}");
    }

    if let Some(party_id) = new_party.filter(|id| *id != old_party) {
        if !party_exists(&mut tx, party_id).await? {
            return Err(AdminInvalid::UnknownParty.into());
        }

        let existing: i64 = sqlx::query("SELECT count(*) AS n FROM invitees WHERE party_id = ?")
            .bind(party_id)
            .fetch_one(&mut *tx)
            .await?
            .try_get("n")?;

        if existing as usize >= shared::PARTY_MAX_MEMBERS {
            return Err(AdminInvalid::PartyTooLarge.into());
        }

        sqlx::query("UPDATE invitees SET party_id = ? WHERE id = ?")
            .bind(party_id)
            .bind(invitee_id)
            .execute(&mut *tx)
            .await?;

        relabel_party(&mut tx, party_id).await?;
        drop_party_if_empty(&mut tx, old_party).await?;
        tracing::info!("admin moved {old_name} from invitation {old_party} to {party_id}");
    }

    relabel_party(&mut tx, old_party).await?;
    tx.commit().await?;
    Ok(())
}

/// Removes a guest. Their reply goes with them — the schema cascades — and an
/// invitation left with nobody on it is removed too, the same tidy-up the
/// seeding script does.
pub async fn delete_invitee(db: &SqlitePool, invitee_id: i64) -> Result<()> {
    let mut tx = db.begin().await?;

    let row = sqlx::query("SELECT party_id, name FROM invitees WHERE id = ?")
        .bind(invitee_id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(row) = row else {
        return Err(AdminInvalid::UnknownInvitee.into());
    };
    let party_id: i64 = row.try_get("party_id")?;
    let name: String = row.try_get("name")?;

    sqlx::query("DELETE FROM invitees WHERE id = ?")
        .bind(invitee_id)
        .execute(&mut *tx)
        .await?;

    if !drop_party_if_empty(&mut tx, party_id).await? {
        relabel_party(&mut tx, party_id).await?;
    }

    tx.commit().await?;
    tracing::info!("admin removed {name} from invitation {party_id}");
    Ok(())
}

/// Removes a whole invitation: its guests, their replies and the note all
/// cascade from the schema.
pub async fn delete_party(db: &SqlitePool, party_id: i64) -> Result<()> {
    let deleted = sqlx::query("DELETE FROM parties WHERE id = ?")
        .bind(party_id)
        .execute(db)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(AdminInvalid::UnknownParty.into());
    }

    tracing::info!("admin deleted invitation {party_id}");
    Ok(())
}

/// Records an answer on a guest's behalf, or clears it.
///
/// `None` deletes the reply rather than storing a third state: "has not
/// answered" is the absence of a row everywhere else in the schema, and the
/// RSVP card counts on that.
pub async fn set_rsvp(db: &SqlitePool, invitee_id: i64, attending: Option<bool>) -> Result<()> {
    let exists = sqlx::query("SELECT 1 FROM invitees WHERE id = ?")
        .bind(invitee_id)
        .fetch_optional(db)
        .await?;

    if exists.is_none() {
        return Err(AdminInvalid::UnknownInvitee.into());
    }

    match attending {
        Some(attending) => {
            sqlx::query(
                "INSERT INTO rsvps (invitee_id, attending)
                 VALUES (?, ?)
                 ON CONFLICT(invitee_id) DO UPDATE SET
                     attending  = excluded.attending,
                     updated_at = datetime('now')",
            )
            .bind(invitee_id)
            .bind(attending)
            .execute(db)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM rsvps WHERE invitee_id = ?")
                .bind(invitee_id)
                .execute(db)
                .await?;
        }
    }

    tracing::info!("admin set rsvp for invitee {invitee_id} to {attending:?}");
    Ok(())
}

/// Names the guest an invitee is bringing, or clears them.
///
/// `None` deletes the row rather than storing an empty name, because a plus one
/// who is coming is a plus one with a name — the same rule the RSVP card keeps,
/// where emptying the box withdraws the guest.
pub async fn set_plus_one(db: &SqlitePool, invitee_id: i64, name: Option<&str>) -> Result<()> {
    let name = match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(raw) => Some(clean_name(raw)?),
        None => None,
    };

    let row = sqlx::query("SELECT name, plus_one FROM invitees WHERE id = ?")
        .bind(invitee_id)
        .fetch_optional(db)
        .await?;

    let Some(row) = row else {
        return Err(AdminInvalid::UnknownInvitee.into());
    };

    // Naming a guest for somebody who was never offered one would record a head
    // the couple did not invite, so the allowance has to be given first.
    if name.is_some() && row.try_get::<i64, _>("plus_one")? == 0 {
        return Err(AdminInvalid::PlusOneNotInvited(row.try_get("name")?).into());
    }

    match &name {
        Some(name) => {
            sqlx::query(
                "INSERT INTO plus_ones (invitee_id, name)
                 VALUES (?, ?)
                 ON CONFLICT(invitee_id) DO UPDATE SET
                     name       = excluded.name,
                     updated_at = datetime('now')",
            )
            .bind(invitee_id)
            .bind(name)
            .execute(db)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM plus_ones WHERE invitee_id = ?")
                .bind(invitee_id)
                .execute(db)
                .await?;
        }
    }

    tracing::info!("admin set the guest of invitee {invitee_id} to {name:?}");
    Ok(())
}

/// Sets or clears an invitation's note. An empty note clears the stored one,
/// matching what the guest-facing form does with a blank box.
pub async fn set_note(db: &SqlitePool, party_id: i64, note: Option<&str>) -> Result<()> {
    let note = note.map(str::trim).filter(|n| !n.is_empty());

    if let Some(note) = note {
        if note.chars().count() > shared::NOTE_MAX_LEN {
            return Err(AdminInvalid::NoteTooLong.into());
        }
    }

    let mut tx = db.begin().await?;

    if !party_exists(&mut tx, party_id).await? {
        return Err(AdminInvalid::UnknownParty.into());
    }

    match note {
        Some(note) => {
            sqlx::query(
                "INSERT INTO party_notes (party_id, note)
                 VALUES (?, ?)
                 ON CONFLICT(party_id) DO UPDATE SET
                     note       = excluded.note,
                     updated_at = datetime('now')",
            )
            .bind(party_id)
            .bind(note)
            .execute(&mut *tx)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM party_notes WHERE party_id = ?")
                .bind(party_id)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

// ── Shared helpers ───────────────────────────────────────────────────────────

type Tx<'a> = sqlx::Transaction<'a, sqlx::Sqlite>;

async fn party_exists(tx: &mut Tx<'_>, party_id: i64) -> Result<bool> {
    Ok(sqlx::query("SELECT 1 FROM parties WHERE id = ?")
        .bind(party_id)
        .fetch_optional(&mut **tx)
        .await?
        .is_some())
}

/// The name already on the list that collides with `name`, if any. Returned
/// rather than a bool so the message can say who.
async fn find_name(tx: &mut Tx<'_>, name: &str) -> Result<Option<String>> {
    let row = sqlx::query("SELECT name FROM invitees WHERE name_key = lower(?)")
        .bind(name)
        .fetch_optional(&mut **tx)
        .await?;

    match row {
        Some(row) => Ok(Some(row.try_get("name")?)),
        None => Ok(None),
    }
}

/// `name_key` is written here, not derived, because the 0002 schema made it a
/// plain unique column. `last_name_key` is generated and needs nothing.
async fn insert_invitee(
    tx: &mut Tx<'_>,
    party_id: i64,
    name: &str,
    plus_one: bool,
) -> Result<i64> {
    let id: i64 = sqlx::query(
        "INSERT INTO invitees (party_id, name, name_key, plus_one)
         VALUES (?, ?, lower(?), ?)
         RETURNING id",
    )
    .bind(party_id)
    .bind(name)
    .bind(name)
    .bind(plus_one)
    .fetch_one(&mut **tx)
    .await?
    .try_get("id")?;

    Ok(id)
}

/// Rewrites an invitation's label from whoever is on it now. Nothing shows this
/// to a guest — it is what the dashboard and the seeder both call an invitation
/// — so it is regenerated rather than edited.
async fn relabel_party(tx: &mut Tx<'_>, party_id: i64) -> Result<()> {
    let rows = sqlx::query("SELECT name FROM invitees WHERE party_id = ? ORDER BY id")
        .bind(party_id)
        .fetch_all(&mut **tx)
        .await?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut names = Vec::with_capacity(rows.len());
    for row in rows {
        names.push(row.try_get::<String, _>("name")?);
    }

    sqlx::query("UPDATE parties SET name = ? WHERE id = ?")
        .bind(party_label(&names))
        .bind(party_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

/// Drops an invitation nobody is on any more. Returns whether it went.
async fn drop_party_if_empty(tx: &mut Tx<'_>, party_id: i64) -> Result<bool> {
    let remaining: i64 = sqlx::query("SELECT count(*) AS n FROM invitees WHERE party_id = ?")
        .bind(party_id)
        .fetch_one(&mut **tx)
        .await?
        .try_get("n")?;

    if remaining > 0 {
        return Ok(false);
    }

    sqlx::query("DELETE FROM parties WHERE id = ?")
        .bind(party_id)
        .execute(&mut **tx)
        .await?;

    tracing::info!("invitation {party_id} removed — nobody left on it");
    Ok(true)
}

/// How the seeding script names a party, kept identical so an invitation made
/// here is indistinguishable from one loaded from the CSV.
fn party_label(names: &[String]) -> String {
    names.join(" & ")
}

/// Splits a trailing `+1` off a typed name: "Alex Moreno +1" is Alex Moreno,
/// invited with a guest.
///
/// The same marker the seeding script reads from a CSV field, so an invitation
/// typed into the dashboard and one loaded from the file carry the allowance the
/// same way and neither has to be edited afterwards.
fn split_plus_one(raw: &str) -> (&str, bool) {
    match raw.trim_end().strip_suffix("+1") {
        Some(name) => (name, true),
        None => (raw, false),
    }
}

fn clean_name(raw: &str) -> std::result::Result<String, AdminInvalid> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");

    if name.is_empty() {
        return Err(AdminInvalid::NameEmpty);
    }
    if name.chars().count() > shared::NAME_MAX_LEN {
        return Err(AdminInvalid::NameTooLong);
    }
    Ok(name)
}

/// Each cleaned name, paired with whether it was written with a `+1`.
fn clean_names(raw: &[String]) -> std::result::Result<Vec<(String, bool)>, AdminInvalid> {
    let mut guests = Vec::with_capacity(raw.len());
    for entry in raw {
        if entry.trim().is_empty() {
            continue;
        }
        let (name, plus_one) = split_plus_one(entry);
        guests.push((clean_name(name)?, plus_one));
    }

    if guests.is_empty() {
        return Err(AdminInvalid::NoMembers);
    }
    if guests.len() > shared::PARTY_MAX_MEMBERS {
        return Err(AdminInvalid::PartyTooLarge);
    }

    // Two people with the same name on one invitation would collide on
    // `name_key` halfway through the insert; caught here so the message names
    // the duplicate instead of surfacing a constraint error.
    for (i, (name, _)) in guests.iter().enumerate() {
        if guests[..i]
            .iter()
            .any(|(other, _)| other.eq_ignore_ascii_case(name))
        {
            return Err(AdminInvalid::NameTaken(name.clone()));
        }
    }

    Ok(guests)
}
