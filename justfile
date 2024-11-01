set shell := ["nu", "-c"]

pwd := `pwd`

default:
    @just --choose

build-package:
    nix build .
test-metadata:
    nix build '.#nixosConfigurations.tester.config.test'
    open result
eval-json:
    nix eval --json '.#nixosConfigurations.tester.config.test' | jq

init-storage:
    mkdir -p test/secrets/renced/tester
clean-exist-deploy:
    #!/usr/bin/env nu
    sudo umount /run/vaultix.d
    sudo rm -r /run/vaultix.d
    sudo rm -r /run/vaultix
