{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      in
      with pkgs;
      {
        devShells.default = mkShell rec {
          nativeBuildInputs = [
            pkg-config
            clang
            cargo-expand
          ];

          buildInputs = [
            (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          ];

          LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
          RUST_BACKTRACE = "1";
        };
      }
    );
}
