{
  lib,
  cfg,
  users,
  ...
}:

let
  inherit (lib)
    types
    elem
    mkOption
    literalExpression
    mkEnableOption
    ;
in
{
  templateType = types.submodule (submod: {
    options = {
      type = mkOption {
        type = types.str;
        default = "template";
        readOnly = true;
        description = "Identifier of option type";
      };
      content = mkOption {
        type = types.str;
        default = "";
        defaultText = literalExpression "";
        description = ''
          Content of the template
        '';
      };
      trim = (mkEnableOption { }) // {
        default = true;
        description = "remove trailing and leading whitespace of the secret content to insert";
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
        default =
          if elem submod.config._module.args.name cfg.beforeUserborn then
            "${cfg.settings.decryptedDirForUser}/${submod.config.name}"
          else
            "${cfg.settings.decryptedDir}/${submod.config.name}";

        defaultText = literalExpression ''
          if elem submod.config._module.args.name cfg.needbyUser then
            "${cfg.settings.decryptedDirForUser}/${submod.config.name}"
          else
            "${cfg.settings.decryptedDir}/${submod.config.name}";
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
        default = "root";
        description = ''
          User of the built template.
        '';
      };
      group = mkOption {
        type = types.str;
        default = users.${submod.config.owner}.group or "root";
        defaultText = literalExpression ''
          users.''${config.owner}.group or "root"
        '';
        description = ''
          Group of the built template.
        '';
      };
    };
  });
}
