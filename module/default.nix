{
  config,
  options,
  pkgs,
  lib,
  self,
  ...
}:
let
  inherit (lib)
    types
    mkOption
    isPath
    readFile
    literalMD
    warn
    literalExpression
    mkIf
    assertMsg
    ;
  inherit (config.users) users;

  cfg = config.vaultix;

  # Using systemd instead of activationScript. Required.
  sysusers = assertMsg (
    options.systemd ? sysusers && (config.systemd.sysusers.enable || config.services.userborn.enable)
  ) "`systemd.sysusers` or `services.userborn` must be enabled.";

  settingsType = types.submodule (submod: {
    options = {

      cacheInStore = mkOption {
        type = types.path;
        readOnly = true;
        default =
          let
            cachePath = "/" + self + "/" + self.vaultix.cache + "/" + config.networking.hostName;
          in
          if builtins.pathExists cachePath then
            builtins.path {
              path = cachePath;
            }
          else
            warn ''
              path not exist: ${cachePath}; Will auto create if you're running `renc`, else the build will fail.
            '' pkgs.emptyDirectory;

        defaultText = literalExpression "path in store";
        description = ''
          Secrets re-encrypted by each host public key. In nix store.
        '';
      };

      decryptedDir = mkOption {
        type = types.path;
        default = "/run/vaultix";
        defaultText = literalExpression "/run/vaultix";
        description = ''
          Folder where secrets are symlinked to.
        '';
      };

      hostKeys = mkOption {
        type = types.listOf (
          types.submodule {
            options = {
              path = mkOption {
                type = types.path;
              };
              type = mkOption {
                type = types.str;
              };
            };
          }
        );
        default = config.services.openssh.hostKeys;
        defaultText = literalExpression "config.services.openssh.hostKeys";
        readOnly = true;
        description = ''
          Ed25519 host private ssh key (identity) path that used for decrypting secrets while deploying.
          Default is `config.services.openssh.hostKeys`.

          Default format:
          ```nix
          [
            {
              path = "/path/to/ssh_host_ed25519_key";
              type = "ed25519";
            }
          ]
          ```
        '';
      };

      hostIdentifier = mkOption {
        type = types.str;
        default = config.networking.hostName;
        defaultText = literalExpression "config.networking.hostName";
        readOnly = true;
        description = ''
          Host identifier
        '';
      };

      decryptedMountPoint = mkOption {
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
        defaultText = literalExpression "/run/vaultix.d";
        description = ''
          Where secrets are created before they are symlinked to {option}`vaultix.settings.decryptedDir`
        '';
      };

      hostPubkey = mkOption {
        type = with types; coercedTo path (x: if isPath x then readFile x else x) str;
        example = literalExpression "./secrets/host1.pub";
        description = ''
          str or path that contains host public key.
          example:
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI....."
          "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpq....."
          "/etc/ssh/ssh_host_ed25519_key.pub"
        '';
      };
    };
  });

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
        default = "${cfg.settings.decryptedDir}/${submod.config.name}";
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
in
{
  imports = [ ./template.nix ];

  options.vaultix = {

    package = mkOption { defaultText = literalMD "`packages.vaultix` from this flake"; };

    settings = mkOption {
      type = settingsType;
      default = { };
      description = ''
        Attrset of settings.
      '';
    };

    secrets = mkOption {
      type = types.attrsOf secretType;
      default = { };
      description = ''
        Attrset of secrets.
      '';
    };
  };

  options.vaultix-debug = mkOption {
    type = types.unspecified;
    default = cfg;
  };

  config =
    let
      profile = pkgs.writeTextFile {
        name = "secret-meta-${config.networking.hostName}";
        text = builtins.toJSON cfg;
      };
      checkRencSecsReport =
        pkgs.runCommandNoCCLocal "secret-check-report" { }
          "${lib.getExe cfg.package} -p ${profile} check > $out";
    in
    mkIf sysusers {
      systemd.services.vaultix-install-secrets = {
        wantedBy = [ "sysinit.target" ];
        after = [ "systemd-sysusers.service" ];
        unitConfig.DefaultDependencies = "no";
        serviceConfig = {
          Type = "oneshot";
          Environment = [
            cfg.settings.cacheInStore
            checkRencSecsReport
          ];
          ExecStart = "${lib.getExe cfg.package} -p ${profile} deploy";
          RemainAfterExit = true;
        };
      };
    };
}
