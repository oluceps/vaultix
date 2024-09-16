set shell := ["nu", "-c"]

pwd := `pwd`

default:
    @just --choose

build-package:
    nix build .
test-metadata:
    nix build '.#nixosConfigurations.tester.config.test'
    open result
eval-hostKeys:
    nix eval --expr "builtins.mapAttrs (_: v: v.config.services.openssh.hostKeys) (builtins.getFlake "{{ pwd }}").nixosConfigurations" --impure --json | jq
eval-json:
    nix eval --json '.#nixosConfigurations.tester.config.test' | jq
