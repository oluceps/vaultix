# Development

## DevShell

```bash
nix develop
```

## Test

For testing basic functions with virtual machine:

```bash
nix run github:nix-community/nixos-anywhere -- --flake .#tester --vm-test
```

Run full test with `just full-test`

## Format

This repo follows `nixfmt-rfc-style` style, reformat with running `nixfmt .`.

## Lint

Lint with statix.
