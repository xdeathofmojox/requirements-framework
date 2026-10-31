{
  lib,
  python3Packages,
  requirements-schemas,
}:
python3Packages.buildPythonApplication {
  pname = "requirements-validator";
  version = (lib.importTOML ./pyproject.toml).project.version;
  pyproject = true;
  src = ./.;
  build-system = [ python3Packages.setuptools ];
  dependencies = with python3Packages; [
    jsonschema
    referencing
  ];
  pythonImportsCheck = [ "requirements_validator.validate" ];
  makeWrapperArgs = [
    "--add-flags"
    "--schema-dir ${requirements-schemas}/share/requirements-framework/schemas"
  ];
  meta.mainProgram = "requirements-validate";
}
