---
name: deploy-server
description: Deploy the wedding site end to end — run scripts/update-server.sh on the Arch host, diagnose whatever it fails on, fix it in the repo, rerun until the deploy is clean, then commit and push the fixes to GitHub. Use when asked to deploy, ship, release, redeploy, update the server, or when update-server.sh has failed.
---

# Deploying the wedding site

[`scripts/update-server.sh`](../../../scripts/update-server.sh) does the deploy:
pull from GitHub, `makepkg` the package in `packaging/`, install it, restart the
API. This skill is the loop around it — run it, fix what breaks, run it again,
push the fixes.

Nothing is uploaded anywhere and no cache is purged. The CDN pulls from this
machine, and what it keeps is decided by the headers in
`packaging/wedding-website.nginx.conf`.

## Where the script runs

It runs **on the Arch server**, as the user that owns the checkout there, never
as root.

Check which machine this session is on before anything else:

```bash
uname -s; command -v makepkg pacman || echo "not the server"
```

- **On the server** (Linux, `makepkg` and `pacman` present): run the script
  directly.
- **Not on the server** (a Mac laptop, no `pacman`): the deploy has to go over
  SSH. Use `$WEDDING_SERVER` if it is set, otherwise ask for the host once, then
  run each command as `ssh "$host" 'cd <checkout> && ...'`. Do not try to
  `makepkg` locally — the package is an Arch x86_64 build.

## The loop

1. **Run it.** `./scripts/update-server.sh` — full run, no flags. It prints a
   `==>` banner per stage, so the stage that failed is the last banner printed.
   It is idempotent: rerunning after a fix is always safe.
2. **If it exits non-zero, diagnose.** Find the stage from the last banner, then
   read the actual error. Check the triage table below before guessing.
3. **Fix the cause in the repo**, not around it. A fix belongs in the tracked
   files — `scripts/`, `packaging/`, `src/` — when the cause is in the
   repo. When the cause is machine state (a missing build tool, a stray file in
   `/usr/share`), fix the machine and change no files; say so in the report.
4. **Rerun the whole script.** Not the failed stage alone — a later stage may
   depend on what an earlier one rebuilt. `--skip-pull` is fine while iterating
   on an uncommitted fix, since the package is built from git history and an
   uncommitted fix would otherwise be invisible; commit first when the fix is to
   something `makepkg` compiles.
5. **Stop after three failed attempts** at the same stage. Report what was
   tried, the exact error, and what is needed. Do not keep looping.
6. **Commit and push** once the deploy is clean — see below.

## Triage

| Failure | Cause | Fix |
|---|---|---|
| `error: working tree has uncommitted changes` | Someone edited on the server | Read `git status --short` and decide per file. Never `git checkout -- .` blindly — that is someone's work. The script already restores makepkg's own `pkgver=` rewrite, so that is not this. |
| `git pull --ff-only` refuses | Server checkout has diverged from origin | `git log --oneline origin/main..HEAD`. Report the commits and ask. Never reset or force. |
| `not on PATH: cargo-leptos` / `wasm-bindgen-cli` | Build tools are not Arch packages | `cargo install --locked cargo-leptos` and `cargo install --locked wasm-bindgen-cli --version 0.2.100`. Machine state; no repo change. |
| `Git LFS pointers are still in place` | The clone `makepkg` builds from holds pointer blobs; the photographs come from GitHub over SSH | `ssh -T git@github.com` to check the key. The repo is private, so the server needs a key that can read it. |
| Rust or Leptos compile error | A real bug in the pushed commit | Fix it in `src/`, commit, rerun. This is the one failure that must produce a commit. |
| `rustc` killed / out of memory | Small EC2 instance | Check `free -m` and for a swapfile. Report rather than silently changing build flags. |
| pacman `exists in filesystem` | A file was installed outside pacman | `pacman -Qo <path>` to find the owner. Do not reach for `--overwrite` without knowing what is being overwritten. |
| `nginx -t` fails | `/etc/nginx/conf.d/wedding-website.conf` is a `backup=` file, so an upgrade may have left a `.pacnew` beside it | Diff the two, merge deliberately, `nginx -t` again. |
| API restarts then fails | Runtime, not build | `journalctl -u wedding-website-api -n 50 --no-pager`. Usual causes: port 3000 already bound, `/var/lib/wedding-website` permissions, an unset `ADMIN_PASSWORD` in `/etc/wedding-website/wedding-website.env` (that one only disables the dashboard, it does not stop the service). |

## Verifying a clean run

The script ends with the unit's status. Confirm the service, the pages and the
cache headers — the headers are the whole CDN configuration, so a merge that
dropped them is invisible until the cache is already wrong:

```bash
systemctl --no-pager --lines=0 status wedding-website-api.service

# Reach the origin directly, past the CDN, so these are this machine's answers.
site=https://<host>
curl -sSf -o /dev/null -w '%{http_code}\n' "$site/"
curl -sSf -o /dev/null -w '%{http_code}\n' -X OPTIONS "$site/api/rsvp"

# no-store on the API, a year on a hashed asset, 404 (not 200) on an unknown path.
curl -sI "$site/api/admin"       | grep -i cache-control
curl -sI "$site/$(curl -s "$site/" | grep -o '/pkg/website\.[^"]*\.css' | head -1)" | grep -i cache-control
curl -s -o /dev/null -w '%{http_code}\n' "$site/no-such-page"
```

## Committing and pushing

Only after a clean deploy, and only the files that were part of a fix.

- **Never commit** `packaging/PKGBUILD`'s `pkgver=` churn (makepkg rewrites it
  every build), `target/`, `packaging/src/`, `packaging/pkg/`, `*.pkg.tar.zst`,
  or `database.db`. Check `git status --short` before staging; stage named paths,
  never `git add -A`.
- One commit per distinct fix. Message in the style of this repo's history — a
  sentence saying what the change does, lowercase imperative, no prefix tags:

  ```
  Keep the deploy from stalling on nginx's .pacnew

  <why, if it is not obvious from the diff>

  Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
  ```

- Push to `main` with a plain `git push origin main`. Never `--force`.
- **Show the diff and the commit message and get a yes before pushing**, unless
  the person asked for an unattended run (`--auto-push`, "don't ask", or
  similar). Pushing publishes to a private repo the server also pulls from.
- If nothing was fixed, there is nothing to commit. Say the deploy was clean and
  stop — do not manufacture a commit.

## Reporting back

End with: the commit that is now deployed (`git rev-parse --short HEAD`), what
broke and what fixed it, anything changed on the machine rather than in the repo
(that is the part that will bite on a rebuild), and the verification results.
