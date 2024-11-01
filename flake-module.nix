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
    mkPackageOption
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
                userFlake' = self;
                inherit system;
              }
            )
        ) config.allSystems;
        readOnly = true;
        description = '''';
      };
    };

    perSystem = flake-parts-lib.mkPerSystemOption (
      {
        config,
        lib,
        pkgs,
        ...
      }:
      {
        options.vaultix = {
          nodes = mkOption {
            type = types.lazyAttrsOf types.unspecified;
            description = "All nixosSystems that should be considered for rekeying.";
            default = self.nixosConfigurations;
            defaultText = lib.literalExpression "self.nixosConfigurations";
          };
          pkgs = mkOption {
            type = types.unspecified;
            description = "The package set to use when defining agenix-rekey scripts.";
            default = pkgs;
            defaultText = lib.literalExpression "pkgs # (module argument)";
          };
        };
      }
    );
  };
}
