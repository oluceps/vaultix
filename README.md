# Vaultix

[![nixos infra](https://img.shields.io/badge/NixOS%20infra-3A8FB7?style=for-the-badge&logo=nixos&logoColor=BBDDE5)](https://nixos.wiki/wiki/Comparison_of_secret_managing_schemes)
[![doc](https://img.shields.io/badge/document-B4A582?style=for-the-badge&logo=gitbook&logoColor=white)](https://milieuim.github.io/vaultix/)
[![eval status](https://img.shields.io/github/actions/workflow/status/milieuim/vaultix/eval.yaml?branch=main&style=for-the-badge&label=eval&color=00AA90)](https://github.com/milieuim/vaultix/actions?query=branch%3Amain)
[![test status](https://img.shields.io/github/actions/workflow/status/milieuim/vaultix/test.yaml?branch=main&style=for-the-badge&label=test&color=00AA90)](https://github.com/milieuim/vaultix/actions?query=branch%3Amain)

Secret managing scheme for NixOS

Highly inspired by [agenix-rekey](https://github.com/oddlama/agenix-rekey) and [sops-nix](https://github.com/Mic92/sops-nix).

+ Based on age rust [implementation](https://docs.rs/age/latest/age)
+ Parallel encryption at host granularity
+ Support secure identity with passphrase
+ Support template for reusing insensitive stanza
+ Support Yubikey PIV with [age-yubikey-plugin](https://github.com/str4d/age-plugin-yubikey)
+ Fits well with new `sysuser` nixos userborn machenism
+ Design with [flake-parts](https://flake.parts/) and modulized flake
+ Compatible and tested with common nixos deployment tools

## Setup

See [docs](https://milieuim.github.io/vaultix/)
