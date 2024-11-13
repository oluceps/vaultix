# Vaultix

![built for nixos](https://img.shields.io/static/v1?logo=nixos&logoColor=white&label=&message=Built%20for%20NixOS&color=41439a)
![CI state eval](https://github.com/oluceps/vaultix/actions/workflows/eval.yaml/badge.svg)
![CI state vm-test](https://github.com/oluceps/vaultix/actions/workflows/vm-test.yaml/badge.svg)
![CI state clippy](https://github.com/oluceps/vaultix/actions/workflows/clippy.yaml/badge.svg)
![CI state fuzz](https://github.com/oluceps/vaultix/actions/workflows/fuzz.yaml/badge.svg)
![CI state statix](https://github.com/oluceps/vaultix/actions/workflows/statix.yaml/badge.svg)

Secret management for NixOS.

This project is highly inspired by [agenix-rekey](https://github.com/oddlama/agenix-rekey) and [sops-nix](https://github.com/Mic92/sops-nix). Based on rust [age](https://docs.rs/age/latest/age) crate.

+ Age Plugin Compatible
+ Support Template
+ Support identity with passphrase
+ Support PIV Card (Yubikey)
+ No Bash

## Setup

> [!NOTE]
> The document not finish yet. for more option details please see [module](./module).

### Prerequisite:

+ `nix-command` feature enabled
+ `flake-parts` structured config
+ `self` as one of `specialArgs` for nixosSystem
+ `systemd.sysusers` or `services.userborn` option enabled

### Configuration:

You could find the minimal configuration in [dev/test.nix](./dev/test.nix)

Adding flake-parts flakeModule:

```nix
# flake.nix
# ...
outputs = inputs@{ flake-parts, self, ... }:
  flake-parts.lib.mkFlake { inherit inputs; } (
  { ... }:
  {
    imports = [inputs.vaultix.flakeModules.default];

    flake.vaultix = {

      nodes = self.nixosConfigurations;

      identity =
        # See https://github.com/str4d/age-plugin-yubikey
        # Also supports age native secrets (recommend protected with passphrase)
        "/home/user/where/age-yubikey-identity-0000ffff.txt.pub";

      extraRecipients =
        # Optional. Backup keys
        # `identity` or private key of this could decrypt secret.
        [ ageKey ];

      cache =
        # Path *str* relative to flake root
        "./secrets/cache"; # default
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
      hostPubkey = "<HOST_SSH_PUBLIC_KEY_STR>";
    };

    secrets = {
    # this parts keeps identical with agenix
      example = {
        file = ./secret/example.age;
        mode = "640";
        owner = "root";
        group = "users";
        name = "example.toml";
        path =
          # Optional. Secret will be extract to this place directly
          # if user specified.
          "/some/place";
      };
    };

    # the templating function acts the same as sops-nix
    templates = {
      test = {
        name = "template.txt";
        # to be notice that the source secret file may have trailing `\n`
        content = "this is a template for testing ${config.vaultix.placeholder.example}";
        # removing trailing and leading whitespace by default
        trim = true;
        # ... permission options
      };
    }
  };
}
```

After this you could reference the path by:

```
<A-PathOption> = config.vaultix.secrets.example.path;
<B-PathOption> = config.vaultix.templates.test.path;
# ...
```

Then run [renc](#nix-app-renc) before deploy.

## Nix App: renc

This step is needed every time the host key or secret content changed.

The wrapped vaultix will decrypt cipher content to plaintext and encrypt it with target host public key, finally stored in `storagelocation`.

```bash
nix run .#vaultix.app.x86_64-linux.renc
```

## Nix App: edit

```bash
nix run .#vaultix.app.x86_64-linux.edit -- ./secrets/some.age
```

## TODO

See [TODO](./TODO.md)

## Credits

+ [agenix](https://github.com/ryantm/agenix)
+ [agenix-rekey](https://github.com/oddlama/agenix-rekey)
+ [sops-nix](https://github.com/Mic92/sops-nix)
