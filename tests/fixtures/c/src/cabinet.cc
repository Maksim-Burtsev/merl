#include "shop/beacon.hh"

namespace shop {

unsigned long Cabinet::Count() const { return 0; }
//                     ^ d: include/shop/beacon.hh:24

void Cabinet::Fill(int items, int boxes) {}
//            ^ d: include/shop/beacon.hh:25

int Cabinet::Lock(int code) { return code; }
//           ^ d: picker include/shop/beacon.hh:27, include/shop/beacon.hh:45

bool seen(Beacon* s) { return s->Ready(); }
//        ^ d: picker include/shop/beacon.hh:5, include/shop/beacon.hh:33
//                               ^ d: picker include/shop/beacon.hh:9, include/shop/beacon.hh:14, include/shop/beacon.hh:19, include/shop/beacon.hh:40

unsigned long stocked(Cabinet* d) {
  d->Fill(1, 2);
  // ^ d: src/cabinet.cc:8
  return d->Count();
  //        ^ d: src/cabinet.cc:5
}

int sweep(int k) {
  Cabinet probe(k, 2);
  return k;
}

int again() { return probe(3); }
//                   ^ d: src/count.cc:3

}  // namespace shop
