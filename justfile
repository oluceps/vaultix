set shell := ["nu", "-c"]

default:
    @just --choose

build-package:
    nix build .
test-metadata:
    nix eval --json .#nixosConfigurations.tester.config.test | str replace --all '"' '' | open $in
eval-hostKeys:
    nix eval --expr $"builtins.mapAttrs \(_: v: v.config.services.openssh.hostKeys) \(builtins.getFlake \"(pwd)\").nixosConfigurations" --impure --json | jq
