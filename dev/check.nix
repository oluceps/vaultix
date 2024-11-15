{ pkgs, config, ... }:
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
