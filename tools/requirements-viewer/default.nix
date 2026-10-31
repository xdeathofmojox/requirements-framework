{
  python3,
  writeShellApplication,
  requirements-validator,
}:
writeShellApplication {
  name = "requirements-view";
  text = ''
    exec ${python3}/bin/python ${./.}/viewer.py \
      --validator ${requirements-validator}/bin/requirements-validate "$@"
  '';
}
