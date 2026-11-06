# Every construct `f` folds in Nix, and the cases that could break it. # f: none
{ config # f: 2-65
, lib # f: 2-65
, pkgs ? import <nixpkgs> { } # f: 2-65
, ... # f: 2-65
}: # f: 2-65

let # f: 8-65
  inherit (lib) mkOption types; # f: 8-65
  cfg = config.services.merl; # f: 8-65
  url = https://example.org/a?b=c; # f: 8-65
  dirs = [ # f: 12-14
    "/etc" # f: 12-14
    { path = "/var"; } # f: 12-14
  ] ++ lib.optional cfg.enable { # f: 15-17
    path = "/opt"; # f: 15-17
  }; # f: 15-17
  script = '' # f: 18-23
    echo "a } and a ] in a string" # f: 18-23
    echo ''${HOME} '''quoted''' ${toString { # f: 20-22
      a = 1; # f: 20-22
    }} # f: 20-22
  ''; # f: 18-23
  greeting = "a { brace, a # hash and ${lib.concatStringsSep ", " [ # f: 24-27
    "interpolated" # f: 24-27
    "list" # f: 24-27
  ]}"; # f: 24-27
  add = x: y: # f: 28-29
    x + y; # f: 28-29
  wrap = args@{ name, ... }: # f: 30-33
    { # f: 31-33
      inherit name; # f: 31-33
    }; # f: 31-33
in # f: 8-65
{ # f: 35-65
  /* a block comment with { braces # f: 35-65
     over two lines */ # f: 35-65
  options.services.merl = { # f: 38-43
    enable = mkOption { # f: 39-42
      type = types.bool; # f: 39-42
      default = false; # f: 39-42
    }; # f: 39-42
  }; # f: 38-43

  config = lib.mkIf cfg.enable (with pkgs; { # f: 45-55
    environment.systemPackages = [ # f: 46-49
      merl # f: 46-49
      git # f: 46-49
    ]; # f: 46-49
    shell = # f: 45-55
      if cfg.enable # f: 51-54
      then "zsh" # f: 51-54
      else # f: 51-54
        "bash"; # f: 51-54
  }); # f: 45-55

  nested = rec { # f: 57-60
    a = 1; # f: 57-60
    b = a + 1; # f: 57-60
  }; # f: 57-60

  folded = map (x: # f: 62-63
    x + 1 # f: 62-63
  ) [ 1 2 3 ]; # f: 35-65
} # f: 35-65
