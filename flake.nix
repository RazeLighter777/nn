{
  description = "Rust musl project with rustup";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          rustup
          pkg-config
          musl
          musl.dev
          gcc
        ];

        shellHook = ''
          export RUSTUP_HOME=$PWD/.rustup
          export CARGO_HOME=$PWD/.cargo

          if [ ! -d "$RUSTUP_HOME" ]; then
            rustup toolchain install stable
            rustup default stable
            rustup target add x86_64-unknown-linux-musl
          fi
        '';
      };
    };
}
