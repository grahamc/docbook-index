with import <nixpkgs> {};
pkgs.mkShell {
  buildInputs = [ cargo carnix jq ];
}
