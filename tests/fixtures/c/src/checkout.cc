#include <vector>
#include "shop/offers.hh"
#include "shop/forward.hh"

namespace shop {

class Basket : public Priced {
//                    ^ d: include/shop/offers.hh:24
 public:
  int price() const override { return tariff_.rate() + coupon_.rate(); }
  //                                  ^ d: src/checkout.cc:16
  //                                          ^ d: include/shop/offers.hh:11
  //                                                           ^ d: include/shop/offers.hh:20

 private:
  Tariff tariff_{1};
//^ d: include/shop/offers.hh:8
  Coupon coupon_;
//^ d: include/shop/offers.hh:18
};

int total(const Tariff& t, const Coupon& c) {
  return t.rate() + c.rate();
  //       ^ d: include/shop/offers.hh:11
  //                  ^ d: include/shop/offers.hh:20
}

std::string label(const Tariff& t) {
  return t.describe();
  //       ^ d: src/offers.cc:5
}

int charge(Priced* p) { return p->price(); }
//                                ^ d: include/shop/offers.hh:26

void wipe(std::vector<int>* v) { v->clear(); }
//                                  ^ d: none

Offer pick() { return Offer::Cut; }
//^ d: include/shop/offers.hh:29
//                           ^ d: include/shop/offers.hh:29

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
  //     ^ d: src/checkout.cc:53
  //                  ^ d: picker src/checkout.cc:5, …
  //                        ^ d: src/checkout.cc:7
}

int voucher(Voucher* v) { return v->rate(); }
//                                  ^ d: include/shop/offers.hh:20

int held(std::unique_ptr<Tariff> t) { return t->rate(); }
//                                              ^ d: include/shop/offers.hh:11

int till(Till* t) { return t->tariff.rate(); }
//                                   ^ d: include/shop/offers.hh:11
//                                   status: rate → Tariff::rate (via t: Till → tariff: Tariff)

int guess() {
  auto t = make();
  return t.rate();
  //       ^ d: picker include/shop/offers.hh:11, include/shop/offers.hh:20
}

namespace sh = shop;
int aliased(sh::Tariff* t) { return t->rate(); }
//                                     ^ d: include/shop/offers.hh:11

int maybe(std::optional<Tariff> o) { return o->rate(); }
//                                             ^ d: picker include/shop/offers.hh:11, include/shop/offers.hh:20

int walked(std::vector<Tariff>::iterator it) { return it->rate(); }
//                                                        ^ d: picker include/shop/offers.hh:11, include/shop/offers.hh:20

int racked(Rack& r) { return r[0].tariff.rate(); }
//                                       ^ d: picker include/shop/offers.hh:11, include/shop/offers.hh:20

void drop(std::vector<Tariff>* ts) { ts->clear(); }
//                                       ^ d: none

void reset(std::optional<std::string> s) { s->clear(); }
//                                            ^ d: none

}  // namespace shop
