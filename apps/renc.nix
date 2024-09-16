{
  nodes,
  userFlake',
  pkgs,
  system,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (pkgs.lib) concatStringsSep traceVal;
  inherit (builtins) attrValues;

  vaultixs = map (n: n.config.vaultix) (attrValues nodes);
  bin = pkgs.lib.getExe userFlake'.packages.${system}.default;

in
writeShellScriptBin "renc" (
  concatStringsSep "\n" (
    map (
      n:
      let
        a = (pkgs.formats.toml { }).generate "secretsMetadata" n;
      in
      "${bin} ${a} renc"
    ) vaultixs
  )
)
