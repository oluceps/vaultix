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
    just vm-tests
vm-tests:
    #!/usr/bin/env nu
    nix run github:nix-community/nixos-anywhere -- --flake .#tester --vm-test
    nix run github:nix-community/nixos-anywhere -- --flake .#tester-empty-secret --vm-test
    nix run github:nix-community/nixos-anywhere -- --flake .#tester-empty-template --vm-test
