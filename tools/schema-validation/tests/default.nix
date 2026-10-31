{
  python3,
  runCommand,
  requirements-schemas,
}:
runCommand "requirements-schema-tests"
  {
    nativeBuildInputs = [ (python3.withPackages (ps: [ ps.jsonschema ])) ];
    REQUIREMENTS_SCHEMA_DATA = "${requirements-schemas}/share/requirements-framework";
  }
  ''
    cd ${../.}
    PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -s tests -v
    touch $out
  ''
