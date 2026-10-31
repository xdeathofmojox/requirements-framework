{ inputs, ... }:
{
  imports = [ inputs.treefmt-nix.flakeModule ];
  perSystem =
    { ... }:
    {
      treefmt.config = {
        projectRootFile = "flake.nix";

        programs = {
          clang-format.enable = true;
          mdformat.enable = true;
          nixfmt.enable = true;
          ruff-format.enable = true;
          jsonfmt.enable = true;
          shellcheck.enable = true;
          shfmt.enable = true;
          toml-sort = {
            enable = true;
            excludes = [ "**/Cargo.toml" ];
          };
        };

        settings.global.excludes = [ ];
      };
    };
}
