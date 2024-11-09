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
    mkEnableOption
    mkDefault
    mapAttrs
    types
    literalExpression
    mkIf
    ;
  inherit (config.users) users;

  cfg = config.vaultix;

  templateType = types.submodule (submod: {
    options = {
      content = mkOption {
        type = types.str;
        default = "";
        defaultText = literalExpression "";
        description = ''
          Content of the template
        '';
      };
      name = mkOption {
        type = types.str;
        default = submod.config._module.args.name;
        defaultText = literalExpression "submod.config._module.args.name";
        description = ''
          Name of the file used in {option}`vaultix.settings.decryptedDir`
        '';
      };
      path = mkOption {
        type = types.str;
        default = "${cfg.settings.decryptedDir}/${submod.config.name}";
        defaultText = literalExpression ''
          "''${cfg.settings.decryptedDir}/''${config.name}"
        '';
        description = ''
          Path where the built template is installed.
        '';
      };
      mode = mkOption {
        type = types.str;
        default = "0400";
        description = ''
          Permissions mode of the built template in a format understood by chmod.
        '';
      };
      owner = mkOption {
        type = types.str;
        default = "0";
        description = ''
          User of the built template.
        '';
      };
      group = mkOption {
        type = types.str;
        default = users.${submod.config.owner}.group or "0";
        defaultText = literalExpression ''
          users.''${config.owner}.group or "0"
        '';
        description = ''
          Group of the built template.
        '';
      };
      symlink = mkEnableOption "symlinking template to destination" // {
        default = true;
      };
    };
  });

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
