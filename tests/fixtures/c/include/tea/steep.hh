#pragma once

#include <array>
#include <map>
#include <vector>

#include "tea/cup.hh"

namespace taste {

struct Leaf {
  int size() const { return 1; }
};

struct Bud {
  int size() const { return 2; }
};

namespace brew {

Mug Steep(int ml);
Mug Pour(int ml);

}  // namespace brew
}  // namespace taste
