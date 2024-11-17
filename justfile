set shell := ["nu", "-c"]

pwd := `pwd`

default:
    @just --choose

build-package:
    nix build .

clean-exist-deploy:
    #!/usr/bin/env nu
    sudo umount /run/vaultix.d
    sudo rm -r /run/vaultix.d
    sudo rm -r /run/vaultix
full-test:
    #!/usr/bin/env nu
    cargo test
    cargo fuzz run --all-features fuzz_parser_1 -- -max_len=256 -runs=16777216
    nix run github:nix-community/nixos-anywhere -- --flake .#tester --vm-test
vm-tests:
    #!/usr/bin/env nu
    ["tester" "tester-empty-secret" "tester-empty-template"] |
      par-each { nix run github:nix-community/nixos-anywhere -- --flake $'.#($in)' --vm-test }
