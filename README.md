# Cameron & Ryan — Save the Date

Static Leptos save-the-date site for May 15, 2027. It has no RSVP form,
database, admin dashboard, or production API.

## Layout

- `src/frontend` — invitation and gallery UI
- `src/server` — local preview server and build-time static generator
- `scripts/build-site.sh` — produces the deployable site
- `packaging` — Arch package and nginx configuration

## Develop

```sh
./run.sh
```

The local server listens on `http://127.0.0.1:3000` unless the Leptos
configuration overrides it.

## Build

```sh
./scripts/build-site.sh --release
```

Deploy `target/site`. Production needs only a static web server.

Build the Arch package from a non-Arch workstation with:

```sh
./build.sh
```

Deploy the resulting package with:

```sh
./deploy.sh
```

See `packaging/README.md` for server setup.
