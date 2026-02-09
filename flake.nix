{
  description =
    "rsmap – generate multi-layered, LLM-friendly index files for Rust codebases";
  inputs = {
    nixpkgs.url =
      "github:NixOS/nixpkgs/d6c71932130818840fc8fe9509cf50be8c64634f";
    flake-parts.url =
      "github:hercules-ci/flake-parts/57928607ea566b5db3ad13af0e57e921e6b12381";
    rust-overlay.url =
      "github:oxalica/rust-overlay/11a396520bf911e4ed01e78e11633d3fc63b350e";
  };
  outputs = inputs@{ self, nixpkgs, flake-parts, rust-overlay }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems =
        [ "aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux" ];
      perSystem = { config, self', inputs', pkgs, system, ... }:
        with import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        let
          rustStable = rust-bin.stable."1.93.0".minimal;
          rustDev = rust-bin.stable."1.93.0".default.override {
            extensions = [ "rust-analyzer" "rust-src" ];
          };
          rsmap = rustPlatform.buildRustPackage {
            pname = "rsmap";
            version = "0.1.1";
            src = lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ rustStable ];
            doCheck = false;
          };
        in {
          packages.default = rsmap;
          packages.rsmap = rsmap;
          devShells.default = mkShell { packages = [ rustDev ]; };
        };
    };
}
