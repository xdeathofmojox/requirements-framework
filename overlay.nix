final: _prev: {
  requirements-schemas = final.callPackage ./schemas { };
  requirements-viewer = final.callPackage ./applications/requirements-viewer { };
  requirements-validator = final.callPackage ./tools/requirements-validator { };
}
