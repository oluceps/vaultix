{
  withSystem,
  self,
  inputs,
  ...
}:
{
  flake.nixosConfigurations.tester = withSystem "x86_64-linux" (
    _ctx@{
      config,
      inputs',
      system,
      ...
    }:
    let
      inherit (inputs.nixpkgs) lib;
    in
    lib.nixosSystem {
      modules = [
        ./configuration.nix
        ./UEFI
        (
          { lib, ... }:
          {
            options.test = lib.mkOption {
              type = lib.types.path;
            };
          }
        )
        self.nixosModules.default
        {
          nixpkgs = {
            hostPlatform = lib.mkDefault system;
          };
        }
      ];
    }
  );
}
