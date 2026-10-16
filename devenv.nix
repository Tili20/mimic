{
  pkgs,
  lib,
  config,
  ...
}: {
  # devenv.sh/languages/
  languages = {
    rust.enable = true;
  };

  # devenv.sh/packages/
  packages = [
    pkgs.gtk4
    pkgs.pkg-config
  ];

  # See full reference at https://devenv.sh/reference/options/
}
