{
  description = "A Git merge conflict resolution tool powered by AI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        lib = pkgs.lib;
        rustVersion = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustVersion;
          rustc = rustVersion;
        };
        projectVersion = lib.getVersion (builtins.readFile ./Cargo.toml);
      in
      {
        packages.default = rustPlatform.buildRustPackage {
          pname = "rizzler";
          version = projectVersion;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          # Check phase (disabled for now as tests fail in sandbox)
          doCheck = false;

          meta = with lib; {
            description = "A Git merge conflict resolution tool powered by AI";
            homepage = "https://github.com/ghuntley/rizzler";
            license = licenses.mit;
            maintainers = with maintainers; [ ghuntley ];
          };
        };

        devShells.default = pkgs.mkShell {
          # Development environment inputs
          inputsFrom = [ self.packages.${system}.default ];
          nativeBuildInputs = with pkgs; [
            rustVersion
            cargo-watch # Example dev tool
            # Add other dev tools like linters, formatters etc.
          ];

          # Environment variables for the dev shell
          # RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });
} 