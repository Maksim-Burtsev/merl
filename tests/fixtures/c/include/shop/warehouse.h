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

typedef int (*courier_cmp)(const courier_t, struct courier);
//                               ^ d: include/shop/warehouse.h:9
typedef int (*courier_eq)(const courier_t, struct courier);
//                                                ^ d: include/shop/warehouse.h:4

#endif
