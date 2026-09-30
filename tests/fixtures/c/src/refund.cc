#include "shop/refund.hh"
#include "shop/invoice.hh"

namespace shop {

Refund::Refund() : cents_(0) {}

struct Drawer::Scanner {
//             ^ d: include/shop/refund.hh:23
  int code;
};

Refund issue(int cents) {
  Refund r;
//^ d: include/shop/refund.hh:5
  if (!r.ok()) return Refund::Partial(cents);
  //                          ^ d: include/shop/refund.hh:9
  //                          status: via Refund
  return Refund(cents);
  //     ^ d: picker include/shop/refund.hh:5, include/shop/refund.hh:8, src/refund.cc:6
}

int Drawer::count() {
  Scanner* first = nullptr;
//^ d: picker include/shop/refund.hh:23, src/refund.cc:8
  return first ? 1 : 0;
}

Slot* slot;
//^ d: picker include/shop/refund.hh:22, include/shop/invoice.hh:7
Aisle* aisle;
//^ d: picker include/shop/refund.hh:16, include/shop/refund.hh:28, include/shop/invoice.hh:5

// The qualifier written on a declaration's line is its owner (#508).
Drawer::Scanner scanner;
//      ^ d: src/refund.cc:8

}  // namespace shop
