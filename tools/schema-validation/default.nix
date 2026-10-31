{
  python3,
  writeShellApplication,
  requirements-schemas,
}:
let
  python = python3.withPackages (ps: [ ps.jsonschema ]);
in
writeShellApplication {
  name = "requirements-validate";
  text = ''
    exec ${python}/bin/python ${./validate.py} \
      --schema-dir ${requirements-schemas}/share/requirements-framework/schemas "$@"
  '';
}
