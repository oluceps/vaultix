# Nix Apps

Provided user friendly cli tools:


## renc

This is needed every time the host key or secret content changed.

The wrapped vaultix will decrypt cipher content to plaintext and encrypt it with target host public key, finally stored in `cache`.

```bash
nix run .#vaultix.app.x86_64-linux.renc
```

## edit

This will decrypt and open file with `$EDITOR`. Will encrypt it after editing finished.

```bash
nix run .#vaultix.app.x86_64-linux.edit -- ./secrets/some.age
```

