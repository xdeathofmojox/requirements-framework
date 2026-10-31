{ ... }:
{
  perSystem = { pkgs, ... }: {
    packages = {
      inherit (pkgs) requirements-schemas requirements-validator;
      default = pkgs.requirements-validator;
    };
  };
}
