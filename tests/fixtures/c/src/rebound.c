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
