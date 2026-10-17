//! Types shared by the static frontend and the RSVP API.
//!
//! This crate is compiled for both `wasm32` and the host, so it must stay free
//! of anything platform-specific: it carries the wire format and the validation
//! rules, nothing else. Keeping the limits here means the form and the handler
//! can never disagree about what counts as a valid reply.

use serde::{Deserialize, Serialize};

/// Longest guest-list name we will store. Generous for a double-barrelled name.
pub const NAME_MAX_LEN: usize = 120;

/// Longest note we will store.
pub const NOTE_MAX_LEN: usize = 1000;

/// Most people we will accept answers for in one submission. Well above the
/// largest family on the list; it exists to bound a hand-crafted request.
pub const PARTY_MAX_MEMBERS: usize = 12;

/// Shortest last name we will look up.
///
/// Not a real limit on names — nobody is called "Ng" less than two letters —
/// but it stops a single character from pulling back a slice of the guest list.
pub const LAST_NAME_MIN_LEN: usize = 2;

/// Most invitations one last name may return. A common surname should offer a
/// choice, not read out the guest list.
pub const PARTY_MATCH_MAX: usize = 8;

/// Asking which invitation a guest is on. Sent as a body rather than a query
/// string so guests' names stay out of URLs, access logs and browser history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyLookupRequest {
    pub last_name: String,
}

/// One person on an invitation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyMember {
    pub id: i64,
    pub name: String,
    /// Whether this person was invited with a guest of their own choosing. The
    /// guest has no name until the reply gives them one, which is the whole
    /// reason the allowance is a flag here rather than another member.
    #[serde(default)]
    pub plus_one: bool,
}

/// Everyone named on one invitation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Party {
    pub id: i64,
    pub members: Vec<PartyMember>,
}

impl Party {
    /// How an invitation is shown to the guest who looked it up: the names on
    /// it, in order, comma-separated.
    ///
    /// Anyone invited with a plus one reads as "Alex Moreno + guest", which is
    /// how the envelope was addressed and how the guest recognises the
    /// invitation as theirs.
    pub fn label(&self) -> String {
        self.members
            .iter()
            .map(|member| {
                if member.plus_one {
                    format!("{} + guest", member.name)
                } else {
                    member.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// What a lookup returns. Empty when nothing matched — that is an answer, not
/// an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyLookupResponse {
    pub parties: Vec<Party>,
}

/// One person's answer within a party's reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RsvpReply {
    pub invitee_id: i64,
    pub attending: bool,
    /// Who they are bringing, for the invitations that carry a plus one.
    ///
    /// `None` is "nobody" — either they were never offered a guest, or they
    /// were and are coming alone. There is no third state: a named guest is a
    /// guest who is coming, so clearing the name is how one is withdrawn.
    #[serde(default)]
    pub plus_one_name: Option<String>,
}

/// A reply submitted from the RSVP form: one party, one answer per member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RsvpRequest {
    pub party_id: i64,
    /// The last name the guest looked their invitation up under, carried back
    /// so the handler can check it still belongs to this party.
    ///
    /// Without it, party and invitee ids are small integers that anyone could
    /// count through — which would both overwrite strangers' replies and hand
    /// back their names, straight past the lookup this field exists to enforce.
    pub last_name: String,
    pub replies: Vec<RsvpReply>,
    /// Optional message to the couple, sent once for the party. Empty notes
    /// arrive as `None`.
    #[serde(default)]
    pub note: Option<String>,
}

/// One stored answer, echoed back with the name it was stored against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedReply {
    pub invitee_id: i64,
    pub name: String,
    pub attending: bool,
    /// The guest stored alongside this answer, if one was named.
    pub plus_one: Option<String>,
}

/// What the API returns once a party's reply has been stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RsvpResponse {
    pub party_id: i64,
    /// True when this party had already replied and these answers replaced it.
    pub updated: bool,
    pub replies: Vec<SavedReply>,
}

impl RsvpResponse {
    /// Whether anyone in the party is coming, which is all the card needs to
    /// pick its thank-you wording.
    pub fn any_attending(&self) -> bool {
        self.replies.iter().any(|r| r.attending)
    }
}

/// Error body returned for any non-2xx response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

/// Why a submitted reply was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsvpInvalid {
    LastNameTooShort,
    LastNameTooLong,
    NoReplies,
    PartyTooLarge,
    DuplicateReply,
    NoteTooLong,
    UnknownParty,
    NotInParty,
    PlusOneNameTooLong,
    PlusOneNotInvited,
    PlusOneNotAttending,
}

impl RsvpInvalid {
    pub fn message(self) -> &'static str {
        match self {
            Self::LastNameTooShort => "Please enter the last name of someone in your party.",
            Self::LastNameTooLong => "That name is too long.",
            Self::NoReplies => "Please let us know who is replying.",
            Self::PartyTooLarge => "That is more guests than one reply can carry.",
            Self::DuplicateReply => "Each guest can only be answered for once.",
            Self::NoteTooLong => "That note is too long.",
            Self::UnknownParty => "We couldn't find that invitation.",
            Self::NotInParty => "That guest isn't on the same invitation.",
            Self::PlusOneNameTooLong => "Your guest's name is too long.",
            Self::PlusOneNotInvited => "That invitation doesn't include a guest.",
            Self::PlusOneNotAttending => {
                "Please only name a guest for someone who is attending."
            }
        }
    }
}

/// Trims a typed last name and checks it is worth looking up.
///
/// Shared so the card can stay quiet until there is something to search for,
/// and the handler can reject the same input if the request arrives by hand.
pub fn checked_last_name(raw: &str) -> Result<&str, RsvpInvalid> {
    let name = raw.trim();
    let length = name.chars().count();

    if length < LAST_NAME_MIN_LEN {
        return Err(RsvpInvalid::LastNameTooShort);
    }
    if length > NAME_MAX_LEN {
        return Err(RsvpInvalid::LastNameTooLong);
    }
    Ok(name)
}

impl RsvpRequest {
    /// Trims the last name, the note, and each guest's name, dropping a note or
    /// a guest name that is only whitespace.
    ///
    /// A blank guest name becoming `None` is deliberate: an empty box means
    /// nobody is coming, which is the same answer as never having been offered
    /// a guest at all.
    ///
    /// Run before [`validate`](Self::validate).
    pub fn normalized(mut self) -> Self {
        self.last_name = self.last_name.trim().to_owned();
        self.note = self
            .note
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty());
        for reply in &mut self.replies {
            reply.plus_one_name = reply
                .plus_one_name
                .take()
                .map(|n| n.trim().to_owned())
                .filter(|n| !n.is_empty());
        }
        self
    }

    /// Checks the shape of the reply: that it names an invitation, answers for
    /// someone, answers for each person once, brings a guest only where that
    /// makes sense, and carries a note we will accept.
    ///
    /// Whether those people are really on that invitation — and whether they
    /// were offered a guest at all — are questions only the database can
    /// answer, so the handler checks those separately.
    pub fn validate(&self) -> Result<(), RsvpInvalid> {
        checked_last_name(&self.last_name)?;

        if self.replies.is_empty() {
            return Err(RsvpInvalid::NoReplies);
        }
        if self.replies.len() > PARTY_MAX_MEMBERS {
            return Err(RsvpInvalid::PartyTooLarge);
        }
        // Quadratic, but bounded by PARTY_MAX_MEMBERS above.
        for (i, reply) in self.replies.iter().enumerate() {
            if self.replies[..i]
                .iter()
                .any(|other| other.invitee_id == reply.invitee_id)
            {
                return Err(RsvpInvalid::DuplicateReply);
            }

            if let Some(guest) = &reply.plus_one_name {
                // A guest of someone who is not coming has nobody to come
                // with, so this is a mistake rather than an extra head.
                if !reply.attending {
                    return Err(RsvpInvalid::PlusOneNotAttending);
                }
                if guest.chars().count() > NAME_MAX_LEN {
                    return Err(RsvpInvalid::PlusOneNameTooLong);
                }
            }
        }
        if let Some(note) = &self.note {
            if note.chars().count() > NOTE_MAX_LEN {
                return Err(RsvpInvalid::NoteTooLong);
            }
        }
        Ok(())
    }
}
