set shell := ["nu", "-c"]

default:
    @just --choose

test-metadata:
     #!/usr/bin/env nu
     nix eval --raw .#nixosConfigurations.tester.config.test --write-to ./test.json
     cat ./test.json | jq .
     rm ./test.json
