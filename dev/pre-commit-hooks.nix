{
  ...
}:
{
  perSystem =
    { ... }:
    {
      pre-commit = {
        check.enable = true;
        settings.hooks = {
          nixfmt-rfc-style.enable = true;
        };
      };
    };
}
