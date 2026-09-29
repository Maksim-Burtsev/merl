#ifndef SHOP_WAREHOUSE_H
#define SHOP_WAREHOUSE_H

struct courier {
    const char *name;
    int zone;
};

typedef struct courier courier_t;

int weigh(int grams);
courier_t *courier_new(const char *name);

typedef struct route {
    int start, end;
} route;

#endif
