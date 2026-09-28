#include <vector>
#include "shop/offers.hh"
#include "shop/forward.hh"

namespace shop {

class Basket : public Priced {
//                    ^ d: include/shop/offers.hh:24
 public:
  int price() const override { return tariff_.rate() + coupon_.rate(); }
  //                                  ^ d: none; want src/checkout.cc:16 (#378)
  //                                          ^ d: picker include/shop/offers.hh:11, include/shop/offers.hh:20; want include/shop/offers.hh:11 (#389)
  //                                                           ^ d: picker include/shop/offers.hh:11, include/shop/offers.hh:20; want include/shop/offers.hh:20 (#389)

 private:
  Tariff tariff_{1};
//^ d: picker include/shop/forward.hh:4, include/shop/offers.hh:8, include/shop/offers.hh:10; want include/shop/offers.hh:8 (#368)
  Coupon coupon_;
//^ d: include/shop/offers.hh:18
};

int total(const Tariff& t, const Coupon& c) {
  return t.rate() + c.rate();
  //       ^ d: picker include/shop/offers.hh:11, include/shop/offers.hh:20; want include/shop/offers.hh:11 (#389)
  //                  ^ d: picker include/shop/offers.hh:11, include/shop/offers.hh:20; want include/shop/offers.hh:20 (#389)
}

std::string label(const Tariff& t) {
  return t.describe();
  //       ^ d: picker include/shop/offers.hh:21, src/offers.cc:5; want src/offers.cc:5 (#389)
}

int charge(Priced* p) { return p->price(); }
//                                ^ d: src/checkout.cc:10; want include/shop/offers.hh:26 (#373)

void wipe(std::vector<int>* v) { v->clear(); }
//                                  ^ d: include/shop/offers.hh:43; want none (#389)

Offer pick() { return Offer::Cut; }
//^ d: include/shop/offers.hh:29
//                           ^ d: none; want include/shop/offers.hh:29 (#373)

Box<int> boxed;
//^ d: picker include/shop/offers.hh:32, include/shop/offers.hh:37
Cents cents = 3;
//^ d: include/shop/offers.hh:50
Ledger ledger;
//^ d: include/shop/offers.hh:41
Label tag;
//^ d: include/shop/offers.hh:46

int weight() {
  Tariff rnd(4);
  return rnd.rate() + shop::Basket(rnd).price();
  //     ^ d: none; want src/checkout.cc:53 (#378)
  //                  ^ d: picker src/checkout.cc:5, …
  //                        ^ d: picker src/checkout.cc:7, src/offers.cc:8; want src/checkout.cc:7 (#465)
}

}  // namespace shop
