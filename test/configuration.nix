{
  lib,
  pkgs,
  modulesPath,
  config,
  ...
}:
{
  vaultix = {
    settings = {
      # re-encryption cipher file store, related to flake root
      storageDirRelative = "./test/secrets/renced/${config.networking.hostName}";
      hostPubkey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEu8luSFCts3g367nlKBrxMdLyOy4Awfo5Rb397ef2AR elen@kaambl";
      masterIdentities = [
        # This indeed not safe since it has not password protection ;
        # ./safekey.txt.pub
        ./age-yubikey-identity-7d5d5540.txt.pub

        # "/somewhere/priv"

        # {
        # This key indeed not safe since it has not password protection ;
        #   identity = ./safekey.txt.pub;
        #   # Optional; This has the same type as `age.rekey.hostPubkey`
        #   # and allows explicit association of a pubkey with the identity.
        #   pubkey = "age1zhwnp754d2puu28tjmhqchfp4ukecxhtulx26nsxeey65ez9cu8qk3295c";
        # }
      ];
    };
    secrets = {
      factorio-server = {
        file = ./secrets/server.age;
        mode = "640";
        owner = "factorio";
        group = "users";
        name = "factorio-server";
      };
      factorio-admin = {
        file = ./secrets/admin.age;
        mode = "640";
        owner = "factorio";
        group = "users";
        name = "factorio-admin";
      };
    };
  };
  services.openssh.hostKeys = [
    {
      # temporary, and insecure.
      path = ./ssh_key;
      type = "ed25519";
    }
  ];

  services.userborn.enable = true;

  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

  time.timeZone = "America/Los_Angeles";
  networking.nameservers = [ "8.8.8.8" ];
  boot.kernelPackages = pkgs.linuxPackages_latest;
  users.allowNoPasswordLogin = true;
  users.mutableUsers = false;

  systemd.network.enable = true;
  services.resolved.enable = true;

  systemd.network.networks.eth0 = {
    matchConfig.Name = "eth0";
    DHCP = "yes";
  };

  services.openssh = {
    enable = true;
    ports = [ 22 ];
    settings = {
      PasswordAuthentication = false;
      PermitRootLogin = lib.mkForce "prohibit-password";
    };
  };

  networking.firewall.enable = false;

  networking.useNetworkd = true;

  networking.hostName = "tester";

  system.stateVersion = "24.05";
}
