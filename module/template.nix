{
  config,
  lib,
  ...
}:
# this part basically inherit from
# https://github.com/Mic92/sops-nix/tree/60e1bce1999f126e3b16ef45f89f72f0c3f8d16f/modules/sops/templates
let
  inherit (lib)
    mkOption
    mkDefault
    mapAttrs
    types
    mkIf
    ;
  inherit (config.users) users;

  cfg = config.vaultix;

  inherit (import ./templateType.nix { inherit lib cfg users; }) templateType;

in
{
  options.vaultix = {

    templates = mkOption {
      type = types.attrsOf templateType;
      default = { };
      description = ''
        Attrset of templates.
      '';
    };

    placeholder = mkOption {
      type = types.attrsOf (
        types.mkOptionType {
          name = "coercibleToString";
          description = "value that can be coerced to string";
          check = lib.strings.isConvertibleWithToString;
          merge = lib.mergeEqualOption;
        }
      );
      default = { };
      visible = false;
      description = ''
        Identical with the attribute name of secrets, NOTICE this if you
        defined the `name` in secrets submodule.
      '';
    };
  };
  config = mkIf (config.vaultix.templates != { }) {
    vaultix.placeholder = mapAttrs (
      n: _: mkDefault "{{ ${builtins.hashString "sha256" n} }}"
    ) cfg.secrets;
  };
}
