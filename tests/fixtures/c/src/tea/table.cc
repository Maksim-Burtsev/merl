#include "tea/kettle.hh"

int stale::steep(int dose) { return dose + 1; }

int set_table() {
  infuse::Leaf leaf{3};
  infuse::Kettle near(1);
  stale::Kettle far;
  //     ^ d: include/tea/kettle.hh:41
  infuse::Teapot pot;
  return steep(leaf) + steep(4) + stale::steep(5) + near.decant(1) + far.decant(1) + pot.decant(1);
  //     ^ d: src/tea/kettle.cc:39
  //                   ^ d: src/tea/kettle.cc:44
  //                                     ^ d: src/tea/table.cc:3
  //                                                     ^ d: src/tea/kettle.cc:23
  //                                                                     ^ d: include/tea/kettle.hh:43
  //                                                                                     ^ d: include/tea/kettle.hh:25
}

int shade() {
  infuse::Kettle cup(2);
  auto f = [](infuse::Teapot cup) { return cup.decant(1); };
  //                                           ^ d: include/tea/kettle.hh:25
  return f(infuse::Teapot{}) + cup.decant(1) + cup.topup();
  //                               ^ d: src/tea/kettle.cc:23
  //                                               ^ d: src/tea/kettle.cc:29
}

namespace bt = infuse;

int once_more() {
  bt::Kettle k(3);
  //  ^ d: include/tea/kettle.hh:11
  bt::Urn urn;
  return k.topup() + urn.serve() + urn.rinse() + Tin<int>{}.seal();
  //                     ^ d: include/tea/kettle.hh:31
  //                                   ^ d: include/tea/kettle.hh:34
  //                                                        ^ d: include/tea/kettle.hh:52
}

using infuse::steep;

int steep_again() {
  infuse::Leaf leaf{1};
  return steep(leaf);
  //     ^ d: src/tea/kettle.cc:39
}

int cosy() {
  infuse::Urn urn;
  //      ^ d: include/tea/kettle.hh:28
  return urn.topup();
  //         ^ d: src/tea/kettle.cc:29
}
