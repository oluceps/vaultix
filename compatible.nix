# This is for compatibility of (Pure Nix without flake-parts framework) users
# This configuration method lacks of option type check, which based on flake-parts module system.
{
  withSystem,
  inputs,
  self,
  ...
}:
{
  flake.configure =
    {
      localFlake,
      nodes,
      cache ? "./secrets/cache",
      identity,
      extraRecipients ? [ ],
    }:
    let
      inherit (inputs.nixpkgs) lib;
    in
    {
      # for nixosSystem
      inherit cache;

      app = lib.mapAttrs (
        system: config':
        lib.genAttrs
          [
            "renc"
            "edit"
          ]
          (
            app:
            import ./apps/${app}.nix {
              inherit
                nodes
                identity
                extraRecipients
                cache
                lib
                ;
              inherit (withSystem system ({ pkgs, ... }: pkgs))
                pkgs
                ;
              package = self.packages.${system}.default;
            }
          )
      ) localFlake.allSystems;
    };
}
