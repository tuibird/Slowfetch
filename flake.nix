{
  description = "The slowest of fetches";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs @ {
    crane,
    flake-parts,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
      ];

      perSystem = {
        self',
        lib,
        pkgs,
        ...
      }: let
        craneLib = crane.mkLib pkgs;

        commonArgs = {
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              # Default files from crane (Rust and cargo files).
              (craneLib.fileset.commonCargoSources ./.)
              # UI assets.
              (lib.fileset.maybeMissing ./src/assets)
            ];
          };
          strictDeps = true;
        };

        slowfetch = craneLib.buildPackage (
          commonArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          }
        );
      in {
        checks = {
          inherit slowfetch;
        };

        packages.default = slowfetch;

        devShells.default = craneLib.devShell {
          inherit (self') checks;

          packages = with pkgs; [
            clippy
          ];
        };
      };
    };
}
