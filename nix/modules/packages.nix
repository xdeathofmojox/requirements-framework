{ ... }:
{
  perSystem = { pkgs, ... }: {
    packages = {
      inherit (pkgs)
        requirements-schemas
        requirements-validator
        requirements-viewer
        requirements-viewer-rust
        ;
      default = pkgs.requirements-validator;
    };
  };
}
