#include "shop/depot.h"

// C++ reads a C struct's field through the receiver's declaration too (#389).
int crate_flags(crate *c)
{
    return c->flags;
    //        ^ d: include/shop/depot.h:19
}

