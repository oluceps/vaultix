# Advanced




## Tricks

In most cases you don't need these.

### Manually deploy

This must be executed on local, and be sure all secrets re-encrypted before that, since there has no module to guarantee it in this case.

Manually deploy not affect next vaultix activation. It's a trick that helps you finish deploy while your flake options of vaultix broken:

This eval nixos vaultix configs to json.

```bash
nix eval .#nixosConfigurations.your-hostname.config.vaultix-debug --json > profile.json
```

So that you can feed it to vaultix cli directly:

```bash
nix run github:oluceps/vaultix -- -p ./profile.json deploy
```

To be notice that deploy secrets that needs to be extracted before user init (deploy with --early) in this way is meaningless.
