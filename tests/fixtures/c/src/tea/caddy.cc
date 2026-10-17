#include "tea/caddy.hh"

namespace caddy {
int scoop(int spoons) { return spoons; }

int brew() {
  return scoop(6);
  //     ^ d: src/tea/caddy.cc:4
}
}  // namespace caddy

int scoop(int spoons) { return -spoons; }

int brew_outside() {
  return scoop(6);
  //     ^ d: src/tea/caddy.cc:12
}
