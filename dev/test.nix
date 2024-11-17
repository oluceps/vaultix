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
    nixosConfigurations = {
      tester = withSystem "x86_64-linux" (
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

                      beforeUserborn = [ "test-secret-2" ];

                      # secret example
                      secrets.test-secret-1 = {
                        file = ./secrets/there-is-a-secret.age;
                        mode = "400";
                        owner = "root";
                        group = "users";
                        # path = "/home/1.txt";
                      };
                      secrets.test-secret-2 = {
                        file = ./secrets/there-is-a-secret.age;
                        mode = "400";
                        owner = "root";
                        group = "users";
                        path = "/home/1.txt";
                      };

                      # template example
                      templates.template-test = {
                        name = "template.txt";
                        content = ''
                          for testing vaultix template ${config.vaultix.placeholder.test-secret-1} nya
                        '';
                        path = "/var/template.txt";
                      };

                    };

                    # for vm testing log
                    systemd.services.vaultix-activate.serviceConfig.Environment = [ "SPDLOG_RS_LEVEL=trace" ];
                  }
                )

                ./configuration.nix
                (
                  { config, pkgs, ... }:
                  {
                    disko.tests = {
                      extraChecks = ''
                        machine.succeed("test -e /run/vaultix.d/0")
                        machine.succeed("test -e /run/vaultix.d/1")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-1.path}")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-2.path}")
                        machine.succeed("test -e ${config.vaultix.templates.template-test.path}")
                        machine.succeed("md5sum -c ${pkgs.writeText "checksum-list" ''
                          2e57c2db0f491eba1d4e496a076cdff7 ${config.vaultix.secrets.test-secret-1.path}
                          2e57c2db0f491eba1d4e496a076cdff7 ${config.vaultix.secrets.test-secret-2.path}
                          ba1efe71bd3d4a9a491d74df5c23e177 ${config.vaultix.templates.template-test.path}
                        ''}")
                      '';
                    };
                  }
                )
              ];
            }
        )
      );
      tester-empty-secret = withSystem "x86_64-linux" (
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

                (_: {
                  services.userborn.enable = true; # or systemd.sysuser, required

                  vaultix = {
                    settings.hostPubkey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEu8luSFCts3g367nlKBrxMdLyOy4Awfo5Rb397ef2AR";

                    beforeUserborn = [ "test-secret-2" ];
                  };

                  # for vm testing log
                  systemd.services.vaultix-activate.serviceConfig.Environment = [ "SPDLOG_RS_LEVEL=trace" ];
                })

                ./configuration.nix
              ];
            }
        )
      );
      tester-empty-template = withSystem "x86_64-linux" (
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

                (_: {
                  services.userborn.enable = true; # or systemd.sysuser, required

                  vaultix = {
                    settings.hostPubkey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEu8luSFCts3g367nlKBrxMdLyOy4Awfo5Rb397ef2AR";

                    beforeUserborn = [ "test-secret-2" ];

                    secrets.test-secret-1 = {
                      file = ./secrets/there-is-a-secret.age;
                      mode = "400";
                      owner = "root";
                      group = "users";
                      # path = "/home/1.txt";
                    };
                  };

                  # for vm testing log
                  systemd.services.vaultix-activate.serviceConfig.Environment = [ "SPDLOG_RS_LEVEL=trace" ];
                })

                ./configuration.nix

                (
                  { config, ... }:
                  {
                    disko.tests = {
                      extraChecks = ''
                        machine.succeed("test -e /run/vaultix.d/0")
                        machine.succeed("test -e /run/vaultix.d/1")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-1.path}")
                      '';
                    };
                  }
                )
              ];
            }
        )
      );
    };
  };
}
