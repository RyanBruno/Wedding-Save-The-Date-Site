# Deploying on Arch

The site ships as a single pacman package. nginx serves the prerendered pages
and terminates TLS; the packaged Axum service sits behind it on localhost and
answers `/api/rsvp`.

```
/usr/share/webapps/wedding-website   prerendered HTML, pkg/, photography
/usr/bin/wedding-website-api         the RSVP service, listening on 127.0.0.1:3000
/etc/nginx/conf.d/wedding-website.conf
/etc/nginx/wedding-website.headers.conf   security headers, included per location
/etc/wedding-website/wedding-website.env  the service's settings, root-only
/usr/lib/systemd/system/wedding-website-api.service
/var/lib/wedding-website/database.db the replies
```

Because nginx proxies `/api/` on the same origin as the page, `RSVP_API_BASE`
stays empty at build time and nothing has to be rebuilt when the domain changes.

A CDN in front of this machine needs nothing installed and nothing uploaded: it
pulls from here, and `wedding-website.conf` is the whole of what it is allowed
to keep. `/api/` answers `no-store`; the fingerprinted assets are pinned for a
year; the HTML gets ten minutes at the edge and none in the browser. The
reasoning is in the comments at the top of that file, and the table is in the
repository README.

> The nginx config is a `backup=` file, so **an upgrade will not replace a copy
> you already have** — it leaves `wedding-website.conf.pacnew` beside it. On a
> machine installed before the cache rules existed, merging that file is what
> turns them on.

## Build the package

`cargo-leptos` and `wasm-bindgen-cli` 0.2.100 are not in the Arch repositories,
so install them once (they are the only things `makepkg` cannot pull in):

```bash
sudo pacman -S --needed base-devel git rustup binaryen dart-sass nginx
rustup default stable
rustup toolchain install nightly --target wasm32-unknown-unknown
cargo install --locked cargo-leptos
cargo install --locked wasm-bindgen-cli --version 0.2.100
```

Then, from this directory:

```bash
makepkg -si
```

The package is built from this working copy's **git history**, not its working
tree, so commit before building. `pkgver()` stamps the package with the commit
count and hash, which means every rebuild upgrades cleanly.

## First-time server setup

1. **Certificate.** A self-signed placeholder, valid for 825 days, covering the
   hostname, `localhost` and the primary IPv4 address:

   ```bash
   sudo /usr/share/doc/wedding-website/generate-self-signed-cert.sh
   ```

   Pass names explicitly once DNS exists:

   ```bash
   sudo /usr/share/doc/wedding-website/generate-self-signed-cert.sh cameronandryan.com www.cameronandryan.com
   ```

2. **Let nginx read `conf.d`.** Arch's stock `nginx.conf` neither includes
   `conf.d` nor leaves port 80 free. Inside its `http` block, comment out the
   default `server { listen 80; ... }` block and add:

   ```nginx
   include conf.d/*.conf;
   ```

3. **Set the admin dashboard's password.** It stays off until you do — the
   page answers 503 and says so.

   ```bash
   sudo nano /etc/wedding-website/wedding-website.env    # ADMIN_PASSWORD=
   ```

4. **Start both services.**

   ```bash
   sudo systemctl enable --now wedding-website-api.service
   sudo systemctl enable --now nginx.service
   ```

## Configuration

`/etc/wedding-website/wedding-website.env` holds everything the service reads
that is worth changing without a rebuild: `ADMIN_PASSWORD`,
`RSVP_ALLOWED_ORIGINS`, `DATABASE_URL`, `LEPTOS_SITE_ADDR`, `RUST_LOG`. The
shipped file documents each one and leaves all but the password commented out.

The binary knows nothing about the file: it reads environment variables, and the
unit's `EnvironmentFile=` is what puts them there. Settings in the file override
the `Environment=` defaults in the unit, which stay behind as the values the
service falls back on if the file is emptied.

It is mode 0600 and owned by root, because `ADMIN_PASSWORD` is in it — a
world-readable copy would hand the guest list to every user on the machine. That
is also why the password is not an `Environment=` line in the unit: unit files
under `/usr/lib/systemd/system` are world-readable, and `systemctl show` would
print it.

```bash
sudo nano /etc/wedding-website/wedding-website.env
sudo systemctl restart wedding-website-api.service
```

pacman lists it in `backup=()`, so upgrades keep your edits and drop any changed
template beside it as `.pacnew`.

## Replacing the self-signed certificate

A CDN checking the origin certificate — Cloudflare's Full (strict) mode, and the
equivalent elsewhere — will not talk to a host it cannot verify, and answers 5xx
for everything instead. So this box needs a real certificate, or an origin
certificate issued by the CDN itself.

The port-80 server block serves `/.well-known/acme-challenge/` out of
`/var/lib/letsencrypt` and redirects everything else, so certbot's webroot
plugin works without touching the packaged config:

```bash
sudo pacman -S certbot
sudo install -dm755 /var/lib/letsencrypt
sudo certbot certonly --webroot -w /var/lib/letsencrypt -d api.example.com
```

Rather than editing `ssl_certificate` and `ssl_certificate_key`, point the two
paths they already name at the issued files. The config is a `backup=` file, so
leaving it untouched means an upgrade never produces a `.pacnew` to merge:

```bash
sudo mv /etc/nginx/ssl/wedding-website.crt /etc/nginx/ssl/wedding-website.self-signed.crt
sudo mv /etc/nginx/ssl/wedding-website.key /etc/nginx/ssl/wedding-website.self-signed.key
sudo ln -sfn /etc/letsencrypt/live/api.example.com/fullchain.pem /etc/nginx/ssl/wedding-website.crt
sudo ln -sfn /etc/letsencrypt/live/api.example.com/privkey.pem   /etc/nginx/ssl/wedding-website.key
sudo nginx -t && sudo systemctl reload nginx
```

Renewal is `certbot-renew.timer`, which runs twice a day and is not enabled by
the certbot package. nginx holds the certificate it read at startup, so the
renewal has to reload it:

```bash
sudo systemctl enable --now certbot-renew.timer
printf '#!/bin/sh\nsystemctl reload nginx.service\n' \
  | sudo install -Dm755 /dev/stdin /etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh
sudo certbot renew --dry-run
```

With a real certificate, the `Strict-Transport-Security` header in the same
config can be uncommented.

## Operating it

```bash
systemctl status wedding-website-api
journalctl -u wedding-website-api -f

sudo sqlite3 /var/lib/wedding-website/database.db \
  "
  SELECT i.name, r.attending, n.note
  FROM rsvps r
  JOIN invitees i ON i.id = r.invitee_id
  LEFT JOIN party_notes n ON n.party_id = i.party_id
  ORDER BY r.updated_at DESC;"
```

The service runs under a `DynamicUser`, so the database is reachable only as
root. `/var/lib/wedding-website` survives package removal.

## Updating the site

On the server, as the user that owns the checkout:

```bash
./scripts/update-server.sh
```

That pulls from GitHub, builds the package, installs it, and restarts the API.
`--skip-pull` builds what is already checked out.

There is nothing to upload and nothing to invalidate. The CDN pulls from this
machine, and the assets it is told to keep for a year carry a content hash in
the name, so the new build is at URLs no cache has seen. The pages themselves
age out of the edge in ten minutes.

In a Claude Code session on the server, `/deploy-server` runs the same script and
then diagnoses, fixes, reruns, and pushes whatever it tripped over; the skill is
in `.claude/skills/deploy-server`.

By hand, the same thing is:

```bash
git pull
cd packaging && makepkg -si
sudo systemctl restart wedding-website-api
sudo systemctl reload nginx
```

The API prerenders the site into `/var/lib/wedding-website/generated` on every
start. That copy is unused — the deployed HTML is the one in the package — but
the binary does it unconditionally, so the unit gives it a writable scratch
directory to do it in.
