#!/usr/bin/env bash
#
# Builds packaging/PKGBUILD into an installable Arch package on a machine that
# is not Arch — and that has the memory the deployment box does not.
#
# The deploy box cannot compile this: cargo-leptos plus a release WASM build
# needs more RAM than it has. So the package is built here, inside a rootless
# Arch container, and the resulting .pkg.tar.zst is copied out to ./dist for
# scp'ing to the server:
#
#     ./build.sh
#     scp dist/wedding-std-*.pkg.tar.zst <server>:
#     ssh <server> 'sudo pacman -U wedding-std-*.pkg.tar.zst'
#
# The container matters for more than makepkg: binaries linked against this
# host's libc would not run on Arch. Everything is compiled against the same
# glibc the target box has.
#
# A few things are worth knowing about how this works:
#
#   * podman runs rootless, so no daemon and no root on the host. If podman is
#     not on PATH it is pulled from nixpkgs for the duration of the build.
#   * The repository is copied into the container rather than bind-mounted
#     writable, because makepkg writes src/ and pkg/ next to the PKGBUILD and
#     those should not land in the working copy. target/ is left behind.
#   * --userns=keep-id maps you onto the container's build user. Without it
#     that user lands on a subordinate UID, which cannot read your 0600 files
#     and writes the finished package to ./dist owned by nobody in particular.
#   * The photographs are Git LFS objects. The PKGBUILD pulls them from GitHub
#     because makepkg's clone carries only pointers; here WEDDING_LFS_REMOTE
#     redirects that remote at the copied-in repository, which already has the
#     objects. No SSH key ever enters the container.
#
# Toolchain versions live in the image and are cached between runs. Pass
# --rebuild-image after changing them, or after a Rust or cargo-leptos bump.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
image="wedding-website-archbuilder:latest"
out_dir="${repo_root}/dist"
cache_dir="${repo_root}/.build-cache/cargo"

# Kept in step with Cargo.lock: cargo-leptos runs wasm-bindgen against the
# artifacts rustc produced, and a version skew between the two is a hard error.
wasm_bindgen_version="$(
    awk '/^name = "wasm-bindgen"$/ { getline; gsub(/[":]/, ""); print $3; exit }' \
        "${repo_root}/Cargo.lock"
)"

rebuild_image=0
for arg in "$@"; do
    case "$arg" in
        --rebuild-image) rebuild_image=1 ;;
        -h|--help) sed -n '2,35p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;;
        *) echo "unknown argument: $arg" >&2; exit 1 ;;
    esac
done

# podman from PATH if it is there, otherwise from nixpkgs. The wrapper keeps the
# rest of the script from caring which.
if command -v podman >/dev/null; then
    podman() { command podman "$@"; }
else
    echo "==> podman not on PATH; using nixpkgs#podman"
    podman() {
        nix --extra-experimental-features 'nix-command flakes' \
            shell nixpkgs#podman -c podman "$@"
    }
fi

# podman refuses to do anything without a signature policy, and NixOS ships no
# /etc/containers. Writing the permissive default is what every distribution's
# podman package does anyway.
policy="${XDG_CONFIG_HOME:-$HOME/.config}/containers/policy.json"
if [ ! -e "$policy" ] && [ ! -e /etc/containers/policy.json ]; then
    echo "==> writing $policy"
    mkdir -p "$(dirname "$policy")"
    cat > "$policy" <<'JSON'
{
    "default": [{"type": "insecureAcceptAnything"}]
}
JSON
fi

if [ "$rebuild_image" = 1 ] || ! podman image exists "$image"; then
    echo "==> building $image (wasm-bindgen ${wasm_bindgen_version})"
    # An empty build context on purpose. The image copies nothing in, and
    # handing podman the repository would make it tar up target/.
    context="$(mktemp -d)"
    trap 'rm -rf "$context"' EXIT
    podman build \
        --build-arg "WASM_BINDGEN_VERSION=${wasm_bindgen_version}" \
        --tag "$image" \
        --file - "$context" <<'CONTAINERFILE'
FROM docker.io/library/archlinux:base-devel
ARG WASM_BINDGEN_VERSION

# makedepends from the PKGBUILD, plus git-lfs for the photographs. base-devel
# already carries the rest of what makepkg expects.
RUN pacman -Syu --noconfirm --needed \
        git git-lfs rustup binaryen dart-sass && \
    pacman -Scc --noconfirm

# makepkg refuses to run as root, on purpose: a bad PKGBUILD would be running
# as root too.
RUN useradd --create-home --shell /bin/bash builder

# One shared toolchain, so builder does not download a second nightly.
# CARGO_HOME is deliberately not set globally: at run time it has to stay the
# default ~/.cargo, which is where the host's registry cache gets mounted.
# cargo-leptos and wasm-bindgen-cli are not in the Arch repositories, and go to
# /usr/local for the same reason — a mount over ~/.cargo cannot hide them.
ENV RUSTUP_HOME=/opt/rustup
RUN export CARGO_HOME=/opt/cargo CARGO_INSTALL_ROOT=/usr/local && \
    rustup toolchain install nightly --profile minimal \
        --component rustfmt --component clippy --component rust-src \
        --target wasm32-unknown-unknown && \
    rustup default nightly && \
    cargo install --locked cargo-leptos && \
    cargo install --locked wasm-bindgen-cli --version "${WASM_BINDGEN_VERSION}" && \
    rm -rf /opt/cargo && \
    chmod -R a+rwX /opt/rustup

USER builder
WORKDIR /home/builder
CONTAINERFILE
fi

mkdir -p "$out_dir" "$cache_dir"
rm -f "$out_dir"/*.pkg.tar.zst

echo "==> running makepkg in $image"
podman run --rm \
    --userns=keep-id:uid=1000,gid=1000 \
    --volume "${repo_root}:/src:ro" \
    --volume "${out_dir}:/out" \
    --volume "${cache_dir}:/home/builder/.cargo" \
    --env GIT_LFS_SKIP_SMUDGE=1 \
    --workdir /home/builder \
    "$image" bash -euo pipefail -c '
    # A writable copy of the working tree. What is left out is either wrong for
    # the container (target/ holds this host'"'"'s object files, result is the nix
    # build symlink), or has no business in a build at all: local environment
    # files, the untracked photo scratch directory, and
    # this script'"'"'s own output.
    mkdir -p /home/builder/work
    tar -C /src -cf - \
        --exclude=./target --exclude=./result \
        --exclude=./.env --exclude=./more_pics --exclude=./.sass-cache \
        --exclude=./dist --exclude=./.build-cache \
        . | tar -C /home/builder/work -xf -

    # Where the PKGBUILD reaches for the LFS objects. The copy already has
    # them, so this stays offline and needs no credentials. The clone makepkg
    # makes checks out pointers (GIT_LFS_SKIP_SMUDGE); prepare() replaces them.
    # An insteadOf rule would not do here: git-lfs resolves the remote on its
    # own and never sees git'"'"'s rewrites, so it would go to GitHub over SSH.
    export WEDDING_LFS_REMOTE="file:///home/builder/work"
    git config --global --add safe.directory "*"

    cd /home/builder/work/packaging
    MAKEFLAGS="-j$(nproc)" makepkg --force --noconfirm --clean

    cp -v ./*.pkg.tar.zst /out/
'

echo
echo "==> package ready:"
ls -lh "$out_dir"/*.pkg.tar.zst
echo
echo "Copy it to the server and install it there:"
echo "    scp dist/*.pkg.tar.zst <server>:"
echo "    ssh <server> 'sudo pacman -U wedding-std-*.pkg.tar.zst'"
