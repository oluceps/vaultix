# Vaultix

### Single-admin Secret Manage Scheme for NixOS.


This project is highly inspired by [agenix-rekey](https://github.com/oddlama/agenix-rekey) and [sops-nix](https://github.com/Mic92/sops-nix).

+ Based on age rust [implemention](https://docs.rs/age/latest/age)
+ Support secure identity with passphrase
+ Support template for reusing insensitive stanza
+ Support Yubikey PIV with [age-yubikey-plugin](https://github.com/str4d/age-plugin-yubikey)
+ Small closure size increase[^1]
+ Fits well with new `sysuser` nixos userborn machenism[^2]
+ Design with [flake-parts](https://flake.parts/) and modulized flake
+ Written in Rust for speed, safety, and simplicity
+ Compatible and tested with known[^3] nixos deployment tools




[^1]: nix build result on Nov 19 2024, 1465128 bytes.
[^2]: See merged pr [270727](https://github.com/NixOS/nixpkgs/pull/270727) and [332719](https://github.com/NixOS/nixpkgs/pull/332719)
[^3]: nixos-rebuild, apply, colmena was confirmed supported
