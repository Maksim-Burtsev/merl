#include "tea/kettle.hh"

namespace infuse {

/*
int Kettle::topup() {
class Teapot {
*/

#if 0
int steep(Leaf leaf) { return 0; }
#endif

static const char* recipe = R"(
int Kettle::decant(int cups) const {
class Urn {
)";

Kettle::Kettle(int spout) : spout_(spout) {}
//                          ^ d: include/tea/kettle.hh:20
//                                 ^ d: src/tea/kettle.cc:19

int Kettle::decant(int cups) const { return cups + spout_ + recipe[0]; }
//          ^ d: include/tea/kettle.hh:14
//                                                          ^ d: src/tea/kettle.cc:14

int Kettle::decant(const char* cups) const { return cups[0]; }

int Kettle::topup() {
  int spout_ = 1;
  return spout_ + this->spout_ + decant(2) + decant("two") + rinse();
  //     ^ d: src/tea/kettle.cc:30
  //                    ^ d: include/tea/kettle.hh:20
  //                             ^ d: src/tea/kettle.cc:23
  //                                         ^ d: src/tea/kettle.cc:27
  //                                                         ^ d: include/tea/kettle.hh:17
}

int steep(Leaf leaf) { return leaf.dose; }
//                                 ^ d: include/tea/kettle.hh:6

}  // namespace infuse

int steep(int dose) { return dose; }
