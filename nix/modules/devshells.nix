{ ... }:
{
  perSystem = { pkgs, ... }: {
    devShells.default = pkgs.mkShell {
      REQUIREMENTS_SCHEMA_DATA = "${pkgs.requirements-schemas}/share/requirements-framework";
      packages = [
        pkgs.requirements-validator
        (pkgs.python3.withPackages (ps: [ ps.jsonschema ]))
      ];
    };
  };
}
