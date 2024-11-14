# Frequent Asked Questions

1. rebooting deploy failed with could not found ssh private key, but it indeed just there.

   Check if using `root on tmpfs`, and modify [hostKeys](https://oluceps.github.io/vaultix/nixos-option.html#hostkeys) path to Absolute path string to your REAL private key location (not bind mounted or symlinked etc.)
