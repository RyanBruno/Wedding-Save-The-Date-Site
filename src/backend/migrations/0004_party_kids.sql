-- Children under 18 are not named on the invitation, so they arrive as part of
-- their family's reply rather than as replies of their own.
--
-- They hang off the party for the same reason the note does: the card asks for
-- them once per invitation, not once per person, so copying them onto every
-- member's row would only invite the copies to disagree.
--
-- The list is stored as a JSON array of {"name", "age"} objects. Nothing
-- queries inside it — it is only ever read back whole, for a head count and a
-- seating plan — so a child table would buy a join and a cascade for no gain.
CREATE TABLE IF NOT EXISTS party_kids (
    party_id   INTEGER PRIMARY KEY REFERENCES parties (id) ON DELETE CASCADE,
    kids       TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
);
