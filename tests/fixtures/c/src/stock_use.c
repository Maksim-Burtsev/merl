#include "shop/stock.h"
#include "shop/round.h"

#define STOCK_MIN 1

int stock_seen(void) { return 0; }

int stock_report(void)
{
    return stock_level(3) + STOCK_MAX + STOCK_MIN + stock_count() + stock_clamp(2);
    //     ^ d: src/stock.c:7
    //     status: stock_level: by name, 1 definition, 1 prototype
    //                      ^ d: include/shop/stock.h:6
    //                                  ^ d: src/stock_use.c:4
    //                                              ^ d: picker src/stock.c:15, src/stock.c:17, include/shop/stock.h:15
    //                                                                ^ d: include/shop/stock.h:14
}

int stock_rounded(void)
{
    return stock_round(stock_seen());
    //     ^ d: src/round.c:3
    //                 ^ d: src/stock_use.c:6
}
