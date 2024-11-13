{
  description = "partition. tests and dev cfg for vaultix";

  inputs = {
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
    };

    # for create system for testing
    disko = {
      url = "github:nix-community/disko";
    };

  };

  outputs = _: { };
}
