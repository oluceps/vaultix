# NixOS Module Options


Configurable option could be divided into 3 parts:

```nix
# configuration.nix
{
  imports = [ inputs.vaultix.nixosModules.default ];
  vaultix = {
    settings = { ... };
    secrets = { ... };
    templates = { ... };
    needByUser = [...];
  };
}
```

## Settings =
Literally.


<div id="dd"></div>

### decryptedDir: path str

Folder where secrets are symlinked to. Default is `/run/vaultix`.

### decryptedDirForUser: path str

Same as above, but for secrets and templates that required by user, which means needs to be initialize before user born.


<div id="dmp"></div>

### decryptedMountPoint: path str with no trailing slash
default is `/run/vaultix.d`

Where secrets are created before they are symlinked to `vaultix.settings.decryptedDir`

Vaultix use this machenism to implement atomic manage, like other secret managing schemes.

It decrypting secrets into this directory, with generation number like `/run/vaultix.d/1`, then symlink it to `decryptedDir`.

### hostKeys

`{ path: str, type: str }`

default is `config.services.openssh.hostKeys`

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

### hostPubkey: str

ssh public key of the private key, which defines above. This is different from every host, since each generates host key while initial booting.

Get this by: `ssh-keyscan ip`. It supports `ed25519` type.

---

## Secrets = { }

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

### path: path str

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


### content: str


To insert secrets in this string, insert `config.vaultix.placeholder.example`.

This pretend the secret which `id` (the keyof attribute of secrets) was defined.

<div id="id-state"></div>

```nix
secrets = {
  # the id is 'example'. despite `name`.
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

TO BE NOTICE that the source secret file may have trailing `\n`:

### trim: bool

default true;

Removing trailing and leading whitespace by default.


## needByUser: [str]

For deploying secrets and templates that required before user init.

List of [id](#id-state) of templates or secrets.
