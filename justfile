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
