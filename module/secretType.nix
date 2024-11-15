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
    ;
in
{
  secretType = types.submodule (submod: {
    options = {
      id = mkOption {
        type = types.str;
        default = submod.config._module.args.name;
        readOnly = true;
        description = "The true identifier of this secret as used in `age.secrets`.";
      };
      name = mkOption {
        type = types.str;
        default = submod.config._module.args.name;
        defaultText = literalExpression "submod.config._module.args.name";
        description = ''
          Name of the file used in {option}`vaultix.settings.decryptedDir`
        '';
      };
      file = mkOption {
        type = types.path;
        description = ''
          Age file the secret is loaded from.
        '';
      };
      path = mkOption {
        type = types.str;
        default =
          if elem submod.config._module.args.name cfg.needByUser then
            "${cfg.settings.decryptedDirForUser}/${submod.config.name}"
          else
            "${cfg.settings.decryptedDir}/${submod.config.name}";
        defaultText = literalExpression ''
          "''${cfg.settings.decryptedDir}/''${config.name}"
        '';
        description = ''
          Path where the decrypted secret is installed.
        '';
      };
      mode = mkOption {
        type = types.str;
        default = "0400";
        description = ''
          Permissions mode of the decrypted secret in a format understood by chmod.
        '';
      };
      owner = mkOption {
        type = types.str;
        default = "root";
        description = ''
          User of the decrypted secret.
        '';
      };
      group = mkOption {
        type = types.str;
        default = users.${submod.config.owner}.group or "root";
        defaultText = literalExpression ''
          users.''${config.owner}.group or "root"
        '';
        description = ''
          Group of the decrypted secret.
        '';
      };
    };
  });
}
