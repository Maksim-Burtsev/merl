#include "tea/trivet.hh"

namespace pantry {

void Trivet::Set() {
  Stand h = 0;
//^ d: include/tea/trivet.hh:15
}

void Trivet::Place(Stand* s) {
//                 ^ d: include/tea/trivet.hh:15
}

template <typename K>
void Coaster<K>::Lay() {
  Stand* s = nullptr;
//^ d: include/tea/trivet.hh:27
}

template <typename K>
void Coaster<K>::Hold(Stand* s) {}
//                    ^ d: include/tea/trivet.hh:27

void Doily::Fold() {
  Stand* s = nullptr;
//^ d: include/tea/trivet.hh:5
  auto lay = [](Stand* t) { return t; };
//              ^ d: include/tea/trivet.hh:5
}

int Doily::Spread(Stand* s) { return s->legs; }
//                ^ d: include/tea/trivet.hh:5

int Count() {
  Stand s{4};
//^ d: include/tea/trivet.hh:5
  return s.legs;
}

int Kettle::Boil() { return Reading()->Read(); }
//                                     ^ d: include/tea/trivet.hh:46

int Pour() { return Reading()->Read(); }
//                             ^ d: include/tea/trivet.hh:42

}
