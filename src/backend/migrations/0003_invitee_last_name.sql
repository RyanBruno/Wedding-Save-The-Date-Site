-- Guests now find their invitation by typing a last name, rather than picking
-- from a menu of everyone invited. The guest list is no longer handed to whoever
-- opens the page, so this column is what the lookup matches on.
--
-- SQLite derives it rather than the seeder writing it, so there is exactly one
-- definition of "last name" and it covers rows that are already stored.
--
-- The expression is the standard last-word trick: `replace(name, ' ', '')` is
-- the set of characters to strip, `rtrim` then eats everything from the right
-- until it meets the final space, and removing that prefix leaves the last
-- word. A name with no space in it comes back whole.
--
-- `lower()` here folds ASCII only, so the handler compares against `lower(?)`
-- rather than lowercasing in Rust — both sides then fold identically.
ALTER TABLE invitees
    ADD COLUMN last_name_key TEXT
    GENERATED ALWAYS AS (
        lower(replace(name, rtrim(name, replace(name, ' ', '')), ''))
    ) VIRTUAL;

CREATE INDEX IF NOT EXISTS idx_invitees_last_name ON invitees (last_name_key);
