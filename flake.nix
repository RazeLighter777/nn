{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchainFor =
          p:
          p.rust-bin.selectLatestNightlyWith (
            toolchain:
            toolchain.default.override {
              extensions = [ "rust-src" ];
            }
          );
        rustToolchain = rustToolchainFor pkgs;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchainFor;

        src = craneLib.cleanCargoSource ./.;

        # Common arguments can be set here to avoid repeating them later
        # Note: changes here will rebuild all dependency crates
        commonArgs = {
          inherit src;
          cargoVendorDir = craneLib.vendorMultipleCargoDeps {
            inherit (craneLib.findCargoFiles src) cargoConfigs;
            cargoLockList = [
              ./Cargo.lock
              # Include rust-src's Cargo.lock so build-std can vendor std dependencies
              "${rustToolchain.passthru.availableComponents.rust-src}/lib/rustlib/src/rust/library/Cargo.lock"
            ];
          };
          cargoExtraArgs = "-Z build-std --target ${pkgs.stdenv.hostPlatform.config}";
          strictDeps = true;
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.openssl
            pkgs.gcc
          ];
          buildInputs = [
            pkgs.openssl.dev
            pkgs.postgresql.dev
            pkgs.sqlite.dev
          ];
        };

        my-crate = craneLib.buildPackage (
          commonArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;

            # Additional environment variables or build phases/hooks can be set
            # here *without* rebuilding all dependency crates
            # MY_CUSTOM_VAR = "some value";
          }
        );
      in
      {
        checks = {
          inherit my-crate;
        };

        packages.default = my-crate;

        apps.default = flake-utils.lib.mkApp {
          drv = my-crate;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          # to fix error adding symbols: DSO missing from command line
          RUSTFLAGS = "-C link-args=-L${pkgs.openssl.dev}/lib -lssl -lcrypto -lpq -lsqlite3";
          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
            pkgs.diesel-cli-ext
            pkgs.diesel-cli
            pkgs.podman-compose
          ];
        };
      }
    );
}
