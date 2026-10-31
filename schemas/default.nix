{ lib, runCommand }:
let
  version = import ./version.nix;
  versions = lib.filterAttrs (
    name: type: type == "directory" && builtins.match "v[1-9][0-9]*" name != null
  ) (builtins.readDir ./.);
in
runCommand "requirements-schemas-${version}" { inherit version; } ''
  mkdir -p $out/share/requirements-framework/schemas
  cp -r ${./examples} $out/share/requirements-framework/examples
  ${lib.concatMapStringsSep "\n" (name: ''
    cp -r ${./. + "/${name}"} $out/share/requirements-framework/schemas/${name}
  '') (builtins.attrNames versions)}
''
