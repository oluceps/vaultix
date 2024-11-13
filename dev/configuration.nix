# NOTICE: All configuration in this file is just for testing
# Not part of vaultix deploy requirements
{
  inputs,
  modulesPath,
  pkgs,
  ...
}:
{
  imports = [
    inputs.disko.nixosModules.disko
    ./UEFI.nix
    (modulesPath + "/profiles/qemu-guest.nix")
  ];
  # WARN: This is just for testing and demostrating, you SHOULD NOT set this option
  # WARN: This is just for testing and demostrating, you SHOULD NOT set this option
  # WARN: This is just for testing and demostrating, you SHOULD NOT set this option
  services.openssh.hostKeys = [
    {
      path = pkgs.writeText "UNSAFE-SSH-PRIVATE-KEY" ''
        -----BEGIN OPENSSH PRIVATE KEY-----
        b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
        QyNTUxOQAAACBLvJbkhQrbN4N+u55Sga8THS8jsuAMH6OUW9/e3n9gEQAAAJA2pUBZNqVA
        WQAAAAtzc2gtZWQyNTUxOQAAACBLvJbkhQrbN4N+u55Sga8THS8jsuAMH6OUW9/e3n9gEQ
        AAAECY8KpFz3qti09XPK9+gNe1hiBe/KF8tVI+se0e+e1QMEu8luSFCts3g367nlKBrxMd
        LyOy4Awfo5Rb397ef2ARAAAAC2VsZW5Aa2FhbWJsAQI=
        -----END OPENSSH PRIVATE KEY-----
      '';
      type = "ed25519";
    }
  ];

  boot.kernelPackages = pkgs.linuxPackages_latest;

  users.mutableUsers = false;

  systemd.network.enable = true;
  services.resolved.enable = true;

  systemd.network.networks.eth0 = {
    matchConfig.Name = "eth0";
    DHCP = "yes";
  };

  networking.useNetworkd = true;

  networking.hostName = "tester";

  system.stateVersion = "24.05";
}
