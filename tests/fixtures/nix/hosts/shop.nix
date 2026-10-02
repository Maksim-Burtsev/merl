# A shop host (#430): a header over lines, a sibling `let`, and declarations in prose.
{
  config,
  pkgs-unstable,
  pkgs,
  ...
}:
let
  cfg = config.shop;
  motd = "welcome
  courierZone = 1;
  ";
in
{
  x = let
    cfg = config.other;
  in cfg.port;
#     ^ d: hosts/shop.nix:16
  y = cfg.enable;
#      ^ d: hosts/shop.nix:9
  z = pkgs.hello;
#      ^ d: hosts/shop.nix:5
#      status: pkgs: local
  # tariffZone = 2; inherit courierNote;
  note = "see { courierZone = 3; }";
  w = courierZone;
#      ^ d: none
  v = tariffZone;
#      ^ d: none
}
