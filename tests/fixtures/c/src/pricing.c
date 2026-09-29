#include "shop/pricing.h"

const char *currency = "EUR";

int discount(int total)
{
    return total < RATE_CAP ? total - 1 : RATE_CAP;
}

money_t
settle(money_t total)
{
    return total;
}

int tally(void) { return 1; }
