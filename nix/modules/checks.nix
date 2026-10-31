{ ... }:
{
  perSystem = { pkgs, ... }: {
    checks.viewer = pkgs.requirements-viewer;
    checks.schemas =
      pkgs.callPackage ../../tools/requirements-validator/src/requirements_validator/tests
        { };
  };
}
