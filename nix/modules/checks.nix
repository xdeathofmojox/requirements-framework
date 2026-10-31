{ ... }:
{
  perSystem = { pkgs, ... }: {
    checks.viewer = pkgs.requirements-viewer;
    checks.schemas = pkgs.callPackage ../../tools/schema-validation/tests { };
  };
}
