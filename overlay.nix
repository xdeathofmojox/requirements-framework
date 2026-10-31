final: _prev: {
  requirements-schemas = final.callPackage ./schemas { };
  requirements-viewer-rust = final.callPackage ./tools/requirements-viewer-rust { };
  requirements-viewer = final.callPackage ./tools/requirements-viewer { };
  requirements-validator = final.callPackage ./tools/schema-validation { };
}
