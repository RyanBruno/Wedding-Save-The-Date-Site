-- One row per guest. `name_key` is the case-folded name and carries the unique
-- index, so a guest who replies a second time overwrites their first reply
-- rather than leaving two conflicting rows behind.
CREATE TABLE IF NOT EXISTS rsvps (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    name_key   TEXT    NOT NULL UNIQUE,
    attending  INTEGER NOT NULL,
    note       TEXT,
    created_at TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_rsvps_attending ON rsvps (attending);
