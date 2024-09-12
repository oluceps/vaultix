{
  description = "Vaultix";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk.url = "github:nix-community/naersk";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = with inputs; [
        pre-commit-hooks.flakeModule
        ./test
      ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          ...
        }:
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = with inputs; [
              rust-overlay.overlays.default
              self.overlays.default
            ];
          };

          packages.default =
            let
              toolchain = pkgs.rust-bin.nightly.latest.minimal;
              inherit (pkgs) callPackage lib;
              inherit
                (callPackage inputs.naersk (
                  lib.genAttrs [
                    "cargo"
                    "rustc"
                  ] (n: toolchain)
                ))
                buildPackage
                ;
            in
            (buildPackage { src = ./.; });

          formatter = pkgs.nixfmt-rfc-style;

          devShells.default = pkgs.mkShell {
            inputsFrom = [
              pkgs.vaultix
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
        overlays.default = final: prev: {
          vaultix = inputs.self.packages.${prev.system}.default;
        };
        nixosModules.default = ./module;
      };
    };
}
