{
  withSystem,
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
      specialArgs =
        {
        };
      modules = [
        ./configuration.nix
        ./UEFI
        {
          nixpkgs = {
            hostPlatform = lib.mkDefault system;
          };
        }
      ];
    }
  );
}
