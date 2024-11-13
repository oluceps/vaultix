{
  description = "Vaultix";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      flake-parts,
      self,
      crane,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } (
      { flake-parts-lib, withSystem, ... }:
      let
        inherit (flake-parts-lib) importApply;
        flakeModules.default = importApply ./flake-module.nix {
          inherit (self) packages;
          inherit withSystem;
        };
      in
      {
        partitionedAttrs.checks = "dev";
        partitions.dev.extraInputsFlake = ./dev;
        partitions.dev.module =
          { inputs, ... }:
          {
            imports = [
              inputs.pre-commit-hooks.flakeModule
              ./dev/pre-commit-hooks.nix
            ];
          };

        imports =
          let
            inherit (inputs) flake-parts;
          in
          [
            flake-parts.flakeModules.easyOverlay
            flake-parts.flakeModules.partitions
            flakeModules.default
          ];
        systems = [
          "x86_64-linux"
          "aarch64-linux"
        ];
        perSystem =
          {
            self',
            pkgs,
            system,
            config,
            ...
          }:
          let
            target = (pkgs.lib.systems.elaborate system).config;
            toolchain = pkgs.rust-bin.nightly.latest.minimal.override {
              extensions = [ "rust-src" ];
              targets = [ target ];
            };
            craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
            inherit (craneLib) buildPackage;
          in
          {
            _module.args.pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = with inputs; [
                rust-overlay.overlays.default
                self.overlays.default
              ];
            };
            apps = {
              default = {
                type = "app";
                program = pkgs.lib.getExe self'.packages.default;
              };
            };

            packages = rec {
              default = buildPackage rec {
                src = craneLib.cleanCargoSource ./.;
                nativeBuildInputs = [
                  pkgs.rustPlatform.bindgenHook
                ];
                RUSTFLAGS = [
                  "-Zlocation-detail=none"
                  "-Zfmt-debug=none"
                ];
                cargoVendorDir = craneLib.vendorMultipleCargoDeps {
                  inherit (craneLib.findCargoFiles src) cargoConfigs;
                  cargoLockList = [
                    ./Cargo.lock

                    # Unfortunately this approach requires IFD (import-from-derivation)
                    # otherwise Nix will refuse to read the Cargo.lock from our toolchain
                    # (unless we build with `--impure`).
                    #
                    # Another way around this is to manually copy the rustlib `Cargo.lock`
                    # to the repo and import it with `./path/to/rustlib/Cargo.lock` which
                    # will avoid IFD entirely but will require manually keeping the file
                    # up to date!
                    "${toolchain.passthru.availableComponents.rust-src}/lib/rustlib/src/rust/library/Cargo.lock"
                  ];
                };

                cargoExtraArgs = ''-Z build-std -Z build-std-features="optimize_for_size" --target ${target}'';
                meta.mainProgram = "vaultix";
              };
              vaultix = default;
            };
            overlayAttrs = config.packages;

            formatter = pkgs.nixfmt-rfc-style;

            devShells.default = craneLib.devShell {
              inputsFrom = [
                pkgs.vaultix
              ];
              buildInputs = with pkgs; [
                just
                nushell
                cargo-fuzz
              ];
            };

          };
        flake = {
          inherit flakeModules;
          nixosModules.default =
            { pkgs, ... }:
            {
              imports = [ ./module ];
              vaultix.package = withSystem pkgs.stdenv.hostPlatform.system (
                { config, ... }: config.packages.vaultix
              );
            };
        };
      }
    );
}
