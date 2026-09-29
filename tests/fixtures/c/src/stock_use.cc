#include "shop/stock.hh"

namespace shop {

int stock_pick() { return 2; }

int stock_run() {
  stock_note(1);
//^ d: picker include/shop/stock.hh:4, include/shop/stock.hh:5
  return stock_pick() + stock_tag("a");
  //     ^ d: src/stock_use.cc:5
  //                    ^ d: src/stock.cc:5
}

}  // namespace shop
