{
  python3,
  nodejs,
  runCommand,
  requirements-validator,
}:
runCommand "requirements-viewer-tests"
  {
    nativeBuildInputs = [
      python3
      nodejs
    ];
    REQUIREMENTS_VALIDATOR = "${requirements-validator}/bin/requirements-validate";
  }
  ''
    cd ${../.}
    node --check web/app.js
    PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -s tests -v
    touch $out
  ''
