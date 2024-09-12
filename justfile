set shell := ["nu", "-c"]

default:
    @just --choose

test-metadata:
     #!/usr/bin/env nu
     nix eval --json .#nixosConfigurations.tester.config.test | str replace --all '"' '' | open $in
