# Vaultix

![built for nixos](https://img.shields.io/static/v1?logo=nixos&logoColor=white&label=&message=Built%20for%20NixOS&color=41439a)
![CI state eval](https://github.com/oluceps/vaultix/actions/workflows/eval.yaml/badge.svg)
![CI state vm-test](https://github.com/oluceps/vaultix/actions/workflows/vm-test.yaml/badge.svg)
![CI state clippy](https://github.com/oluceps/vaultix/actions/workflows/clippy.yaml/badge.svg)
![CI state fuzz](https://github.com/oluceps/vaultix/actions/workflows/fuzz.yaml/badge.svg)
![CI state statix](https://github.com/oluceps/vaultix/actions/workflows/statix.yaml/badge.svg)
![CI state doc](https://github.com/oluceps/vaultix/actions/workflows/doc.yaml/badge.svg)

Secret management for NixOS.

This project is highly inspired by [agenix-rekey](https://github.com/oddlama/agenix-rekey) and [sops-nix](https://github.com/Mic92/sops-nix).

+ Based on age rust [implemention](https://docs.rs/age/latest/age)
+ Support secure identity with passphrase
+ Support template for reusing insensitive stanza
+ Support Yubikey PIV with [age-yubikey-plugin](https://github.com/str4d/age-plugin-yubikey)
+ Small closure size increase (less than 1.5M)
+ Fits well with new `sysuser` nixos userborn machenism
+ Design with [flake-parts](https://flake.parts/) and modulized flake
+ Compatible and tested with most nixos deployment tools (nixos-rebuild, apply, colmena)

## Setup

See [docs](https://oluceps.github.io/vaultix/)

## TODO

See [todo](./TODO.md)
