# Vaultix

Secret management for NixOS. Subset replacement of agenix.

## Target
+ Minimal Bash Script Code Size

[!WARNING]
At early development stage, Not ready for production.


## Compatibilities

Inherited Options:

```
config.age.identityPaths      # /persist/keys/ssh_host_ed25519_key
config.age.secrets
config.age.secretsDir         # /run/vaultix
config.age.secretsMountPoint  # /run/vaultix.d
```
