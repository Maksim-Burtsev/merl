int trees(void) { return slide(3) + send_bits(4); }
//                       ^ d: picker src/zl/window.c:3, src/zl/window.h:1
//                                  ^ d: none

int build_tree(void) { return flush_block(5) + pqdownheap(6); }
//                            ^ d: src/zl/deflate.c:7
//                                             ^ d: src/zl/deflate.c:9
