#include "shop/stock.h"

#define STOCK_MIN 0

static int stock_seen = 0;

int stock_level(int shelf)
//  ^ d: picker include/shop/stock.h:13
//  status: at a declaration, 1 other by name
{
    return stock_clamp(shelf) + stock_seen;
}

#ifdef STOCK_FAST
int stock_count(void) { return 1; }
#else
int stock_count(void) { return 2; }
#endif
