# Workflows
Common used workflow with vaultix.

## Add new secret


### 1. Run edit:

```bash
nix run .#vaultix.app.x86_64-linux.edit -- ./where/new-to-add.age
```

### 2. Add a secret to nixos module:

```nix
secrets = {
  #...
  new-to-add.file = ./where/new-to-add.age;
};
```

### 3. Run renc:


```bash
nix run .#vaultix.app.x86_64-linux.renc
```

### 4. Add new produced stuff to git.



## Modify existed secret


```bash
nix run .#vaultix.app.x86_64-linux.edit -- ./where/to-edit.age
```

```bash
nix run .#vaultix.app.x86_64-linux.renc
```

Then add changes to git.

## Remove secret


```diff
secrets = {
  #...
-  new-to-add.file = ./where/new-to-add.age;
};
```

```bash
rm ./where/new-to-add.age
```

```bash
nix run .#vaultix.app.x86_64-linux.renc
```
Then add changes to git.
