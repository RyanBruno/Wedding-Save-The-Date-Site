#!/usr/bin/env bash
#
# Installs a package built by ./build.sh onto the machine that serves the site.
#
#     ./build.sh      # writes dist/wedding-website-*.pkg.tar.zst
#     ./deploy.sh     # copies that over, installs it, reloads nginx
#
# This is the counterpart to build.sh and covers the whole deployment: the
# site and assets come off this one machine, so there is nothing else to
# upload. The CDN in front of it is a pull cache that
# fetches from this box, and the cache headers it obeys ship in the package.
#
# Not to be confused with scripts/update-server.sh either: that one runs *on*
# the server and builds the package there. This box cannot compile the release
# WASM — not enough RAM — which is the whole reason build.sh exists.
#
# The install is deliberately unconditional. pacman is told to go ahead even
# when the package it is handed is the version already installed, because
# pkgver() stamps the commit count and short hash, so two builds of the same
# commit carry the same version while their contents can still differ: a
# changed asset, a rebuilt WASM, a PKGBUILD edit. A deploy that quietly did
# nothing in that case would be worse than a redundant one.
#
#     ./deploy.sh                        # newest package in dist/
#     ./deploy.sh --build                # build it first, then deploy
#     ./deploy.sh --host ryan@other.host
#     ./deploy.sh --package dist/wedding-website-0.1.0.r56.g78ec697-1-x86_64.pkg.tar.zst
#
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

host=${WEDDING_DEPLOY_HOST:-ryan@66.228.36.80}
package=
build=0

while [ $# -gt 0 ]; do
    case $1 in
        --host)    host=${2:?--host needs user@hostname}; shift 2 ;;
        --package) package=${2:?--package needs a path}; shift 2 ;;
        --build)   build=1; shift ;;
        -h|--help) sed -n '2,28p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;;
        *) echo "error: unknown argument '$1'" >&2; exit 1 ;;
    esac
done

step() { printf '\n==> %s\n' "$1"; }

# ------------------------------------------------------------------- build ---

if [ "$build" = 1 ]; then
    step "Building the package"
    "${repo_root}/build.sh"
fi

# ----------------------------------------------------------------- package ---

if [ -z "$package" ]; then
    # build.sh clears dist/ before every run, so there is only ever one build
    # in there — but it writes two files, and the -debug split package holds
    # the detached symbols rather than the site. Never install that one.
    for candidate in "${repo_root}"/dist/wedding-website-[0-9]*.pkg.tar.zst; do
        [ -e "$candidate" ] || continue
        case $(basename "$candidate") in *-debug-*) continue ;; esac
        package=$candidate
    done
fi

if [ -z "$package" ]; then
    echo "error: no package in ${repo_root}/dist. Run ./build.sh, or pass --package." >&2
    exit 1
fi

[ -f "$package" ] || { echo "error: no package at '$package'." >&2; exit 1; }

package_name=$(basename "$package")
checksum=$(sha256sum "$package" | cut -d' ' -f1)

echo "package $package_name"
echo "host    $host"
echo "sha256  $checksum"

# ------------------------------------------------------------------ upload ---

step "Copying the package to $host"

# /tmp, not the home directory: this is scratch, and the remote step below
# removes it once pacman has read it.
remote_package="/tmp/$package_name"
scp -q "$package" "$host:$remote_package"

# ----------------------------------------------------------------- install ---

step "Installing on $host"

# One ssh session for the whole remote half, so a failure anywhere in it stops
# the deploy with that step's exit status. $checksum and $remote_package are
# passed as arguments rather than interpolated into the script body.
ssh "$host" bash -euo pipefail -s -- "$remote_package" "$checksum" <<'REMOTE'
remote_package=$1
expected=$2

trap 'rm -f "$remote_package"' EXIT

actual=$(sha256sum "$remote_package" | cut -d' ' -f1)
if [ "$actual" != "$expected" ]; then
    echo "error: the package did not survive the copy." >&2
    echo "       expected $expected" >&2
    echo "       got      $actual" >&2
    exit 1
fi

# No --needed, on purpose: that flag is what would make pacman skip a package
# whose version is already installed, and reinstalling in that case is the
# point. pacman reinstalls at the same version and downgrades to an older one
# without further prompting under --noconfirm.
#
# The nginx config and the service's env file are backup=() entries in the
# PKGBUILD, so an install leaves whatever is on the box alone — including the
# certificate symlinks nginx reads through.
echo "installed before: $(pacman -Q wedding-website 2>/dev/null || echo 'not installed')"
sudo pacman -U --noconfirm "$remote_package"

# nginx serves the static files straight out of the package, so it needs no
# more than a reload — and it only gets one if it is actually running here.
if sudo systemctl is-active --quiet nginx.service; then
    sudo nginx -t
    sudo systemctl reload nginx.service
fi

printf '\n==> Installed\n'
pacman -Q wedding-website
REMOTE

# ------------------------------------------------------------------ verify ---

step "Checking the site"

# Over the public name and over TLS, which is the only way the site is served:
# port 80 redirects. This is the last word on whether the deploy worked, so it
# runs from here rather than on the box — a 200 through the real certificate
# and the real nginx is what a guest gets.
site="https://${host#*@}"
failed=0

for path in /; do
    code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 "${site}${path}" || echo 000)
    printf '%-14s %s\n' "$path" "$code"
    case $code in 2*|3*) ;; *) failed=1 ;; esac
done

if [ "$failed" = 1 ]; then
    echo "error: the site did not answer after the install." >&2
    echo "       ssh $host 'sudo nginx -t'" >&2
    exit 1
fi

step "Done"
