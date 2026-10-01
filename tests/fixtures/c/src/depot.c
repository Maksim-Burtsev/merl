#include "shop/depot.h"
#include "shop/twin_a.h"

crate *crate_open(int n)
{
    return 0;
}

int stacked(crate *c, hatch *h, struct twin *t)
{
    int n = c->flags;
    //         ^ d: include/shop/depot.h:19
    // status: via c: crate
    n += c->lid->seal.flags;
    //                ^ d: include/shop/depot.h:6
    // status: via c: crate → lid: lid → seal: seal
    n += c->stacks[1]->flags;
    //                 ^ d: include/shop/depot.h:15
    // status: via c: crate → stacks: pallet
    n += yard.flags;
    //        ^ d: include/shop/depot.h:15
    // status: via yard: pallet
    n += h->flags;
    //      ^ d: src/hatch.c:4
    // status: via h: hatch
    n += t->seal->flags;
    //            ^ d: picker include/shop/depot.h:6, include/shop/depot.h:10, include/shop/depot.h:15, include/shop/depot.h:19, src/hatch.c:4, include/shop/pricing.h:35, src/depot.c:35
    // status: chain broke at t
    return n + crate_open(1)->flags;
    //                        ^ d: include/shop/depot.h:19
    // status: via crate_open(): crate
}

static struct dock {
    int flags;
} dock;

int docked(void)
{
    return dock.flags;
    //          ^ d: src/depot.c:35
    // status: via dock: dock
}
