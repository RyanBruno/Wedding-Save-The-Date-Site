#!/usr/bin/env bash
#
# Loads the guest list into the database. That list is where the RSVP card's
# name menu comes from, so until this runs the menu is empty.
#
# One line per invitation, names comma-separated. Everyone on a line forms one
# party and is asked about together on the card:
#
#     Dana Whitfield, Sam Whitfield
#     Alex Moreno +1
#     # lines starting with # are ignored
#
# A name ending in "+1" is invited with a guest of their own choosing. The guest
# has no name here: the card collects it if that person replies that they are
# bringing somebody. The marker is an allowance, not part of the name, and is
# never stored as one.
#
# Usage:
#
#     ./scripts/seed-invitees.sh guests.csv [database.db]
#
# Re-running is safe. A name already on the list is left where it is, so the
# script adds newcomers without disturbing replies already collected. That also
# means adding "+1" to somebody already loaded changes nothing here — give them
# the plus one from the dashboard instead. Moving someone between parties is a
# manual edit — deliberately, because it changes who answers for whom.
set -euo pipefail

cd "$(dirname "$0")/.."

csv="${1:-}"
db="${2:-database.db}"

if [ -z "$csv" ] || [ ! -f "$csv" ]; then
    echo "usage: $0 <guests.csv> [database.db]" >&2
    exit 1
fi

if [ ! -f "$db" ]; then
    echo "no database at $db — start the server once so migrations run" >&2
    exit 1
fi

# SQLite string literals escape a quote by doubling it.
esc() { printf '%s' "$1" | sed "s/'/''/g"; }

trim() { printf '%s' "$1" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'; }

sql=$(
    echo "PRAGMA foreign_keys = ON;"
    echo "BEGIN;"
    # `last_insert_rowid()` moves on as soon as the first member is inserted,
    # so the new party's id is parked here for the rest of the line.
    echo "CREATE TEMP TABLE IF NOT EXISTS seed_party (id INTEGER);"

    while IFS= read -r line || [ -n "$line" ]; do
        line="${line%$'\r'}"
        line="$(trim "$line")"
        [ -z "$line" ] && continue
        # A `case` here would do, but bash 3.2 — still what macOS ships —
        # mis-parses one inside a command substitution.
        [[ "$line" == \#* ]] && continue

        members=()
        plus_ones=()
        IFS=',' read -ra fields <<< "$line"
        for field in "${fields[@]}"; do
            field="$(trim "$field")"
            [ -z "$field" ] && continue

            # The "+1" comes off before anything is stored, so the name written
            # to the list is the name on the envelope. `[[ ]]` rather than
            # `case` for the same bash 3.2 reason as above.
            plus=0
            if [[ "$field" == *+1 ]]; then
                plus=1
                field="$(trim "${field%+1}")"
                [ -z "$field" ] && continue
            fi

            members+=("$field")
            plus_ones+=("$plus")
        done
        [ ${#members[@]} -eq 0 ] && continue

        # A party is named for the people on it, which is all the name is for:
        # nothing shows it to a guest.
        party="${members[0]}"
        for ((i = 1; i < ${#members[@]}; i++)); do
            party="$party & ${members[i]}"
        done

        echo "DELETE FROM seed_party;"
        echo "INSERT INTO parties (name) VALUES ('$(esc "$party")');"
        echo "INSERT INTO seed_party VALUES (last_insert_rowid());"

        for ((i = 0; i < ${#members[@]}; i++)); do
            safe="$(esc "${members[i]}")"
            echo "INSERT INTO invitees (party_id, name, name_key, plus_one)
                  SELECT (SELECT id FROM seed_party), '$safe', lower('$safe'), ${plus_ones[i]}
                  WHERE NOT EXISTS (
                      SELECT 1 FROM invitees WHERE name_key = lower('$safe')
                  );"
        done
    done < "$csv"

    # A line whose names were all already on the list leaves an empty party.
    echo "DELETE FROM parties WHERE id NOT IN (SELECT party_id FROM invitees);"
    echo "COMMIT;"
)

printf '%s\n' "$sql" | sqlite3 "$db"

sqlite3 "$db" "SELECT 'guest list: ' || (SELECT count(*) FROM invitees) || ' invitees across ' || (SELECT count(*) FROM parties) || ' parties, ' || (SELECT count(*) FROM invitees WHERE plus_one = 1) || ' with a plus one';"
