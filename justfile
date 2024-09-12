set shell := ["nu", "-c"]

default:
    @just --choose

build-package:
    nix build .
test-metadata:
    nix eval --json .#nixosConfigurations.tester.config.test | str replace --all '"' '' | open $in
