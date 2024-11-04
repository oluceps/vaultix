vaultixSelf:
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
    isAttrs
    isPath
    readFile
    literalExpression
    mkEnableOption
    mkIf
    assertMsg
    ;
  inherit (config.users) users;

  cfg = config.vaultix;

  # Using systemd instead of activationScript. Required.
  sysusers = assertMsg (
    options.systemd ? sysusers && (config.systemd.sysusers.enable || config.services.userborn.enable)
  ) "`systemd.sysusers` or `services.userborn` must be enabled.";

  storagePath = self + "/" + cfg.settings.storageDirRelative;
  storageExist = assertMsg (builtins.pathExists storagePath) "${storagePath} doesn't exist plz manually create and add to git first (may need a placeholder for git to recognize it)";

  settingsType = types.submodule (submod: {
    options = {

      storageDirRelative = mkOption {
        type = types.str;
        example = literalExpression ''./. /* <- flake root */ + "/secrets/renced/myhost" /* separate folder for each host */'';
        description = ''
          The local storage directory for rekeyed secrets. MUST be a str of path related to flake root.
        '';
      };
      storageDirStore = mkOption {
        type = types.path;
        readOnly = true;
        default = builtins.path { path = self + "/" + submod.config.storageDirRelative; };
        example = literalExpression ''./. /* <- flake root */ + "/secrets/renced/myhost" /* separate folder for each host */'';
        description = ''
          The local storage directory for rekeyed secrets. MUST be a str of path related to flake root.
        '';
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

      masterIdentities = mkOption {
        type =
          with types;
          let
            identityPathType = coercedTo path toString str;
          in
          listOf (
            # By coercing the old identityPathType into a canonical submodule of the form
            # ```
            # {
            #   identity = <identityPath>;
            #   pubkey = ...;
            # }
            # ```
            # we don't have to worry about it at a later stage.
            coercedTo identityPathType
              (
                p:
                if isAttrs p then
                  p
                else
                  {
                    identity = p;
                  }
              )
              (submodule {
                options = {
                  identity = mkOption { type = identityPathType; };
                  pubkey = mkOption {
                    type = coercedTo path (x: if isPath x then readFile x else x) str;
                    default = "";
                  };
                };
              })
          );
        default = [ ];
        example = [
          ./secrets/my-public-yubikey-identity.txt.pub
          {
            identity = ./password-encrypted-identity.pub;
            pubkey = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
          }
        ];
      };

      extraRecipients = mkOption {
        type = with types; listOf (coercedTo path toString str);

        default = [ ];
        example = [
          ./backup-key.pub
          "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq"
        ];
      };

      hostPubkey = mkOption {
        type = with types; coercedTo path (x: if isPath x then readFile x else x) str;
        default = # This pubkey is just binary 0x01 in each byte, so you can be sure there is no known private key for this
          "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";

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
      symlink = mkEnableOption "symlinking secrets to their destination" // {
        default = true;
      };
    };
  });

in
{
  options.vaultix = {

    package = mkOption {
      type = types.package;
      default = vaultixSelf.packages.${pkgs.system}.vaultix;
    };

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

  config =
    let
      profile = (pkgs.formats.toml { }).generate "secret-meta-${config.networking.hostName}" cfg;
      checkRencSecsReport =
        pkgs.runCommandNoCCLocal "secret-check-report" { }
          "${lib.getExe cfg.package} ${profile} check > $out";
    in
    mkIf (sysusers && storageExist) {
      systemd.services.vaultix-install-secrets = {
        wantedBy = [ "sysinit.target" ];
        after = [ "systemd-sysusers.service" ];
        unitConfig.DefaultDependencies = "no";
        serviceConfig = {
          Type = "oneshot";
          Environment = [
            ("storage:" + cfg.settings.storageDirStore)
            checkRencSecsReport
          ];
          ExecStart = "${lib.getExe cfg.package} ${profile} deploy";
          RemainAfterExit = true;
        };
      };
    };
}
