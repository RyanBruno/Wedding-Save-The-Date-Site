-- Some invitations carry a plus one: a guest the couple has not named, because
-- at the time the envelopes went out nobody knew who it would be.
--
-- Two separate facts, so two separate places:
--
--   * `invitees.plus_one` is the *allowance* — this person may bring someone.
--     It belongs on the invitee rather than the party, because "Alex and a
--     guest" and "Dana, Sam, and a guest for Sam" are both ordinary cases and
--     only the invitee knows which of them it is.
--
--   * `plus_ones` is the *answer* — who they are actually bringing. A row here
--     means someone is coming and this is their name; no row means nobody is.
--     Absence is the same "has not / will not" the rest of the schema uses for
--     an unanswered `rsvps` row and a cleared note, so nothing has to store a
--     third state or reconcile a name against a flag that disagrees with it.
--
-- The plus one is deliberately not an `invitees` row of its own. That table's
-- `name_key` is NOT NULL and unique and its `last_name_key` drives the lookup,
-- so an unnamed placeholder would need a fake name to satisfy the index, and
-- would then be findable by surname and answerable for in its own right.
ALTER TABLE invitees
    ADD COLUMN plus_one INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS plus_ones (
    invitee_id INTEGER PRIMARY KEY REFERENCES invitees (id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
);
