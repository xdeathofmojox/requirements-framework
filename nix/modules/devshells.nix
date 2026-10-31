{ ... }:
{
  perSystem = { pkgs, ... }: {
    devShells.default = pkgs.mkShell {
      REQUIREMENTS_VALIDATOR = "${pkgs.requirements-validator}/bin/requirements-validate";
      REQUIREMENTS_SCHEMA_DATA = "${pkgs.requirements-schemas}/share/requirements-framework";
      packages = [
        pkgs.cargo
        pkgs.rustc
        pkgs.rustfmt
        pkgs.clippy
        pkgs.requirements-viewer
        pkgs.requirements-validator
        (pkgs.python3.withPackages (ps: [ ps.jsonschema ]))
      ];
    };
  };
}
