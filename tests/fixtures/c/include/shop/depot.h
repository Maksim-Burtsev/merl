#ifndef SHOP_DEPOT_H
#define SHOP_DEPOT_H

/* Receivers typed by their declarations (#386): each `flags` below is another struct's. */
struct seal {
    int flags;
};

struct lid {
    int flags;
    struct seal seal;
};

struct pallet {
    int flags;
};

typedef struct crate {
    int flags;
    struct lid *lid;
    struct pallet **stacks;
} crate;

/* Its body is in src/hatch.c alone. */
typedef struct hatch hatch;

extern struct pallet yard;

crate *crate_open(int n);

#endif
