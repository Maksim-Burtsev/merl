#include "shop/depot.h"

// C++ keeps master's lookup of `x->word`: the struct rule of #386 reads `.c` and `.h` alone.
int crate_flags(crate *c)
{
    return c->flags;
    //        ^ d: picker include/shop/depot.h:6, include/shop/depot.h:10, include/shop/depot.h:15, include/shop/depot.h:19, include/shop/pricing.h:35, src/depot.c:35, src/hatch.c:4
}

