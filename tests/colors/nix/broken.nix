# Broken on purpose: an open interpolation, list and string.
{ pkgs, ... }:
let
  a = "unterminated ${pkgs.hello;
  b = [ 1 2
in
{
  c = a +;
  d = ''
    open
}
