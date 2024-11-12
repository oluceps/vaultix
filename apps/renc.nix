{
  nodes,
  pkgs,
  package,
  identity,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (pkgs.lib) concatStringsSep;
  inherit (builtins) attrValues;

  vaultixs = map (n: n.config.vaultix) (attrValues nodes);
  bin = pkgs.lib.getExe package;

in
writeShellScriptBin "renc" (
  concatStringsSep "\n" (
    map (
      n:
      let
        profile = pkgs.writeTextFile {
          name = "secret-meta";
          text = builtins.toJSON n;
        };
      in
      "${bin} --profile ${profile} renc --identity ${identity}"
    ) vaultixs
  )
)
