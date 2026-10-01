# The shop's flake (#430): an attribute `pkgs` that the parameter `pkgs` of hosts/web.nix is not.
{
  description = "shop";
  outputs = { self, nixpkgs }: {
#                    ^ d: flake.nix:4
    pkgs = nixpkgs.legacyPackages.aarch64-darwin;
#           ^ d: flake.nix:4
#           status: nixpkgs: local
    lib = import ./lib { lib = nixpkgs.lib; };
#                   ^ d: lib/default.nix:1
    hosts.web = import ./hosts/web.nix;
#                               ^ d: hosts/web.nix:1
    shell = import ./shell.nix;
  };
}
