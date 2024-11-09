# Vaultix

![built for nixos](https://img.shields.io/static/v1?logo=nixos&logoColor=white&label=&message=Built%20for%20NixOS&color=41439a)
![CI state](https://github.com/oluceps/vaultix/actions/workflows/eval.yaml/badge.svg)

Secret management for NixOS.

Highly inspired by agenix-rekey and sops-nix. Based on rust age crate.

+ Age Plugin Compatible
+ Support identity with passphase
+ Support PIV Card (Yubikey)
+ Support Template
+ No Bash

## Setup

### Prerequisite:

+ `nix-command` feature enabled
+ `flake-parts` structured config
+ `self` as specialArgs, to `nixosSystem`
+ `systemd.sysusers` or `services.userborn` option enabled

> [!NOTE]
> The `edit` subcommand is not implement yet, For adding new secrets, you could just simply use `rage` cli, with recipient of `settings.identity`.

### Configuration:

Adding flake-parts flakeModule:

```nix
# flake.nix
# ...
outputs = inputs@{ flake-parts, self, ... }:
  flake-parts.lib.mkFlake { inherit inputs; } (
  { ... }:
  {
    imports = [inputs.vaultix.flakeModules.default];
    perSystem = {
      vaultix.nodes = self.nixosConfigurations;
    };
    # ...
  }
inputs.vaultix.url = "github:oluceps/vaultix";
```

Adding nixosModule config:

```nix
# configuration.nix
{
  imports = [ inputs.vaultix.nixosModules.default ];
  vaultix = {

    settings = {
      storageLocation =
      # relative to flake root, used for storing host public key -
      # re-encrypted secrets.
        "./secret/renc/${config.networking.hostName}";

      # extraRecipients =
      # not implement yet
      #  [ data.keys.ageKey ];

      identity =
        # See https://github.com/str4d/age-plugin-yubikey
        # Also supports age native secrets (with password encrypted)
        (self + "/secret/age-yubikey-identity-0000ffff.txt.pub");
    };

    secrets = {
    # this parts keeps identical with agenix
      example = {
        file = ./secret/example.age;
        mode = "640";
        owner = "root";
        group = "users";
        name = "example.toml";
        # symlink = true; # both not supported yet
        # path = "/some/place";
      };
    };
  };
}
```

After this you could reference the decrypted secret path by:

```
<AnyPathOption> = config.vaultix.secrets.example.path;
# ...
```

Then run [renc](#nix-app-renc) before deploy.

## Nix App: renc

This step is needed every time the host key or secret content changed.

The wrapped vaultix will decrypt cipher content to plaintext and encrypt it with target host public key, finally stored in `storagelocation`.

```bash
nix run .#vaultix.x86_64-linux.renc
```

## Cli Args

Seldom use cli directly. Use Nix Wrapped App such as `nix run .#vaultix.x86_64-linux.renc`.

Currently not support `edit` command, you could directly use rage for creating your encrypted file.


```bash
> ./vaultix --help
Usage: vaultix <profile> [-f <flake-root>] <command> [<args>]

Vaultix cli | Secret manager for NixOS

Positional Arguments:
  profile           secret profile

Options:
  -f, --flake-root  toplevel of flake repository
  --help            display usage information

Commands:
  renc              Re-encrypt changed files
  edit              Edit encrypted file # NOT SUPPORT YET
  check             Check secret status
  deploy            Decrypt and deploy cipher credentials
```

## TODO

See [TODO](./TODO.md)

## Credits

+ [agenix](https://github.com/ryantm/agenix)
+ [agenix-rekey](https://github.com/oddlama/agenix-rekey)
