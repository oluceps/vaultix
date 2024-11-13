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
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
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
        imports = with inputs; [
          pre-commit-hooks.flakeModule
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
            ...
          }:
          let
            target =
              if system == "x86_64-linux" then
                "x86_64-unknown-linux-gnu"
              else if system == "aarch64-linux" then
                "aarch64-unknown-linux-gnu"
              else
                throw "unsupported platform";
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

            pre-commit = {
              check.enable = true;
              settings.hooks = {
                nixfmt-rfc-style.enable = true;
              };
            };

          };
        flake = {
          inherit flakeModules;

          overlays.default = final: prev: {
            vaultix = inputs.self.packages.${prev.system}.default;
          };
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
