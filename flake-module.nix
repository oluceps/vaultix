localFlake:
{
  lib,
  self,
  config,
  flake-parts-lib,
  ...
}:
let
  inherit (lib)
    mkOption
    types
    ;

in
{
  options = {
    flake = flake-parts-lib.mkSubmoduleOptions {
      vaultix = mkOption {
        type = types.lazyAttrsOf (types.lazyAttrsOf types.package);
        default = lib.mapAttrs (
          system: config':
          lib.genAttrs
            [
              "renc"
              # "edit"
            ]
            (
              app:
              import ./apps/${app}.nix {
                inherit (config'.vaultix) nodes pkgs;
                package = localFlake.packages.${system}.default;
                inherit system;
              }
            )
        ) config.allSystems;
        readOnly = true;
      };
    };

    perSystem = flake-parts-lib.mkPerSystemOption (
      {
        lib,
        pkgs,
        ...
      }:
      {
        options.vaultix = {
          nodes = mkOption {
            type = types.lazyAttrsOf types.unspecified;
            default = self.nixosConfigurations;
            defaultText = lib.literalExpression "self.nixosConfigurations";
          };
          pkgs = mkOption {
            type = types.unspecified;
            default = pkgs;
            defaultText = lib.literalExpression "pkgs";
          };
        };
      }
    );
  };
}
