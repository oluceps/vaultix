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

  allApps = [
    "edit"
    "renc"
  ];
in
{
  options = {
    # flake = flake-parts-lib.mkSubmoduleOptions {
    #   agenix-rekey = mkOption {
    #     type = types.lazyAttrsOf (types.lazyAttrsOf types.package);
    #     default = lib.mapAttrs (
    #       _system: config':
    #       lib.genAttrs allApps (
    #         app:
    #         import ./apps/${app}.nix {
    #           inherit (config'.agenix-rekey) nodes pkgs;
    #           agePackage = _: config'.agenix-rekey.agePackage;
    #           userFlake = self;
    #         }
    #       )
    #     ) config.allSystems;
    #     defaultText = "Automatically filled by agenix-rekey";
    #     readOnly = true;
    #     description = ''
    #       The agenix-rekey apps specific to your flake. Used by the `agenix` wrapper script,
    #       and can be run manually using `nix run .#agenix-rekey.$system.<app>`.
    #     '';
    #   };
    # };

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
          # package = mkOption {
          #   type = types.package;
          #   default = config.agenix-rekey.pkgs.callPackage ./nix/package.nix {
          #     inherit allApps;
          #   };
          #   defaultText = "<agenix script derivation from agenix-rekey>";
          #   readOnly = true;
          #   description = ''
          #     The agenix-rekey wrapper script `agenix`.
          #     We recommend adding this to your devshell so you can execute it easily.
          #     By using the package provided here, you can skip adding the overlay to your pkgs.
          #     Alternatively you can also pass it to your flake outputs (apps or packages).
          #   '';
          # };
        };
      }
    );
  };
}
