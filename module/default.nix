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
    literalExpression
    mkEnableOption
    mkIf
    assertMsg
    warnIf
    ;
  inherit (config.users) users;

  cfg = config.vaultix;

  # Using systemd instead of activationScript. Required.
  sysusers = assertMsg (
    options.systemd ? sysusers && (config.systemd.sysusers.enable || config.services.userborn.enable)
  ) "`systemd.sysusers` or `services.userborn` must be enabled.";

  storagePath = "/" + self + "/" + cfg.settings.storageLocation;
  storageExist = builtins.pathExists storagePath;
  storageNotFoundWarn =
    warnIf (!storageExist)
      # NOTICE: here has ASCII control char `\e` which may not shown in editor
      "[31mpath not exist: ${storagePath}\n[41;97mThis build will fail[0m please run renc app and add ${cfg.settings.storageLocation} to git first."
      true;

  settingsType = types.submodule (submod: {
    options = {

      storageLocation = mkOption {
        type = types.str;
        example = literalExpression ''./. /* <- flake root */ + "/secrets/renced/myhost" /* separate folder for each host */'';
        description = ''
          The local storage directory for re-encrypted secrets. MUST be a str of path related to flake root.
        '';
      };
      storageInStore = mkOption {
        type = types.path;
        readOnly = true;
        default =
          if builtins.pathExists storagePath then
            (builtins.path { path = self + "/" + submod.config.storageLocation; })
          else
            pkgs.emptyDirectory;
      };

      decryptedDir = mkOption {
        type = types.path;
        default = "/run/vaultix";
        description = ''
          Folder where secrets are symlinked to
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
        readOnly = true;
        description = ''
          `config.services.openssh.hostKeys`
        '';
      };

      hostIdentifier = mkOption {
        type = types.str;
        default = config.networking.hostName;
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
        description = ''
          Where secrets are created before they are symlinked to {option}`vaultix.settings.decryptedDir`
        '';
      };

      hostPubkey = mkOption {
        type = with types; coercedTo path (x: if isPath x then readFile x else x) str;
        #example = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI.....";
        #example = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
        example = literalExpression "./secrets/host1.pub";
        #example = "/etc/ssh/ssh_host_ed25519_key.pub";
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
        default = "0";
        description = ''
          User of the decrypted secret.
        '';
      };
      group = mkOption {
        type = types.str;
        default = users.${submod.config.owner}.group or "0";
        defaultText = literalExpression ''
          users.''${config.owner}.group or "0"
        '';
        description = ''
          Group of the decrypted secret.
        '';
      };
      symlink = mkEnableOption "symlinking secrets to destination" // {
        default = true;
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
          "${lib.getExe cfg.package} ${profile} check > $out";
    in
    mkIf (sysusers && storageNotFoundWarn) {
      systemd.services.vaultix-install-secrets = {
        wantedBy = [ "sysinit.target" ];
        after = [ "systemd-sysusers.service" ];
        unitConfig.DefaultDependencies = "no";
        serviceConfig = {
          Type = "oneshot";
          Environment = [
            ("STORAGE=" + cfg.settings.storageInStore)
            checkRencSecsReport
          ];
          ExecStart = "${lib.getExe cfg.package} ${profile} deploy";
          RemainAfterExit = true;
        };
      };
    };
}
