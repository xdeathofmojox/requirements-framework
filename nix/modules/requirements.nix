{ inputs, ... }:
let
  requirementsModule =
    { flake-parts-lib, ... }:
    {
      options.perSystem = flake-parts-lib.mkPerSystemOption (
        {
          config,
          pkgs,
          lib,
          ...
        }:
        {
          options.requirements = {
            documents = lib.mkOption {
              type = lib.types.listOf lib.types.path;
              default = [ ];
              description = "Requirement documents to validate during nix flake check.";
            };
            schema = lib.mkOption {
              type = lib.types.nullOr lib.types.path;
              default = null;
              description = "Optional project schema composed with the framework core.";
            };
            schemaVersion = lib.mkOption {
              type = lib.types.nullOr (lib.types.strMatching "v[1-9][0-9]*");
              default = null;
              description = "Bundled schema version; null uses v1. Mutually exclusive with schema.";
            };
          };
          config.checks = lib.mkIf (config.requirements.documents != [ ]) {
            requirements =
              assert lib.assertMsg (
                config.requirements.schema == null || config.requirements.schemaVersion == null
              ) "requirements.schema and requirements.schemaVersion are mutually exclusive";
              pkgs.runCommand "requirements-validation" { } ''
                ${
                  inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.requirements-validator
                }/bin/requirements-validate \
                  ${
                    lib.optionalString (
                      config.requirements.schemaVersion != null
                    ) "--schema-version ${lib.escapeShellArg config.requirements.schemaVersion}"
                  } \
                  ${
                    lib.optionalString (
                      config.requirements.schema != null
                    ) "--schema ${lib.escapeShellArg "${config.requirements.schema}"}"
                  } \
                  ${lib.escapeShellArgs (map (path: "${path}") config.requirements.documents)}
                touch $out
              '';
          };
        }
      );
    };
in
{
  imports = [ requirementsModule ];
  flake.flakeModules.default = requirementsModule;
}
