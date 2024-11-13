{
  disko.tests = {
    extraChecks = ''
      machine.succeed("test -e /run/vaultix.d/0")
      machine.succeed("test -e /run/vaultix.d/0/test-secret-1")
      machine.succeed("test -e /run/vaultix/test-secret-1")
      machine.succeed("test -e /var/template.txt")
    '';
  };
}
