#include "shop/attic.h"

#if 0
static int dust_off(int box) { return box; }
#define ATTIC_SHELVES 2
#else
static int brush(int box) { return box; }
#endif

int climb(void) {
  return dust_off(1) + ATTIC_SHELVES + brush(2);
  //     ^ d: src/attic.c:3
  //                   ^ d: include/shop/attic.h:5
  //                                   ^ d: src/attic_use.c:7
}

#if 0
int old_climb(void) {
  int rung = 1;
  return rung;
  //     ^ d: src/attic_use.c:19
}
#endif
