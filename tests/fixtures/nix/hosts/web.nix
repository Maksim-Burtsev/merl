{ config, pkgs, lib, ... }:
let
  shopLib = import ../lib { inherit lib; };
#                      ^ d: lib/default.nix:1
#                                    ^ d: hosts/web.nix:1
  api = shopLib.mkService { name = "api"; };
#                ^ d: lib/default.nix:8
#                status: mkService: by name, 1 match
#        ^ d: hosts/web.nix:3
#        status: shopLib: local
  double = x: x * 2;
#              ^ d: hosts/web.nix:11
in
{
  imports = [ ./nginx.nix ];
#                ^ d: hosts/nginx.nix:1
  services.nginx.enable = api.enable;
#                          ^ d: hosts/web.nix:6
#                          status: api: local
#           ^ d: none
  environment.systemPackages = [ pkgs.hello ];
#                                 ^ d: hosts/web.nix:1
#                                 status: pkgs: local
#                                      ^ d: none
  environment.etc = with pkgs; [ hello ];
#                         ^ d: hosts/web.nix:1
  networking.hostName = config.services.nginx.enable;
#                        ^ d: hosts/web.nix:1
#                               ^ d: none
  shop.weight = shopLib.weighParcel (double 2);
#                        ^ d: lib/default.nix:18
#                                     ^ d: hosts/web.nix:11
  shop.describe = shopLib."describe-tariff" 3;
#                           ^ d: lib/default.nix:20
  shop.zone = zoneName;
#              ^ d: none
  shop.rate = shopLib.rate';
#                      ^ d: lib/default.nix:28
  shop.pkg = shopLib.my-package;
#                     ^ d: lib/default.nix:26
  shop.free = tariffRate;
#              ^ d: none
  shop.note = courierNote;
#              ^ d: none
}
