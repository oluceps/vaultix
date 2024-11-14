## flakeModule Options


The Vaultix configuration option has two parts: in flake level and nixos module level. You need to set both to make it work.


Here is a flake module configuration, it should be written in your flake top-level or in flake module.

Commented options means its default value.

You could find the definition [here](https://github.com/oluceps/vaultix/blob/main/flake-module.nix)

```nix
flake.vaultix = {
  nodes = self.nixosConfigurations;
  identity = "/where/sec/age-yubikey-identity-7d5d5540.txt.pub";
  # extraRecipients = [ ];
  # cache = "./secrets/cache";
};
```

### node =

NixOS systems that allow vaultix to manage. Generally pass `self.nixosConfigurations` will work, if you're using framework like `colmena` that produced unstandard system outputs, you need manually convertion, there always some way. For example, for `colmena`:

```nix
nodes = inherit ((colmena.lib.makeHive self.colmena).introspect (x: x)) nodes;
```


### identity =

`Age identity file`.

Supports age native secrets (recommend protected with passphrase), this could be a:

+ **string**, of absolute path to your local age identity. Thus it can avoid loading identity to nix store.

+ **path**, relative to your age identity in your configuration repository. Note that writting path directly will copy your secrets into nix store, with **Global READABLE**.

This is THE identity that could decrypt all of your secret, take care of it.

> Every `path` in your nix configuration will load file to nix store, eventually shows as string of absolute path to nix store.

Since it inherited great compatibility of `age`, you could use [yubikey](https://github.com/str4d/age-plugin-yubikey). Feel free to test other plugins like [age tpm](https://github.com/Foxboron/age-plugin-tpm). 



### extraRecipients =

Recipients used for backup. Any of identity of them will able to decrypt all secrets, like the `identity`.

> Changing this will not take effect to `renc` command output. The hash of host pub key re-encrypted filename is `blake3(encrypted secret content + host public key)`.

I personally don't recommend setting this.


### cache =

**String** of path that **relative** to flake root, used for storing host public key
re-encrypted secrets. It's default `./secrets/cache`.
