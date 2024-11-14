---
title: Prerequisits
id: prerequisits
---

# Prerequisits

## enable `nix-command` `flakes` features
Vaultix depends on nix flake and nix apps to perform basic function.

```nix
nix.settings = {
  experimental-features = [
    "nix-command"
    "flakes"
  ];
}
```

## `flake-parts` structured config

[flake-parts](https://flake.parts/) provides modulized flake config, vaultix using [flake module](https://github.com/oluceps/vaultix/blob/main/flake-module.nix) produce nix apps.

## `self` as one of `specialArgs` for nixosSystem

For passing top-level flake arguments to nixos module.

Also looking forward for a better implementation in nixpkgs that more gracefully to do so.

## enable `systemd.sysusers` or `services.userborn`

Vaultix using systemd for running on startup or switch (activation).

For reduce code complexity it has no legacy `activationScript`. It meams that you need to using `nixos-24.05` or newer version for these options.
