{
  withSystem,
  self,
  inputs,
  ...
}:
{
  flake = {
    vaultix = {
      # minimal works configuration
      nodes = self.nixosConfigurations;
      identity = "/home/elen/unsafe-test";

      cache = "./dev/secrets/cache"; # relative to the flake root.
    };

    nixosConfigurations.tester = withSystem "x86_64-linux" (
      {
        system,
        ...
      }:
      with inputs.nixpkgs;
      lib.nixosSystem (
        lib.warn
          "THIS SYSTEM IS ONLY FOR TESTING, If this msg appears in production there MUST be something wrong."
          {
            inherit system;
            specialArgs = {
              inherit
                self # Required
                inputs
                ;
            };
            modules = [
              self.nixosModules.vaultix

              (
                { config, ... }:
                {
                  services.userborn.enable = true; # or systemd.sysuser, required

                  vaultix = {
                    settings.hostPubkey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEu8luSFCts3g367nlKBrxMdLyOy4Awfo5Rb397ef2AR";

                    # secret example
                    secrets.test-secret-1 = {
                      file = ./secrets/there-is-a-secret.age;
                      mode = "400";
                      owner = "root";
                      group = "users";
                    };

                    # template example
                    templates.template-test = {
                      name = "template.txt";
                      content = ''
                        for testing vaultix template ${config.vaultix.placeholder.test-secret-1} nya
                      '';
                      # path = "/home/user/template.txt";
                    };

                  };

                  # for vm testing log
                  systemd.services.vaultix-install-secrets.serviceConfig.Environment = [ "SPDLOG_RS_LEVEL=trace" ];
                }
              )

              ./configuration.nix
            ];
          }
      )
    );
  };
}
