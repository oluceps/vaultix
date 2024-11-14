{ pkgs, ... }:
{
  disko.tests = {
    extraChecks = ''
      machine.succeed("test -e /run/vaultix.d/0")
      machine.succeed("test -e /run/vaultix.d/0/test-secret-1")
      machine.succeed("test -e /run/vaultix/test-secret-1")
      machine.succeed("test -e /var/template.txt")
      machine.succeed("md5sum -c ${pkgs.writeText "checksum-list" ''
        2e57c2db0f491eba1d4e496a076cdff7 /run/vaultix/test-secret-1
        ba1efe71bd3d4a9a491d74df5c23e177 /var/template.txt
      ''}")
    '';
  };
}
