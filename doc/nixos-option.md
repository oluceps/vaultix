# NixOS Module Options


This is in nixosConfiguration, produced by `nixosSystem` function. Any confusion about this please refer to [our test system config](https://github.com/milieuim/vaultix/blob/7b14c948e7d7a8e86ed5ee4c5927c588ee344db7/dev/test.nix#L16) and [unofficial doc](https://nixos-and-flakes.thiscute.world/nixos-with-flakes/nixos-flake-configuration-explained)

Configurable option could be divided into 3 parts:

```nix
# in configuration.nix etc.
{
  imports = [ inputs.vaultix.nixosModules.default ];
  vaultix = {
    settings = {
      hostPubkey = "ssh-ed25519 AAAA..."; # required
      # ...
    };
    secrets = { };
    templates = { };
    beforeUserborn = [ ];
  };
}
```

## Settings
Literally.


<div id="dd"></div>

### decryptedDir

+ type: `string of absolute path`
+ default: `/run/vaultix`

Folder where secrets are symlinked to.

### decryptedDirForUser

+ type: `string of absolute path`
+ default: `/run/vaultix-for-user`

Same as above, but for secrets and templates that required by user, which means needs to be initialize before user born.


<div id="dmp"></div>

### decryptedMountPoint

+ type: `string of absolute path`
+ default: `/run/vaultix.d`

Path str with no trailing slash

Where secrets are created before they are symlinked to `vaultix.settings.decryptedDir`

Vaultix use this machenism to implement atomic manage, like other secret managing schemes.

It decrypting secrets into this directory, with generation number like `/run/vaultix.d/1`, then symlink it to `decryptedDir`.

### hostKeys

+ type: `{ path: str, type: str }`
+ default: `config.services.openssh.hostKeys`

This generally has no need to manually change, unless you know clearly what you're doing.

Ed25519 host private ssh key (identity) path that used for decrypting secrets while deploying.

format:

```nix
[
  {
    path = "/path/to/ssh_host_ed25519_key";
    type = "ed25519";
  }
]
```

### hostPubkey

+ type: `(string of pubkey) or (path of pubkey file)`

example:

```nix
hostPubkey = "ssh-ed25519 AAAAC3Nz....."
# or
hostPubkey = ./ssh_host_ed25519_key.pub # no one like this i think
```

ssh public key of the hostKey. This is different from every host, since each generates host key while initial booting.

Get this of remote machine by: `ssh-keyscan ip`. It supports `ed25519` type.

You could find it in `/etc/ssh/` next to host ssh private key, with `.pub` suffix.

This could be either literal string or path, the previous one is more recommended.

---

## Secrets

Here is a secrets:
```nix
secrets = {
  example = {
    file = ./secret/example.age;
  };
};
```
The secret is expected to appear in `/run/vaultix/` with `0400` and own by uid0.

Here is full options that configurable:

```nix
secrets = {
  example = {
    file = ./secret/example.age;
    mode = "640"; # default 400
    owner = "root";
    group = "users";
    name = "example.toml";
    path = "/some/place";
  };
};
```

This part basically keeps identical with `agenix`. But has few diffs:

+ no `symlink: bool` option, since it has an systemd function called [tmpfiles.d](https://www.freedesktop.org/software/systemd/man/latest/tmpfiles.d.html).

### path

+ type: `absolute path string`

If you manually set this, it will deploy to specified location instead of to `/run/vaultix.d` (default value of [decryptedMountPoint](#dmp)).

If you still set the path to directory to `/run/vaultix` (default value of [decryptedDir](#dd)), you will receive a warning, because you should use the `name` option instead of doing that.


## Templates

`Vaultix` provides templating function. This makes it able to insert secrets content into plaintext config while deploying.

Overview of this option:

```nix
templates = {
  test-template = {
    name = "template.txt";
    content = "this is a template for testing ${config.vaultix.placeholder.example}";
    trim = true;

    # permission options like secrets
    mode = "640"; # default 400
    owner = "root";
    group = "users";
    name = "example.toml";
    path = "/some/place";
  };
}
```


### content


Insert `config.vaultix.placeholder.example` in plain string content.

This expects the `placeholder.<*>` identical with defined secret `id` (the keyof it).

<div id="id-state"></div>

```nix
secrets = {
  # the id is 'example' here.
  example = {
    file = ./secret/example.age;
  };
};
```

The content could also be multiline:
```nix
''
this is a template for testing ${config.vaultix.placeholder.example}
this is another ${config.vaultix.placeholder.what}
${config.vaultix.placeholder.some} here
''
```

> [!NOTE]
> Source secret text may have trailing `\n`, if you don't want automatically remove it please see [trim](#trim):

### trim

+ type: `bool`
+ default: `true`;

Removing trailing and leading whitespace by default.


## beforeUserborn

+ type: `list of string`

For deploying secrets and templates that required before user init.

List of [id](#id-state) of templates or secrets.

example:

```nix
beforeUserborn = ["secret1" "secret2" "template1"];
```
