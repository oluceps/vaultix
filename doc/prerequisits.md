# Prerequisits

It basically require:

+ `nix-command` feature enabled
+ `flake-parts` structured config
+ `self` as one of `specialArgs` for nixosSystem
+ `systemd.sysusers` or `services.userborn` option enabled

See following for reason and details.

---

> enable `nix-command` `flakes` features

Which is almost every user will do.

Vaultix depends on nix flake and nix apps to perform basic function.

```nix
nix.settings = {
  experimental-features = [
    "nix-command"
    "flakes"
  ];
}
```

---

> `flake-parts` structured config

[flake-parts](https://flake.parts/) provides modulized flake config, vaultix using [flake module](https://github.com/oluceps/vaultix/blob/main/flake-module.nix) to produce nix apps and hidding complexity.

---

> `self` as one of `specialArgs` for nixosSystem

For passing top-level flake arguments to nixos module.

This requirement may change in the future, with backward compatiblility. Looking forward for a better implementation in nixpkgs that more gracefully to do so.

---

> enable `systemd.sysusers` or `services.userborn`

`sysusers` was comes with [Perlless Activation](https://github.com/NixOS/nixpkgs/pull/270727).

`userborn` was introduced in [Aug 30 2024](https://github.com/NixOS/nixpkgs/pull/332719)

Vaultix using systemd instead of old perl script for activating on system startup or switch. It meams that you need on `nixos-24.05` or newer version for using it.
