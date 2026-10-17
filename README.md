# Cameron & Ryan — Save the Date

Save-the-date site. [Leptos](https://leptos.dev), static. The first iteration is
a scroll-open hydrangea invitation and the existing photo gallery. It exposes
no RSVP form. The existing Axum API and deployment packaging remain in place
for migration continuity.

```
target/site/          whole site. Upload to any static host.
target/release/server the API. Finds an invitation, takes the reply, and serves
                      the admin dashboard that reads them all back.
```

## Layout

| Path | What |
| --- | --- |
| [`src/frontend`](src/frontend) | Leptos app — components, page, SCSS, images |
| [`src/backend`](src/backend) | Static generation at start-up, then the `/api` service |
| [`src/backend/src/admin`](src/backend/src/admin) | The admin dashboard — served by the API, never by the CDN |
| [`src/shared`](src/shared) | Wire types and validation both sides compile against |
| [`scripts/build-site.sh`](scripts/build-site.sh) | Produces deployable `target/site` |
| [`scripts/seed-invitees.sh`](scripts/seed-invitees.sh) | Loads the guest list the RSVP menu reads from |
| [`scripts/update-server.sh`](scripts/update-server.sh) | Run on the server: pull, package, install |
| [`packaging/`](packaging) | The pacman package: nginx config, systemd unit, cache headers |

## Dev

```bash
nix develop
cargo leptos watch
```

<http://127.0.0.1:3000>, hot reload. That HTML carries a dev auto-reload client.
Never deploy it.

## Build

```bash
./scripts/build-site.sh            # debug
./scripts/build-site.sh --release  # deployable
```

`cargo leptos build` alone clears `target/site` and writes no HTML — pages come
from running the server binary with `--generate-only`. Script does both. Use script.

Result:

```
target/site/index.html     prerendered page
target/site/pkg/           website.wasm, website.js, website.css
target/site/*.JPG          photography
```

Release trims WASM 6.4 MB to ~700 kB and drops auto-reload. **Always `--release`.**

HTML only, from existing build:

```bash
./target/release/server --generate-only
```

## Release

```bash
nix build
result/share/site/       # the whole site, as files
result/bin/website-api   # run next to SQLite file
```

## Deploy

One machine, one package. nginx serves the prerendered files and terminates
TLS; the Axum service sits behind it on localhost answering `/api/`. Same
origin, so `RSVP_API_BASE` stays empty and `RSVP_ALLOWED_ORIGINS` never fires.

Full instructions, first-time setup and certificates:
[`packaging/README.md`](packaging/README.md).

```bash
./build.sh && ./deploy.sh     # build here, install on the server
./scripts/update-server.sh    # or: pull, build and install on the server itself
```

## CDN

A CDN in front of the box is a pull cache. Nothing is uploaded to it, no deploy
invalidates it, and the only thing that decides what it keeps is the headers in
[`packaging/wedding-website.nginx.conf`](packaging/wedding-website.nginx.conf).

| Path | `Cache-Control` | Why |
| --- | --- | --- |
| `/api/*` | `no-store` | Every response is one guest's invitation, or the guest list |
| `/pkg/*` | `public, max-age=31536000, immutable` | Content hash in the filename |
| `/landing.js` | `public, max-age=31536000, immutable` | Content hash in the query string |
| images, `woff2` | `public, max-age=31536000, immutable` | Filenames are never reused |
| HTML | `public, max-age=0, s-maxage=600, must-revalidate` | Edge 10 min, browser never |
| 404 | `public, max-age=0, s-maxage=60, must-revalidate` | Short, so a bad deploy does not stick |

### Nothing under `/api` is cacheable

Both halves say so, and that is deliberate — which of the two is right should
not decide what the edge keeps.

- [`src/backend/src/api.rs`](src/backend/src/api.rs) layers `no-store` over the
  whole prefix, the dashboard included, so a route added later is covered
  without anyone remembering to cover it.
- nginx hides whatever the service sent and sets `no-store` itself, so the
  response carries one `Cache-Control` and not two.

Caching `/api/admin/overview` or `/api/admin/export.csv` even once would hand
the entire guest list to whoever asked next.

### Why the assets can be pinned for a year

`hash-files` is on in the workspace `Cargo.toml`, so cargo-leptos writes
`website.<hash>.js`, `.wasm` and `.css`, and the prerendered HTML links to those
names — `HydrationScripts` and `HashedStylesheet` read the hash back out of
`hash.txt`. `landing.js` is copied verbatim out of `assets-dir` and keeps its
name, so the shell appends a hash of its bytes as a query string instead
(`LANDING_JS_VERSION` in [`src/frontend/src/app.rs`](src/frontend/src/app.rs)).

A build that changed an asset changed its URL. There is no stale copy to purge,
and no window where new HTML asks for an old file. Only the HTML itself lives at
a URL a deploy reuses, which is why it is the one thing with a short TTL.

`build-site.sh` fails the build if the generated HTML still names an unhashed
asset — that combination plus the `immutable` header above is a year of 404s.

### Unknown paths are 404, not the home page

`try_files` ends in `=404` and `error_page 404 /404.html` serves the prerendered
[`/404` route](src/frontend/src/pages/not_found.rs). Falling back to
`/index.html` instead answers every scanner, typo and mistyped asset URL with
`200` and a copy of the landing page, and a CDN stores one of those per URL:

- the cache fills with junk keys and evicts the real assets;
- crawlers index an unbounded set of soft-404s;
- an asset URL that 404s during a bad deploy gets a **year-long immutable**
  cache entry containing HTML.

The 404 route is prerendered without setting a 404 status, because
`leptos_axum` throws away any static route that renders one. nginx attaches the
status. See [`not_found.rs`](src/frontend/src/pages/not_found.rs).

### Cloudflare settings

The headers above are the whole policy, so the dashboard only has to be told not
to override them:

- **Caching → Configuration → Browser Cache TTL: Respect Existing Headers.**
  Anything else replaces `max-age` on every response.
- **A cache rule matching `/api/*` set to Bypass cache.** `no-store` is already
  respected under the standard cache level; the rule is what survives someone
  later switching the zone to Cache Everything.
- **A cache rule for HTML with Edge TTL "Use cache-control header",** if the
  edge should hold pages at all. HTML is not cached by default.
- Leave Auto Minify and Rocket Loader off: both rewrite the markup, and the
  inline scripts carry a per-build nonce.

Origin logs see the edge, not the guest: `$remote_addr` is Cloudflare, and the
real address arrives in `CF-Connecting-IP`, which nothing here reads.

## Config

`.env`, read at start-up, not in version control:

```dotenv
DATABASE_URL=sqlite:./database.db
LEPTOS_SITE_ADDR=127.0.0.1:3000
LEPTOS_SITE_ROOT=target/site

# Comma-separated origins allowed to call /api.
# Unset means any origin — fine locally, pin it in production.
# RSVP_ALLOWED_ORIGINS=https://example.com
```

`RSVP_API_BASE` is the exception: **build-time**, not runtime, compiled into the WASM.

```bash
RSVP_API_BASE=https://api.example.com ./scripts/build-site.sh --release
```

Leave it unset when site and API share an origin, which is how the package is
built: nginx proxies `/api/` on the same host that serves the page.

`ADMIN_PASSWORD` turns the dashboard on; see below.

## Guest list

The card asks for a last name and finds the invitation under it. It never offers
a menu of who is invited: the prerendered HTML carries no names, and the API
names nobody until it is handed a last name already on the list.

One invitation per line, names comma-separated — everyone on a line is one
**party** and is asked about together:

```
Dana Whitfield, Sam Whitfield
Alex Moreno +1
Priya Raghunathan, Tom O'Dell, Ida O'Dell
```

```bash
./scripts/seed-invitees.sh guests.csv [database.db]
```

Re-running adds newcomers and leaves everyone already on the list alone, replies
included. Moving someone between parties is a manual edit, deliberately: it
changes who answers for whom.

Keep the CSV out of the repo. Names are the guest list.

### Plus ones

A name ending in `+1` is invited with a guest nobody has named yet — the marker
is an allowance, never stored as part of the name. The card then asks that person
whether they are bringing somebody, and takes the name if they are.

Two separate facts, in two separate places (see the
[0006 migration](src/backend/migrations/0006_plus_ones.sql)):

| Fact | Where | Meaning |
| --- | --- | --- |
| Allowance | `invitees.plus_one` | This person may bring a guest |
| Answer | a `plus_ones` row | This is who they are bringing |

No row means nobody is coming, the same way a missing `rsvps` row means nobody
has answered. A plus one is deliberately not an `invitees` row of its own: that
table's names are unique, not null, and drive the lookup, so a placeholder would
need a fake name — and would then be findable by surname.

Re-running the seeder does not add `+1` to somebody already on the list; it
leaves existing rows alone. Use the dashboard for that, where **Allow +1** sits
next to Rename on each guest. Taking an allowance away deletes the guest named
under it.

Who is bringing whom:

```bash
sqlite3 database.db "
  SELECT i.name, g.name AS guest
  FROM invitees i
  LEFT JOIN plus_ones g ON g.invitee_id = i.id
  WHERE i.plus_one = 1 ORDER BY i.name;"
```

### What counts as a last name

The final word of the name, derived by SQLite (see the
[0003 migration](src/backend/migrations/0003_invitee_last_name.sql)) so one rule
covers rows already stored. A surname written in several words matches too —
"Mary Jo Van Buren" answers to both `Buren` and `Van Buren`. Matching folds case,
and matches whole words only, so `ell` finds no O'Dells.

Check what a name resolves to:

```bash
sqlite3 database.db "SELECT name, last_name_key FROM invitees ORDER BY name;"
```

### What this does and does not hide

Hidden: the guest list. Nothing serves it, and no request returns a name that was
not asked for by surname.

Not hidden: whether a given surname is on the list. Anyone who guesses one
correctly is shown that invitation's names — which is the feature. There is no
rate limit, so a determined script could work through a dictionary of surnames.
Worth knowing before the URL goes out; a wedding guest list is usually not worth
more than that.

## The API

Two routes, both `POST`. The lookup is a read, but a query string would put
guests' names into CDN access logs and browser history, so it takes a body.

`POST /api/lookup` — which invitations carry a last name:

```json
{ "last_name": "Whitfield" }
```

```json
{
  "parties": [
    {
      "id": 2,
      "members": [
        { "id": 2, "name": "Dana Whitfield", "plus_one": false },
        { "id": 3, "name": "Sam Whitfield", "plus_one": true }
      ]
    }
  ]
}
```

No match is `{ "parties": [] }` and a `200` — an answer, not an error. At most 8
come back, each named by the people on it. The card always asks which invitation
is theirs, a single match included: one path through the form, and the guest
confirms the invitation before answering for anyone on it.

`POST /api/rsvp` — one answer per member of one party:

```json
{
  "party_id": 2,
  "last_name": "Whitfield",
  "replies": [
    { "invitee_id": 2, "attending": true, "plus_one_name": null },
    { "invitee_id": 3, "attending": true, "plus_one_name": "Nour Haddad" }
  ],
  "note": "Sam is bringing a friend from work."
}
```

```json
{
  "party_id": 2,
  "updated": false,
  "replies": [
    { "invitee_id": 2, "name": "Dana Whitfield", "attending": true, "plus_one": null },
    { "invitee_id": 3, "name": "Sam Whitfield", "attending": true, "plus_one": "Nour Haddad" }
  ]
}
```

`last_name` is carried back and re-checked against the party. Without it, party
and invitee ids are small integers anyone could count through — which would both
overwrite strangers' replies and hand back their names, straight past the lookup.
Answers are only accepted for people on that invitation.

Replying again overwrites the party's previous answers and returns
`"updated": true`. The whole party lands in one transaction, so a family is never
half-recorded. The note belongs to the party, not to a person, and an empty note
clears the stored one.

`plus_one_name` is only accepted for a member whose `plus_one` is set, and only
alongside `"attending": true`. It is the whole truth about the guest they bring:
a later reply without it withdraws whoever it named before.

Invalid submissions return `400` with `{ "error": "…" }`, which the form shows
verbatim. Limits live in [`src/shared/src/lib.rs`](src/shared/src/lib.rs),
enforced on both sides.

Read the replies — the [dashboard](#admin) is the comfortable way, and SQLite is
always there:

```bash
sqlite3 database.db "
  SELECT i.name, r.attending, n.note
  FROM rsvps r
  JOIN invitees i ON i.id = r.invitee_id
  LEFT JOIN party_notes n ON n.party_id = i.party_id
  ORDER BY r.updated_at DESC;"
```

Who has not replied:

```bash
sqlite3 database.db "
  SELECT name FROM invitees
  WHERE id NOT IN (SELECT invitee_id FROM rsvps) ORDER BY name;"
```

## Admin

The guest list, the replies and an editor for both, at **`/api/admin`**.

```bash
ADMIN_PASSWORD='something long' \
DATABASE_URL=sqlite:./database.db \
./target/release/server
```

Then <http://127.0.0.1:3000/api/admin>.

Unset `ADMIN_PASSWORD` and the dashboard answers `503` and says so. There is no
default password, because the page hands over the whole guest list — the one
thing [the lookup](#what-this-does-and-does-not-hide) exists to withhold.

On the packaged server it is set in `/etc/wedding-website/wedding-website.env`,
which systemd reads into the service's environment; see
[packaging/README.md](packaging/README.md#configuration).

### Where it lives, and why

Not a page of the site. Every route in the Leptos app is prerendered to flat
HTML and installed for nginx to serve, so a route defined there would be a
world-readable file that the CDN would copy to every edge. The dashboard is
three files compiled into the binary and served by this process; `target/site`
never contains it and the package never ships it.

It sits under `/api` because that prefix answers `no-store` on everything, so no
cache may hold a page of the guest list. Nothing new to configure — but it does
mean the dashboard is reachable from the internet, which is what the password is
for.

It carries no CORS layer. The guest-facing routes invite the CDN's origin to
post a reply; the dashboard is only ever fetched by itself.

### What it does

- **Overview** — head count, reply progress, the latest replies and every note
  guests have left.
- **Invitations** — one row per invitation; open one to record replies for each
  guest, edit the note, add or remove guests, give or take away a plus one and
  name the guest it brings, move a guest to another invitation, or delete the
  invitation outright.
- **Guests** — every named guest flat, filtered by answer, with the reply
  settable inline and any plus one shown under the name.
- **Export CSV** — one row per guest, with their plus one and the invitation's
  note alongside each of its members.

Recording a reply here writes the same rows the RSVP card does, so a guest who
later replies themselves overwrites it, and a guest whose answer is *cleared* is
back to never having answered — not to having declined.

### Sessions

The password buys a cookie: `HttpOnly`, `SameSite=Strict`, `Secure`, scoped to
`/api/admin`, good for twelve hours. Sessions live in memory, so restarting the
API — which a deploy does — signs every browser out.

Six wrong passwords in a row start a lock-out, doubling from 30 seconds to a
15-minute ceiling; the right password is refused during one too, which is the
point of it. A successful sign-in clears the counter.

`ADMIN_COOKIE_INSECURE=1` drops `Secure` from the cookie. It exists for
previewing over plain `http://` in a browser that will not keep a secure cookie
on localhost. Setting it on a deployed box puts the session token on the wire.

### Working on it

The HTML, CSS and JavaScript are `include_str!`ed into the binary, so editing
them means `cargo build -p server` and a restart — `cargo leptos watch` does not
pick them up.

## Why not server functions

A Leptos `#[server]` function needs a server rendering the page to route the call
through. None at runtime here — the HTML is a file nginx reads off disk — so the
form posts plain JSON to the standalone API. That split keeps the page fully
static, which is what makes it cacheable at an edge.
