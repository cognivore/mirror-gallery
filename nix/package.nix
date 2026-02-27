{ pkgs }:

let
  script = ../src/mirror-gallery.rs;
in
pkgs.writeShellApplication {
  name = "mirror-gallery";
  runtimeInputs = with pkgs; [
    rust-script
    rustc
    cargo
    gh
    git
  ];
  text = ''
    exec rust-script ${script} "$@"
  '';
}
