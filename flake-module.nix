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
        type = types.submodule (submod: {
          options = {
            cache = mkOption {
              type = types.addCheck types.str (s: (builtins.substring 0 1 s) == ".");
              default = "./secrets/cache";
            };
            nodes = mkOption {
              type = types.lazyAttrsOf types.unspecified;
              default = self.nixosConfigurations;
              defaultText = lib.literalExpression "self.nixosConfigurations";
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
            app = mkOption {
              type = types.lazyAttrsOf (types.lazyAttrsOf types.package);
              default = lib.mapAttrs (
                system: config':
                lib.genAttrs
                  [
                    "renc"
                    "edit"
                  ]
                  (
                    app:
                    import ./apps/${app}.nix {
                      inherit (submod.config)
                        nodes
                        identity
                        extraRecipients
                        cache
                        ;
                      inherit (config'.vaultix) pkgs;
                      inherit lib;
                      package = vaultixFlake.packages.${system}.default;
                      localFlake = self;
                    }
                  )
              ) config.allSystems;
              readOnly = true;
            };
          };
        });
        default = { };
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
          pkgs = mkOption {
            type = types.unspecified;
            default = pkgs;
            defaultText = lib.literalExpression "pkgs";
          };
        };
      }
    );
  };
  _file = __curPos.file;
}
