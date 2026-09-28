#include <stdlib.h>
#include "shop/warehouse.h"

int weigh(int grams)
{
    return grams / 1000;
}

courier_t *courier_new(const char *name)
{
    courier_t *c = malloc(sizeof *c);
    c->name = name;
    return c;
}

static int tally(void) { return 2; }

int restocked(void)
{
    return tally();
    //     ^ d: picker include/shop/pricing.h:41, src/pricing.c:16, src/warehouse.c:16; want src/warehouse.c:16 (#364)
    // status: by name
}
