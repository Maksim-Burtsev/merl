#include "deflate.h"

local int slide(int n) { return n + 1; }

local int send_bits(int n) { return n; }

ZEXTERN int flush_block(int n) { return n; }

zlocal int pqdownheap(int n) { return n; }

int deflate(void) { return slide(1) + send_bits(2); }
//                         ^ d: src/zl/deflate.c:3
//                                    ^ d: src/zl/deflate.c:5
