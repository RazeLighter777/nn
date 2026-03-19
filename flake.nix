{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = crane.mkLib pkgs;

        # Common arguments can be set here to avoid repeating them later
        # Note: changes here will rebuild all dependency crates
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
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
          ];
        };
      }
    );
}
