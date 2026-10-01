enum shelf_state {
    // The order is stored (do not change it).
    SHELF_EMPTY,
    SHELF_FULL = 4,
    SHELF_LOW, SHELF_GONE
};

/* The bin's state, as the scanner reads it
 * (open or shut). */
typedef enum
{
    BIN_OPEN,
    BIN_SHUT
} bin_state;

int shelf_codes[] = { SHELF_EMPTY, SHELF_FULL };
//                    ^ d: src/shelf.c:3
//                                 ^ d: src/shelf.c:4

int shelf_order[] = {
    SHELF_LOW,
    SHELF_GONE
};

int shelf_check(enum shelf_state s)
{
    return s == SHELF_GONE ? BIN_SHUT : SHELF_LOW;
    //          ^ d: src/shelf.c:5
    //                       ^ d: src/shelf.c:13
    //                                  ^ d: src/shelf.c:5
}
