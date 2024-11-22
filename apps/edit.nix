{
  pkgs,
  package,
  identity,
  extraRecipients,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (pkgs.lib) concatStringsSep;

  bin = pkgs.lib.getExe package;
  recipientsArg = concatStringsSep " " (map (n: "--recipient ${n}") extraRecipients);

in
writeShellScriptBin "edit-secret" "${bin} edit --identity ${identity} ${recipientsArg} $1"
