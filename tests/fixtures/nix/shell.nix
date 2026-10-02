let
  pkgs = import <nixpkgs> { };
  api = "http://localhost:9090";
#  ^ d: shell.nix:3
  drv = pkgs.hello // { postInstall = ''
    courierNote = "fragile";
    escaped = '''quoted''' and ''${notCode} and ''\n;
    weighParcel = grams: grams;
  ''; };
in
pkgs.mkShell {
#^ d: shell.nix:2
  API_URL = api;
#            ^ d: shell.nix:3
#            status: api: local
  packages = [ drv ];
#               ^ d: shell.nix:5
  shellHook = ''
    zoneName=north
    courierNote = 1;
    export SHOP_API="${api}"
#                       ^ d: shell.nix:3
  '';
  /* A block comment:
  courierNote = 2;
  */
  # tariffRate = 3;
}
