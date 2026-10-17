#!/usr/bin/env bash
#
# Builds the complete static site into target/site.
#
# Two steps are needed, and neither is optional:
#
#   1. `cargo leptos build` compiles the WASM and CSS into target/site/pkg and
#      copies the assets across. It also CLEARS target/site first, so it never
#      leaves prerendered HTML behind.
#   2. The server binary writes the HTML. Run with `--generate-only` it renders
#      every route into target/site and exits without opening a port or touching
#      the database.
#
# Pass --release for a deployable build: that sets the Leptos env to PROD, which
# drops the dev auto-reload script from the generated HTML.
#
#   ./scripts/build-site.sh --release
#
# Set RSVP_API_BASE when the API will live on a different origin than the CDN;
# it is baked into the WASM at compile time.
set -euo pipefail

cd "$(dirname "$0")/.."

profile=debug
for arg in "$@"; do
    if [ "$arg" = "--release" ]; then
        profile=release
    fi
done

cargo leptos build "$@"

# hash-files = true is read out of the environment, not out of Cargo.toml:
# cargo-leptos exports LEPTOS_HASH_FILES when it runs the server itself, and
# step 2 below runs the binary directly, so it has to be exported here instead.
# Without it the prerendered HTML points at /pkg/website.css while the file on
# disk is website.<hash>.css — every asset a 404, and every one of them held by
# the CDN for a year under the immutable headers in the nginx config.
LEPTOS_HASH_FILES=true "./target/$profile/server" --generate-only

# ...which is worth checking rather than trusting, for exactly that reason.
if grep -qF '/pkg/website.css' target/site/index.html; then
    echo "error: the prerendered HTML names unhashed assets." >&2
    echo "       target/$profile/hash.txt is missing or was not read." >&2
    exit 1
fi

echo
echo "Static site ready in target/site — deploy that directory:"
find target/site -maxdepth 2 -name '*.html' -o -maxdepth 2 -name '*.wasm' | sort
