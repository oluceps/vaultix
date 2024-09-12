{
  config,
  options,
  pkgs,
  lib,
  ...
}:
let
  inherit (lib)
    types
    mkOption
    literalExpression
    mkEnableOption
    mkIf
    ;
  inherit (config.users) users;

  cfg = config.vaultix;

  # Using systemd instead of activationScript. Required.
  sysusers = lib.assertMsg (
    options.systemd ? sysusers && (config.systemd.sysusers.enable || config.services.userborn.enable)
  ) "`systemd.sysusers` or `services.userborn` must be enabled.";

  secretType = types.submodule (
    { config, ... }:
    {
      options = {
        name = mkOption {
          type = types.str;
          default = config._module.args.name;
          defaultText = literalExpression "config._module.args.name";
          description = ''
            Name of the file used in {option}`age.secretsDir`
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
          default = "${cfg.secretsDir}/${config.name}";
          defaultText = literalExpression ''
            "''${cfg.secretsDir}/''${config.name}"
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
          default = "0";
          description = ''
            User of the decrypted secret.
          '';
        };
        group = mkOption {
          type = types.str;
          default = users.${config.owner}.group or "0";
          defaultText = literalExpression ''
            users.''${config.owner}.group or "0"
          '';
          description = ''
            Group of the decrypted secret.
          '';
        };
        symlink = mkEnableOption "symlinking secrets to their destination" // {
          default = true;
        };
      };
    }
  );
in
{
  options.vaultix = {
    secrets = mkOption {
      type = types.attrsOf secretType;
      default = { };
      description = ''
        Attrset of secrets.
      '';
    };
    secretsDir = mkOption {
      type = types.path;
      default = "/run/vaultix";
      description = ''
        Folder where secrets are symlinked to
      '';
    };
    secretsMountPoint = mkOption {
      type =
        types.addCheck types.str (
          s:
          (builtins.match "[ \t\n]*" s) == null # non-empty
          && (builtins.match ".+/" s) == null
        ) # without trailing slash
        // {
          description = "${types.str.description} (with check: non-empty without trailing slash)";
        };
      default = "/run/vaultix.d";
      description = ''
        Where secrets are created before they are symlinked to {option}`age.secretsDir`
      '';
    };
    identityPaths = mkOption {
      type = types.listOf types.path;
      default =
        if (config.services.openssh.enable or false) then
          map (e: e.path) (
            lib.filter (e: e.type == "rsa" || e.type == "ed25519") config.services.openssh.hostKeys
          )
        else
          [ ];
      description = ''
        Path to SSH keys to be used as identities in age decryption.
      '';
    };
  };

  config =
    let
      secretsMetadata = (pkgs.formats.toml { }).generate "secretsMetadata" cfg.secrets;
    in
    mkIf sysusers {
      test = secretsMetadata;
    };

}
