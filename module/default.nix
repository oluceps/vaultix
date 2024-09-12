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

  settingsType = types.submodule (
    { config, ... }:
    {
      options = {

        decryptedDir = mkOption {
          type = types.path;
          default = "/run/vaultix";
          description = ''
            Folder where secrets are symlinked to
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
              coercedTo identityPathType (p: if isAttrs p then p else { identity = p; }) (submodule {
                options = {
                  identity = mkOption { type = identityPathType; };
                  pubkey = mkOption {
                    type = nullOr (coercedTo path (x: if isPath x then readFile x else x) str);
                    default = null;
                  };
                };
              })
            );
          description = ''
            The list of age identities that will be presented to `rage` when decrypting the stored secrets
            to rekey them for your host(s). If multiple identities are given, they will be tried in-order.

            The recommended options are:

            - Use a split-identity ending in `.pub`, where the private part is not contained (a yubikey identity)
            - Use an absolute path to your key outside of the nix store ("/home/myuser/age-master-key")
            - Or encrypt your age identity and use the extension `.age`. You can encrypt an age identity
              using `rage -p -o privkey.age privkey` which protects it in your store.

            If you are using YubiKeys, you can specify multiple split-identities here and use them interchangeably.
            You will have the option to skip any YubiKeys that are not available to you in that moment.

            To prevent issues with master keys that may be sometimes unavailable during encryption,
            an alternate syntax is possible:

            ```nix
            age.rekey.masterIdentities = [
              {
                # This has the same type as the other ways to specify an identity.
                identity = ./password-encrypted-identity.pub;
                # Optional; This has the same type as `age.rekey.hostPubkey`
                # and allows explicit association of a pubkey with the identity.
                pubkey = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
              }
            ];
            ```

            If a pubkey is explicitly specified, it will be used
            in place of the associated identity during encryption. This prevents additional prompts
            in the case of a password encrypted key file or prompts for identities that can only be accessed
            by certain people in a multi-user scenario. For Yubikey identities the pubkey can be automatically
            extracted from the identity file, if there is a comment of the form `Recipient: age1yubikey1<key>`
            present in the identity file.
            This should be the case for identity files generated by the `age-plugin-yubikey` CLI.
            See the description of [pull request #28](https://github.com/oddlama/agenix-rekey/pull/28)
            for more information on the exact criteria for automatic pubkey extraction.

            For setups where the primary identity may change depending on the situation, e.g. in a multi-user setup,
            where each person only has access to their own personal Yubikey, check out the
            `AGENIX_REKEY_PRIMARY_IDENTITY` environment variable.

            Be careful when using paths here, as they will be copied to the nix store. Using
            split-identities is fine, but if you are using plain age identities, make sure that they
            are password protected.
          '';
          default = [ ];
          example = [
            ./secrets/my-public-yubikey-identity.txt
            {
              identity = ./password-encrypted-identity.pub;
              pubkey = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
            }
          ];
        };

        extraEncryptionPubkeys = mkOption {
          type = with types; listOf (coercedTo path toString str);
          description = ''
            When using `agenix edit FILE`, the file will be encrypted for all identities in
            rekey.masterIdentities by default. Here you can specify an extra set of pubkeys for which
            all secrets should also be encrypted. This is useful in case you want to have a backup indentity
            that must be able to decrypt all secrets but should not be used when attempting regular decryption.

            If the coerced string is an absolute path, it will be used as if it was a recipient file.
            Otherwise, the string will be interpreted as a public key.
          '';
          default = [ ];
          example = [
            ./backup-key.pub
            "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq"
          ];
        };

        hostPubkey = mkOption {
          type = with types; coercedTo path (x: if isPath x then readFile x else x) str;
          description = ''
            The age public key to use as a recipient when rekeying. This either has to be the
            path to an age public key file, or the public key itself in string form.
            HINT: If you want to use a path, make sure to use an actual nix path, so for example
            `./host.pub`, otherwise it will be interpreted as the content and cause errors.
            Alternatively you can use `readFile "/path/to/host.pub"` yourself.

            If you are managing a single host only, you can use `"/etc/ssh/ssh_host_ed25519_key.pub"`
            here to allow the rekey app to directly read your pubkey from your system.

            If you are managing multiple hosts, it's recommended to either store a copy of each
            host's pubkey in your flake and use refer to those here `./secrets/host1-pubkey.pub`,
            or directly set the host's pubkey here by specifying `"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI..."`.

            Make sure to NEVER use a private key here, as it will end up in the public nix store!
          '';
          default = # This pubkey is just binary 0x01 in each byte, so you can be sure there is no known private key for this
            "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";

          #example = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI.....";
          #example = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
          example = literalExpression "./secrets/host1.pub";
          #example = "/etc/ssh/ssh_host_ed25519_key.pub";
        };

      };
    }
  );

  secretType = types.submodule (
    { config, ... }:
    {
      options = {
        id = mkOption {
          type = types.str;
          default = config._module.args.name;
          readOnly = true;
          description = "The true identifier of this secret as used in `age.secrets`.";
        };
        name = mkOption {
          type = types.str;
          default = config._module.args.name;
          defaultText = literalExpression "config._module.args.name";
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
          default = "${cfg.settings.decryptedDir}/${config.name}";
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
      secretsMetadata = (pkgs.formats.toml { }).generate "secretsMetadata" cfg.secrets;
    in
    mkIf sysusers {
      test = secretsMetadata;
    };
}