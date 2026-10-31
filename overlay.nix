final: _prev: {
  requirements-schemas = final.callPackage ./schemas { };
  requirements-validator = final.callPackage ./tools/schema-validation { };
}
