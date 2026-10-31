{ ... }:
{
  perSystem = { pkgs, ... }: {
    checks.schemas = pkgs.callPackage ../../tools/schema-validation/tests { };
  };
}
