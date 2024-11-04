# Vaultix

![built for nixos](https://img.shields.io/static/v1?logo=nixos&logoColor=white&label=&message=Built%20for%20NixOS&color=41439a)
![CI state](https://github.com/oluceps/vaultix/actions/workflows/lint.yaml/badge.svg)

Secret management for NixOS.

Highly inspired by agenix-rekey. Based on rust age crate.

> [!CAUTION]
> This project is in VERY early dev stage, NOT ready for production.

+ AGE Support Only
+ PIV Card (Yubikey) Support
+ Age Plugin Compatible
+ No Bash

## Prerequisite:

+ `nix-command` feature enabled
+ `flake-parts` structured config
+ `self` as specialArgs, to `nixosSystem`
+ `systemd.sysusers` or `services.userborn` option enabled

## Setup

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
      # relative to flake root, used for storing host public key -
      # re-encrypted secrets.
      storageLocation = "./secret/renc/${config.networking.hostName}";
      # extraRecipients =
      # not supported yet, plain to used in edit command
      #  [ data.keys.ageKey ];
      identity =
        # See https://github.com/str4d/age-plugin-yubikey
        # Also supports age native secrets (with password encrypted)
        (self + "/secret/age-yubikey-identity-0000ffff.txt.pub");
    };
    secrets = {
      example = {
        file = ./secret/example.age;
        mode = "640";
        owner = "root";
        group = "users";
        name = "example.toml";
        # symlink = true; # both not supported yet
        # path = "/some/place";
      };
      # ...
    };
  };
}
```

> [!TIP]
> During first setup, you need to manually create `storageLocation` and
> add it to git (create an placeholder since git ignores empty directory).

```bash
mkdir -p ./secret/renc/HOSTNAME_HERE
touch ./secret/renc/HOSTNAME_HERE/.placeholder
# So that you could add the directory to git (for recognizing by flake).
git add .
# Freely could remove placeholder once complete this step.
nix run .#vaultix.x86_64-linux.renc
```


## Cli Args

Seldon use cli directly. Use Nix Wrapped App such as `nix run .#vaultix.x86_64-linux.renc`.

Currently not support `edit` command, you could directly use rage for creating your encrypted file.


```bash
> ./vaultix --help
Usage: vaultix <profile> [-f <flake-root>] <command> [<args>]

Vaultix cli | Secret manager for NixOS

Positional Arguments:
  profile           toml secret profile

Options:
  -f, --flake-root  toplevel of flake repository
  --help            display usage information

Commands:
  renc              Re-encrypt changed files
  edit              Edit encrypted file # NOT SUPPORT YET
  check             Check secret status
  deploy            Decrypt and deploy cipher credentials
```

## Credits

+ [agenix](https://github.com/ryantm/agenix)
+ [agenix-rekey](https://github.com/oddlama/agenix-rekey)
