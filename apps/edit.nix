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
  recipientsArg =
    if extraRecipients != [ ] then "--recipients" + (concatStringsSep " " extraRecipients) else "";

in
writeShellScriptBin "edit-secret" "${bin} edit --identity ${identity} ${recipientsArg} $1"
