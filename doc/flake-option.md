## flake Configuration


The Vaultix configuration takes into two parts:

+ flake level setup.

  You can choose either [flakeModule](./flake-module.md) or [pure nix](pure-nix-config.md) in this part.
  it's **recommended** to use `flakeModule`, since it provides type check and more elegant configuration interface.

+ nixos module level setup.


It's required to complete setup in both part to make it work.
