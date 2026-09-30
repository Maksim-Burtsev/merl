#include "shop/offers.hh"

namespace shop {

std::string Tariff::describe() const { return "tariff"; }

const char *kBanner = R"(
struct Basket {
int weigh(int grams) {
)";

}  // namespace shop

namespace shop {

// A raw string runs to `)`, its delimiter and `"`; the `)"` inside it closes nothing (#465).
const char *kQuery = u8R"sql(
  select ")" from parcels;
struct Spare {
)sql";

int Unwrapped() { return sizeof kQuery; }

int Tally() {
  Spare *spare = nullptr;
//^ d: none
  return Unwrapped() + (spare != nullptr);
  //     ^ d: src/offers.cc:22
}

// An out-of-line method is its class's by the qualifier on its line, not `Coupon`'s (#508).
auto describer = &Tariff::describe;
//                        ^ d: src/offers.cc:5

}  // namespace shop
