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
    lib.nixosSystem (
      lib.warn
        "THIS SYSTEM IS ONLY FOR TESTING, If u meet this msg in production there must be something wrong."
        {
          specialArgs = {
            inherit self;
          };
          pkgs = import inputs.nixpkgs {
            inherit system;
            config = { };
            overlays = [ self.overlays.default ];
          };
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
    )
  );
}
