# setup

You could also find the minimal complete nixos configuration on [CI VM test](https://github.com/oluceps/vaultix/tree/main/dev).

### Layout Preview

```nix
{
  withSystem,
  self,
  inputs,
  ...
}:
{
  flake = {

    vaultix = {
      nodes = self.nixosConfigurations;
      identity = "/home/who/key";
    };

    nixosConfigurations.host-name = withSystem "x86_64-linux" ({ system, ... }:
      inputs.nixpkgs.lib.nixosSystem (
          {
            inherit system;
            specialArgs = {
              inherit self; # Required
            };
            modules = [
              inputs.oluceps.nixosModules.vaultix # import nixosModule

              (
                { config, ... }:
                {
                  services.userborn.enable = true; # or systemd.sysuser, required

                  vaultix = {
                    settings.hostPubkey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEu8luSFCts3g367nlKBrxMdLyOy4Awfo5Rb397ef2BC";
                    secrets.test-secret-1 = {
                      file = ./secrets/there-is-a-secret.age;
                    };
                  };
                }
              )
              ./configuration.nix
            ];
          }
      )
    );
  };
}
```
And you will be able to use secret on any module with, e.g.:

```
{
  services.proxy1.environmentFile = config.vaultix.secrets.example.path;
}
# ...
```
