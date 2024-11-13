{
  nodes,
  lib,
  pkgs,
  package,
  identity,
  cache,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (lib) concatStringsSep foldlAttrs;
  bin = pkgs.lib.getExe package;

  rencCmds = foldlAttrs (
    acc: name: value:

    let
      profile = pkgs.writeTextFile {
        name = "secret-meta-${name}";
        text = builtins.toJSON value.config.vaultix;
      };
    in
    acc
    ++ [
      "${bin} --profile ${profile} renc --identity ${identity} --cache ${
        cache + "/" + value.config.networking.hostName
      }"
    ]
  ) [ ] nodes;

in
writeShellScriptBin "renc" (concatStringsSep "\n" rencCmds)
