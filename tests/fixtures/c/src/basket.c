#include "shop/pricing.h"
//             ^ d: !jump
#include "shop/warehouse.h"

#define WEIGHT_LIMIT 30

int limit = 5;
int hook_args = 0;

struct basket {
    int items;
    parcel box;
 // ^ d: include/shop/pricing.h:33
};

int gross(struct basket *b, rate_fn fn)
//               ^ d: src/basket.c:10
//                          ^ d: include/shop/pricing.h:16
{
    return discount(fn(b->items)) + CENTS(RATE_CAP);
    //     ^ d: src/pricing.c:5
    //                    ^ d: src/basket.c:11
    //                              ^ d: include/shop/pricing.h:10
    //                                    ^ d: include/shop/pricing.h:9
}

money_t bonus(enum offer o, struct bits *bits, union tag_value *v)
//^ d: include/shop/pricing.h:15
//                 ^ d: include/shop/pricing.h:18
//                                 ^ d: include/shop/pricing.h:20
//                                                   ^ d: include/shop/pricing.h:25
{
    return o == OFFER_CUT ? settle(bits->whole) : v->number;
    //          ^ d: include/shop/pricing.h:18
    //                      ^ d: src/pricing.c:11
    //                                   ^ d: include/shop/pricing.h:21
    //                                               ^ d: picker include/shop/invoice.hh:8, include/shop/pricing.h:26; want include/shop/pricing.h:26 (#386)
}

int restock(int discount)
{
    return discount + WEIGHT_LIMIT;
    //     ^ d: src/basket.c:40
    //                ^ d: src/basket.c:5
}

int overweight(int grams)
{
    int limit = WEIGHT_LIMIT + 20;
    return weigh(grams) > limit;
    //     ^ d: src/warehouse.c:4
    //                    ^ d: src/basket.c:49
}

const char *dispatch(void)
{
    courier_t *courier = courier_new("post");
 // ^ d: include/shop/warehouse.h:9
    //                   ^ d: src/warehouse.c:9
    return courier->name;
    //              ^ d: include/shop/warehouse.h:5
}

int counted(stamp *s)
//          ^ d: include/shop/pricing.h:35
{
    return tally() + s->flags + hook_args;
    //     ^ d: src/pricing.c:16
    //                  ^ d: include/shop/pricing.h:35
    //                          ^ d: picker src/basket.c:8, include/shop/pricing.h:12; want src/basket.c:8 (#382)
}

const char *money(void)
{
    return currency;
    //     ^ d: src/pricing.c:3
}

int width(route *r)
//        ^ d: include/shop/warehouse.h:16
{
    return r->end - r->start;
}

int span(struct route *r)
//              ^ d: include/shop/warehouse.h:14
{
    return r->end - r->start;
}
