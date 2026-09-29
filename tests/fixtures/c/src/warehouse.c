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
    //     ^ d: src/warehouse.c:16
    // status: by name
}
