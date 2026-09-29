#ifndef SHOP_STOCK_H
//      ^ d: include/shop/stock.h:3
#define SHOP_STOCK_H

#ifndef STOCK_MAX
#define STOCK_MAX 100
#endif

#if !defined(stock_round)
#define stock_round(x) (x)
#endif

int stock_level(int shelf);
static inline int stock_clamp(int n) { return n < 0 ? 0 : n; }
int stock_count(void);

#endif
