# Vaultix

Secret management for NixOS.

Highly inspired by agenix-rekey. Based on rust age crate.

> [!CAUTION]
> This project is in early dev stage, NOT ready for production.

+ AGE Key Support
+ PIV Card (Yubikey) Support

## Usage

Prerequisite:

+ flake.

+ nix-command feature enabled

+ flake-parts.

+ `self` as specialArgs, to `nixosSystem`.
