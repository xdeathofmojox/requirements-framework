{ ... }:
{
  perSystem = { pkgs, ... }: {
    checks.rust-viewer = pkgs.requirements-viewer-rust;
    checks.viewer = pkgs.callPackage ../../tools/requirements-viewer/tests { };
    checks.schemas = pkgs.callPackage ../../tools/schema-validation/tests { };
  };
}
