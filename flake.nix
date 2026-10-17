{
  description = "Rust + Leptos dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        rust = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-gARSjceSEFY+8IYGJFhN8O+oqKPN/eyMFW+aqGVu9hk=";
        };

        # Wire crane up to use your fenix toolchain
        craneLib = (crane.mkLib pkgs).overrideToolchain rust;

        commonArgs = {
          src = ./.;
          pname = "website";
          version = "0.1.0";
          nativeBuildInputs = [
            pkgs.cargo-leptos
            pkgs.binaryen
            pkgs.wasm-bindgen-cli_0_2_121
            pkgs.openssl
            pkgs.pkg-config
            pkgs.dart-sass
          ];
        };

        # Build deps with standard cargo so crane's stub-source trick doesn't
        # confuse cargo-leptos, which requires a real workspace layout.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the leptos app using the pre-built artifacts.
        #
        # The site is statically generated: after cargo-leptos produces the WASM
        # and CSS, the server binary is run once with `--generate-only`, which
        # prerenders every route into `target/site` and exits. That directory is
        # the complete deployable site and needs no server to host it.
        #
        # `$out/share/site` contains the complete deployable site.
        website = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          # Through the script rather than the two commands it wraps: it is
          # what exports LEPTOS_HASH_FILES for the generator — without which
          # the prerendered HTML names website.css while the file on disk is
          # website.<hash>.css — and what then fails the build if the HTML came
          # out that way regardless.
          buildPhaseCargoCommand = ''
            ./scripts/build-site.sh --release -vv
          '';
          doNotPostBuildInstallCargoBinaries = true;

          installPhaseCommand = ''
            mkdir -p $out/share
            cp -r target/site $out/share/site
          '';
        });

      in {
        # Your existing dev shell, unchanged
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rust
            pkgs.cargo-leptos
            pkgs.binaryen
            pkgs.openssl
            pkgs.wasm-bindgen-cli_0_2_121
            #pkgs.sass
            pkgs.dart-sass
            pkgs.pkg-config
          ];
          RUST_LOG = "info";
        };

        # New: buildable package
        packages.default = website;
      });
}
