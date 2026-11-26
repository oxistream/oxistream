{
  description = "Oxistream development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
    in
    {
      packages = forAllSystems (pkgs: rec {
        rustypipe-botguard = pkgs.callPackage ./nix/rustypipe-botguard.nix { };
        default = rustypipe-botguard;
      });

      devShells = forAllSystems (
        pkgs:
        let
          botguard = pkgs.callPackage ./nix/rustypipe-botguard.nix { };
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = [
              rust
              botguard
              pkgs.sqlx-cli
              pkgs.nodejs_26
              pkgs.pnpm
              pkgs.cargo-insta
            ];
            OXISTREAM_BOTGUARD_BIN = "${botguard}/bin/rustypipe-botguard";
          };
        }
      );

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
