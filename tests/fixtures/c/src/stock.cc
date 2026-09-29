#include "shop/stock.hh"

namespace shop {

int stock_tag(const char *, int n) { return n; }

namespace {
int stock_pick() { return 1; }
}  // namespace

}  // namespace shop
