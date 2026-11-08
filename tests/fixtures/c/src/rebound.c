struct gourd { int slosh; };
struct terrine { int slosh; };
void block(struct gourd *g) { { struct terrine *g = 0; g->slosh = 1; } g->slosh = 1; }
//                                                        ^ d: src/rebound.c:2
//                                                                        ^ d: src/rebound.c:1
void before(struct gourd *g) { g->slosh = 1; { struct terrine *g = 0; g->slosh = 1; } }
//                                ^ d: src/rebound.c:1
//                                                                       ^ d: src/rebound.c:2
void loop(struct gourd *g) { for (struct terrine *g = 0; g; ) g->slosh = 1; g->slosh = 1; }
//                                                               ^ d: src/rebound.c:2
//                                                                             ^ d: src/rebound.c:1
void value(void) { struct gourd g; { struct terrine g; g.slosh = 1; } g.slosh = 1; }
//                                                       ^ d: src/rebound.c:2
//                                                                      ^ d: src/rebound.c:1
void closed(struct gourd *g) { { struct terrine *g = 0; } g->slosh = 1; }
//                                                           ^ d: src/rebound.c:1
void paren(struct gourd *g) { { struct terrine (*g) = 0; g->slosh = 1; } }
//                                                          ^ d: src/rebound.c:2
int deref(struct gourd *g);
void called(struct gourd *g) { deref(g); g->slosh = 1; }
//                                          ^ d: src/rebound.c:1
#define CDECLARE(T, n) T n = 0
#define CSHOW(n) (void)(n)
void cdeclared(struct gourd *g) { { CDECLARE(struct terrine *, g); g->slosh = 1; } g->slosh = 1; }
//                                                                    ^ d: picker src/rebound.c:1, src/rebound.c:2
//                                                                                    ^ d: src/rebound.c:1
void cshown(struct gourd *g) { { CSHOW(g); g->slosh = 1; } }
//                                            ^ d: src/rebound.c:1
void tureen(struct gourd *g) { { typedef struct terrine Tureen; Tureen *g = 0; g->slosh = 1; } }
//                                                                                ^ d: src/rebound.c:2
typedef struct gourd Ladle;
void ladle(void) { typedef struct terrine Ladle; Ladle *g = 0; g->slosh = 1; }
//                                                                ^ d: src/rebound.c:2
void ladle_outer(void) { Ladle *g = 0; g->slosh = 1; }
//                                        ^ d: src/rebound.c:1
void ladle_closed(void) { { typedef struct terrine Ladle; } Ladle *g = 0; g->slosh = 1; }
//                                                                           ^ d: src/rebound.c:1
void ladle_late(void) { Ladle *g = 0; typedef struct terrine Ladle; g->slosh = 1; }
//                                                                     ^ d: src/rebound.c:1
void ladle_ptr(void) { typedef struct terrine *Scoop; Scoop g = 0; g->slosh = 1; }
//                                                                    ^ d: src/rebound.c:2
#define CLOCK(m) lock(&m)
#define CCLEAR(x) memset(&x, 0, sizeof x)
#define CPEEK(p) peek(*p)
#define CMAX(a, b) if (b > a) a = b
void cread(struct gourd *g, struct gourd *h) { { CLOCK(g); g->slosh = 1; CCLEAR(g); g->slosh = 1; } }
//                                                            ^ d: src/rebound.c:1
//                                                                                     ^ d: src/rebound.c:1
void cpeek(struct gourd *g, struct gourd *h) { { CPEEK(g); g->slosh = 1; CMAX(g, h); g->slosh = 1; } }
//                                                            ^ d: src/rebound.c:1
//                                                                                      ^ d: src/rebound.c:1
#include "shop/declare.h"
void hdeclared(struct gourd *g) { { HDECLARE(struct terrine *, g); g->slosh = 1; } g->slosh = 1; }
//                                                                    ^ d: picker src/rebound.c:1, src/rebound.c:2
//                                                                                    ^ d: src/rebound.c:1
void hshown(struct gourd *g) { { HSHOW(g); g->slosh = 1; } }
//                                            ^ d: src/rebound.c:1
