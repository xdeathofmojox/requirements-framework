{
  lib,
  rustPlatform,
  makeWrapper,
  nodejs,
  requirements-validator,
}:
rustPlatform.buildRustPackage {
  pname = "requirements-viewer-rust";
  version = (lib.importTOML ./Cargo.toml).package.version;
  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type: baseNameOf path != "target" && lib.cleanSourceFilter path type;
  };
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  nativeCheckInputs = [ nodejs ];
  preCheck = ''
    node --check web/theme.js
    node --check web/app.js
    node --check web/schema-form.js
    node --test tests/view.test.js tests/schema-form.test.js
  '';
  REQUIREMENTS_VALIDATOR = "${requirements-validator}/bin/requirements-validate";
  postFixup = ''
    wrapProgram $out/bin/requirements-viewer-rust \
      --set REQUIREMENTS_VALIDATOR ${requirements-validator}/bin/requirements-validate
  '';
  meta.mainProgram = "requirements-viewer-rust";
}
