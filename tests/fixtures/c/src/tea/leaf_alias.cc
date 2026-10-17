#include "tea/kettle.hh"

using Leaf = int;

struct Cosy {
  int fold(int n);
  int fold(infuse::Leaf l);
};

int alias_leaf(Cosy cosy) {
  Leaf x = 1;
  return steep(x) + cosy.fold(x);
  //     ^ d: !jump
  //                     ^ d: picker src/tea/leaf_alias.cc:6, src/tea/leaf_alias.cc:7
}
