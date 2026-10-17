#!/usr/bin/env bash
#
# The whole update, run on the server that hosts this site:
#
#   1. fast-forward this working copy from GitHub
#   2. build the pacman package in packaging/
#   3. install the package and restart the API
#
#   ./scripts/update-server.sh
#
# Run it as the normal user that owns this checkout, not as root: makepkg
# refuses to run as root, and the two steps that do need privileges ask for
# them with sudo.
#
# There is no upload step. nginx on this machine serves the static files
# straight out of the installed package, and the CDN in front of it is a pull
# cache: it fetches from here, and what it may keep is decided entirely by the
# headers in packaging/wedding-website.nginx.conf.
set -euo pipefail

cd "$(dirname "$0")/.."
repo=$PWD

skip_pull=0

while [ $# -gt 0 ]; do
    case $1 in
        --skip-pull) skip_pull=1; shift ;;      # build what is already checked out
        -h|--help)
            sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//; $d'
            exit 0 ;;
        *) echo "error: unknown argument '$1'" >&2; exit 1 ;;
    esac
done

if [ "$(id -u)" -eq 0 ]; then
    echo "error: run this as the user that owns $repo; makepkg will not run as root." >&2
    exit 1
fi

sudo=sudo
command -v sudo >/dev/null || { echo "error: sudo not found." >&2; exit 1; }

step() { printf '\n==> %s\n' "$1"; }

# ---------------------------------------------------------------- git pull ---

if [ "$skip_pull" -eq 0 ]; then
    step "Pulling $(git rev-parse --abbrev-ref HEAD) from origin"

    # makepkg rewrites the PKGBUILD's pkgver= line after every VCS build, so this
    # checkout is dirty by the second run through here. Put that one line back —
    # and only that line, so a real edit to the PKGBUILD still stops the deploy.
    if ! git diff --quiet -- packaging/PKGBUILD &&
       [ -z "$(git diff -U0 -- packaging/PKGBUILD |
               grep -E '^[-+][^-+]' | grep -vE '^[-+]pkgver=')" ]; then
        git checkout -- packaging/PKGBUILD
    fi

    # The package is built from this repository's git history, so anything not
    # committed is invisible to the build anyway — but a dirty tree here means
    # someone was editing on the server, and a pull would either fail halfway or
    # silently drop their work out of the deploy.
    if ! git diff-index --quiet HEAD --; then
        echo "error: working tree has uncommitted changes; commit or stash them first." >&2
        git status --short >&2
        exit 1
    fi

    before=$(git rev-parse HEAD)
    git pull --ff-only
    after=$(git rev-parse HEAD)

    if [ "$before" = "$after" ]; then
        echo "already at $(git rev-parse --short HEAD) — rebuilding anyway"
    else
        git --no-pager log --oneline "$before..$after"
    fi
fi

# ----------------------------------------------------------------- makepkg ---

step "Building the package"

# -f overwrites the package file from an earlier run at the same version, which
# happens whenever this is re-run without a new commit. The install is a
# separate step below, so makepkg only builds here.
(cd packaging && makepkg -f)

# pkgver() stamps the package with the commit count and hash, so the filename is
# not known ahead of time; ask makepkg what it just wrote. The list also holds
# the -debug package makepkg splits the symbols into, which is not the one to
# install.
#
# Read the whole list first and pick the line in bash: piping makepkg into head
# closes the pipe after one line, and the SIGPIPE that follows is an exit
# status of 141 that pipefail turns into a failed deploy.
packagelist=$(cd packaging && makepkg --packagelist)

package=
while IFS= read -r line; do
    case $line in *-debug-*.pkg.tar.*) continue ;; esac
    package=$line
    break
done <<<"$packagelist"

[ -n "$package" ] || { echo "error: makepkg listed no package to install." >&2; exit 1; }
[ -f "$package" ] || { echo "error: makepkg produced no package at '$package'." >&2; exit 1; }
echo "package $package"

# ----------------------------------------------------------------- install ---

step "Installing the package"

$sudo pacman -U --noconfirm "$package"

# The unit files may have been replaced by the install; reload systemd before
# restarting so the running service picks up the version just written.
$sudo systemctl daemon-reload
$sudo systemctl restart wedding-website-api.service

# nginx only serves this machine's copy of the site, and its config is a backup=
# file pacman leaves alone on upgrade, so a reload is enough — and it is skipped
# entirely if nginx is not running on this host.
if $sudo systemctl is-active --quiet nginx.service; then
    $sudo nginx -t
    $sudo systemctl reload nginx.service
fi

step "Done"
systemctl --no-pager --lines=0 status wedding-website-api.service || true
