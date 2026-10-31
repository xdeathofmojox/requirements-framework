{ ... }:
{
  perSystem = { pkgs, ... }: {
    packages = {
      inherit (pkgs)
        requirements-schemas
        requirements-validator
        requirements-viewer
        ;
      default = pkgs.requirements-validator;
    };
  };
}
