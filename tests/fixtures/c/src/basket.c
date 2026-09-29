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
    //     ^ d: picker include/shop/pricing.h:39, src/pricing.c:5; want src/pricing.c:5 (#364)
    //                    ^ d: none; want src/basket.c:11 (#359)
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
    //          ^ d: none; want include/shop/pricing.h:18 (#373)
    //                      ^ d: picker include/shop/pricing.h:40, src/pricing.c:11; want src/pricing.c:11 (#364)
    //                                   ^ d: none; want include/shop/pricing.h:21 (#359)
    //                                               ^ d: none; want include/shop/pricing.h:26 (#359)
}

int restock(int discount)
{
    return discount + WEIGHT_LIMIT;
    //     ^ d: picker include/shop/pricing.h:39, src/pricing.c:5; want src/basket.c:40 (#378)
    //                ^ d: src/basket.c:5
}

int overweight(int grams)
{
    int limit = WEIGHT_LIMIT + 20;
    return weigh(grams) > limit;
    //     ^ d: picker include/shop/warehouse.h:11, src/warehouse.c:4; want src/warehouse.c:4 (#364)
    //                    ^ d: src/basket.c:7; want src/basket.c:49 (#378)
}

const char *dispatch(void)
{
    courier_t *courier = courier_new("post");
 // ^ d: include/shop/warehouse.h:9
    //                   ^ d: picker include/shop/warehouse.h:12, src/warehouse.c:9; want src/warehouse.c:9 (#364)
    return courier->name;
    //              ^ d: none; want include/shop/warehouse.h:5 (#359)
}

int counted(stamp *s)
//          ^ d: include/shop/pricing.h:35
{
    return tally() + s->flags + hook_args;
    //     ^ d: picker include/shop/pricing.h:41, src/pricing.c:16, src/warehouse.c:16; want src/pricing.c:16 (#364)
    //                  ^ d: include/shop/pricing.h:35
    //                          ^ d: picker src/basket.c:8, include/shop/pricing.h:12; want src/basket.c:8 (#382)
}

const char *money(void)
{
    return currency;
    //     ^ d: picker include/shop/pricing.h:37, src/pricing.c:3; want src/pricing.c:3 (#364)
}

int width(route *r)
//        ^ d: picker include/shop/warehouse.h:14, include/shop/warehouse.h:16; want include/shop/warehouse.h:16 (#368)
{
    return r->end - r->start;
}
