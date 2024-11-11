{
  nodes,
  pkgs,
  package,
  identity,
  extraRecipients,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (pkgs.lib) concatStringsSep;
  inherit (builtins) attrValues;

  vaultixs = map (n: n.config.vaultix) (attrValues nodes);
  bin = pkgs.lib.getExe package;
  recipientsArg = concatStringsSep " " extraRecipients;

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
      "${bin} ${profile} renc --identity ${identity} --recipient ${recipientsArg}"
    ) vaultixs
  )
)
