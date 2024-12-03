## flake Configuration


The Vaultix configuration takes into two parts:

+ flake level setup.

  You can choose [flakeModule](./flake-module.md) OR [pure nix](pure-nix-config.md) in this part.
  it's more recommend to use `flakeModule`, since it provides type check and more elegant configuration interface.

+ nixos module level setup.

  Some of flake level config passthrough into this by setting specialArgs.


You need to complete both to make it work.
