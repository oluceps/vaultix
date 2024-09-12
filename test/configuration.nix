{
  lib,
  pkgs,
  modulesPath,
  ...
}:
{
  vaultix = {
    secrets = {
      factorio-server = {
        file = ./secret/server.age;
        mode = "640";
        owner = "factorio";
        group = "users";
        name = "factorio-server";
      };
      factorio-admin = {
        file = ./secret/admin.age;
        mode = "640";
        owner = "factorio";
        group = "users";
        name = "factorio-admin";
      };
    };
  };

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
