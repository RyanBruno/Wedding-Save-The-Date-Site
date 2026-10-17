#!/usr/bin/env sh
# nixpkgs' rustc links wasm32 with `lld` from PATH instead of a bundled
# rust-lld, and the user profile does not carry lld, so pull it in for the
# duration of the watch.
exec nix --extra-experimental-features 'nix-command flakes' \
  shell nixpkgs#lld -c cargo leptos watch
