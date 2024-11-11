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
  recipientsArg = concatStringsSep " " extraRecipients;

in
writeShellScriptBin "edit-secret" "${bin} edit --identity ${identity} --recipients ${recipientsArg} $1"
