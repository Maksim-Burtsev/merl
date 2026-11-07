let
  depot = 1;
  stock = import ./lib;
in
{
  shelf = (let depot = 2; in depot) + depot;
#                                     ^ d: rebound.nix:2
  crate = (depot: depot) depot;
#                        ^ d: rebound.nix:2
  pallet = map (depot: depot * 2) [ depot ];
#                                   ^ d: rebound.nix:2
  bay = { left = depot; right = depot: depot; };
#                ^ d: rebound.nix:2
#                                      ^ d: rebound.nix:12
  dock = rec { depot = 3; ramp = depot; };
#                                ^ d: rebound.nix:15
  yard = rec {
    depot = 3;
    ramp = depot;
#          ^ d: rebound.nix:18
  };
  gate = rec { inherit (stock) depot; ramp = depot; };
#                                            ^ d: rebound.nix:22
  plain = { depot = 3; ramp = depot; };
#                             ^ d: rebound.nix:2
  hall = let depot = 2; in depot;
#                          ^ d: rebound.nix:26
  lane = depot: depot;
#               ^ d: rebound.nix:28
  wall = depot;  # depot: depot
#        ^ d: rebound.nix:2
  roof = "${depot}";
#           ^ d: rebound.nix:2
  pkg = rec {
    depot = 4;
#   ^ d: rebound.nix:35
    src = fetch {
      url = "${depot}";
#              ^ d: rebound.nix:35
      depot = 5;
#     ^ d: !jump
    };
    passthru.tests = {
      depot = 6;
#     ^ d: !jump
    };
    ramp = depot;
#          ^ d: rebound.nix:35
  };
}
