#ifndef SHOP_PRICING_H
#define SHOP_PRICING_H

/* A block comment that reads like code declares nothing:
int weigh(int grams) {
struct basket {
*/

#define RATE_CAP 100
#define CENTS(x) ((x) * 100)
#define DECLARE_HOOK(ret, name, hook_args) \
    typedef ret name##_t hook_args; \
    void name(void)

typedef int money_t;
typedef int (*rate_fn)(int);

enum offer { OFFER_PLAIN, OFFER_CUT };

struct bits {
    unsigned whole;
    unsigned half;
};

union tag_value {
    int number;
    char letter;
};

typedef struct {
    int grams;
    int zone;
} parcel;

typedef struct stamp { int flags; } stamp;

extern const char *currency;

int discount(int total);
money_t settle(money_t total);
int tally(void);

#endif
