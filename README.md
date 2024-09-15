# Vaultix

Secret management for NixOS. Subset replacement of agenix.

Highly inspired by agenix-rekey.

> [!CAUTION]
> This project is in early dev stage, NOT ready for production.

## Target

+ Less Bash
+ Parallel Encryption / Decryption
+ AGE Key Support
+ PIV Card Support
+ **No** GPG Support

### Support platforms:

```nix
systems = [
  "x86_64-linux"
  "aarch64-linux"
];
```

## Usage

Prerequisite:

+ using flake.

+ using nix-command feature

+ using flake-parts.

+ pass `self` as specialArgs, to `nixosSystem`.


## Compatibilities

(Will) Inherited Options:

```
config.age.identityPaths      # /persist/keys/ssh_host_ed25519_key
config.age.secrets
config.age.secretsDir         # /run/vaultix
config.age.secretsMountPoint  # /run/vaultix.d
```
