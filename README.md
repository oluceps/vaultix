# Vaultix

![built for nixos](https://img.shields.io/static/v1?logo=nixos&logoColor=white&label=&message=Built%20for%20NixOS&color=41439a)
![CI state eval](https://github.com/oluceps/vaultix/actions/workflows/eval.yaml/badge.svg)
![CI state vm-test](https://github.com/oluceps/vaultix/actions/workflows/vm-test.yaml/badge.svg)
![CI state clippy](https://github.com/oluceps/vaultix/actions/workflows/clippy.yaml/badge.svg)
![CI state fuzz](https://github.com/oluceps/vaultix/actions/workflows/fuzz.yaml/badge.svg)
![CI state statix](https://github.com/oluceps/vaultix/actions/workflows/statix.yaml/badge.svg)
![CI state doc](https://github.com/oluceps/vaultix/actions/workflows/doc.yaml/badge.svg)

Secret management for NixOS.

This project is highly inspired by [agenix-rekey](https://github.com/oddlama/agenix-rekey) and [sops-nix](https://github.com/Mic92/sops-nix). Based on rust [age](https://docs.rs/age/latest/age) crate.

+ Age Plugin Compatible
+ Support Template
+ Support identity with passphrase
+ Support PIV Card (Yubikey)
+ No Bash

## Setup

See [docs](https://oluceps.github.io/vaultix/)

## TODO

See [TODO](./TODO.md)

## Credits

+ [agenix](https://github.com/ryantm/agenix)
+ [agenix-rekey](https://github.com/oddlama/agenix-rekey)
+ [sops-nix](https://github.com/Mic92/sops-nix)
