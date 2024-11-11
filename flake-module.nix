vaultixFlake:
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
                inherit (config'.vaultix)
                  nodes
                  pkgs
                  identity
                  extraRecipients
                  ;
                package = vaultixFlake.packages.${system}.default;
                localFlake = self;
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
          identity = mkOption {
            type =
              with types;
              let
                identityPathType = coercedTo path toString str;
              in
              nullOr identityPathType;
            default = null;
            example = ./password-encrypted-identity.pub;
          };
          extraRecipients = mkOption {
            type = with types; listOf (coercedTo path toString str);
            default = [ ];
            example = [
              "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq"
            ];
          };
        };
      }
    );
  };
}
