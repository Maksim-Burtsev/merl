# The shop's library (#430): what hosts/web.nix imports through ../lib.
{ lib }:
let
  api = "http://localhost:8080";
  tariffRate = 3;
in
{
  mkService = { name, port ? 8080 }: {
#                      ^ d: lib/default.nix:8
    inherit name port;
#            ^ d: lib/default.nix:8
#                 ^ d: lib/default.nix:8
    enable = true;
    endpoint = api;
#               ^ d: lib/default.nix:4
#               status: api: local
  };
  weighParcel = grams: grams * 2;
#                       ^ d: lib/default.nix:18
  "describe-tariff" = rate: "rate ${toString rate}";
#                                             ^ d: lib/default.nix:20
  courier.zone = "north";
  free = tariffRate == 0;
#         ^ d: lib/default.nix:5
  ${zoneName} = 3;
  my-package = lib.mkDefault 1;
#               ^ d: lib/default.nix:2
  rate' = tariffRate + 1;
}
