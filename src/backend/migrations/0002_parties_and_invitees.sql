-- The guest list moves into the database so the RSVP card can offer names
-- instead of a free-text box.
--
-- `parties` is one invitation; `invitees` are the people named on it. A reply
-- now points at an invitee rather than carrying a typed name, which is what
-- lets one selection open an answer for every member of a couple or family.

CREATE TABLE IF NOT EXISTS parties (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS invitees (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    party_id   INTEGER NOT NULL REFERENCES parties (id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    -- Case-folded, and unique across the whole list: a person belongs to one
    -- party, so the same name cannot appear on two invitations.
    name_key   TEXT    NOT NULL UNIQUE,
    created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_invitees_party ON invitees (party_id);

-- Anyone who replied under the old free-text form becomes a party of one, so
-- their reply still points at a real invitee afterwards.
INSERT INTO parties (id, name)
SELECT id, name FROM rsvps;

INSERT INTO invitees (id, party_id, name, name_key)
SELECT id, id, name, name_key FROM rsvps;

-- The card asks for one note, not one per person, so the note belongs to the
-- party rather than being copied onto every member's row.
CREATE TABLE IF NOT EXISTS party_notes (
    party_id   INTEGER PRIMARY KEY REFERENCES parties (id) ON DELETE CASCADE,
    note       TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO party_notes (party_id, note, created_at, updated_at)
SELECT id, note, created_at, updated_at
FROM rsvps
WHERE note IS NOT NULL AND trim(note) <> '';

-- SQLite cannot add a UNIQUE column or drop one in place, so the reply table is
-- rebuilt around `invitee_id` and the old rows are carried over by id.
CREATE TABLE rsvps_rebuilt (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    invitee_id INTEGER NOT NULL UNIQUE REFERENCES invitees (id) ON DELETE CASCADE,
    attending  INTEGER NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO rsvps_rebuilt (id, invitee_id, attending, created_at, updated_at)
SELECT id, id, attending, created_at, updated_at FROM rsvps;

DROP TABLE rsvps;

ALTER TABLE rsvps_rebuilt RENAME TO rsvps;

CREATE INDEX IF NOT EXISTS idx_rsvps_attending ON rsvps (attending);
