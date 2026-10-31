{
  lib,
  rustPlatform,
  makeWrapper,
  nodejs,
  requirements-validator,
}:
rustPlatform.buildRustPackage {
  pname = "requirements-viewer";
  version = (lib.importTOML ../../Cargo.toml).workspace.package.version;
  src = lib.cleanSourceWith {
    src = ../..;
    filter = path: type: baseNameOf path != "target" && lib.cleanSourceFilter path type;
  };
  cargoLock.lockFile = ../../Cargo.lock;
  cargoBuildFlags = [
    "-p"
    "requirements-viewer"
  ];
  cargoTestFlags = [
    "-p"
    "requirements-viewer"
  ];
  nativeBuildInputs = [ makeWrapper ];
  nativeCheckInputs = [ nodejs ];
  preCheck = ''
    node --check applications/requirements-viewer/web/theme.js
    node --check applications/requirements-viewer/web/app.js
    node --check applications/requirements-viewer/web/schema-form.js
    node --test applications/requirements-viewer/tests/view.test.js applications/requirements-viewer/tests/schema-form.test.js
  '';
  REQUIREMENTS_VALIDATOR = "${requirements-validator}/bin/requirements-validate";
  postFixup = ''
    wrapProgram $out/bin/requirements-viewer \
      --set REQUIREMENTS_VALIDATOR ${requirements-validator}/bin/requirements-validate
  '';
  meta.mainProgram = "requirements-viewer";
}
