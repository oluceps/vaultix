# Pure Nix Configuration


> [!NOTE]
> If you completed setup with [flakeModule](./flake-module.md), you can skip to [nixos module setup](nixos-option.md)


The option is identical to [flakeModule](./flake-module.md), but different way to perform nix app producing.


```nix
vaultix = vaultix.configure {

  localFlake = self; # different from flakeModule way

  # identical
  nodes = self.nixosConfigurations;
  identity = self + "/age-yubikey-identity-deadbeef.txt.pub";
  extraRecipients = [ ];
  cache = "./secret/.cache";
};
```

---

In this way your flake will looks like:

```nix
{
  description = "An Example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    vaultix.url = "github:milieuim/vaultix";
  };

  outputs = { self, nixpkgs, ... }@inputs: {
    nixosConfigurations.my-nixos = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";

      specialArgs = {
        inherit self;
      };

      modules = [
        inputs.vaultix.nixosModules.default
        ./configuration.nix
      ];
      # ...
    };
    vaultix = vaultix.configure {
    
      localFlake = self; # different from flakeModule way
    
      # identical
      nodes = self.nixosConfigurations;
      identity = self + "/age-yubikey-identity-deadbeef.txt.pub";
      extraRecipients = [ ];
      cache = "./secret/.cache";
    };
  };
}
```
