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
            # config,
            self',
            # inputs',
            pkgs,
            system,
            ...
          }:
          let
            toolchain = pkgs.rust-bin.nightly.latest.minimal;
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

            vaultix = {
              nodes = self.nixosConfigurations;
            };
            apps = {
              default = {
                type = "app";
                program = pkgs.lib.getExe self'.packages.default;
              };
            };

            packages = rec {
              default = (
                buildPackage {
                  src = craneLib.cleanCargoSource ./.;
                  nativeBuildInputs = [
                    pkgs.rustPlatform.bindgenHook
                  ];
                  meta.mainProgram = "vaultix";
                }
              );
              vaultix = default;
            };

            formatter = pkgs.nixfmt-rfc-style;

            devShells.default = craneLib.devShell {
              inputsFrom = [
                pkgs.vaultix
              ];

              # see https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/12
              RUST_SRC_PATH = "${
                pkgs.rust-bin.nightly.latest.default.override {
                  extensions = [ "rust-src" ];
                }
              }/lib/rustlib/src/rust/library";
              buildInputs = with pkgs; [
                just
                nushell
                rust-bin.beta.latest.complete
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
