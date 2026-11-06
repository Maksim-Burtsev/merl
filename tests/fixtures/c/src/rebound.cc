struct Gourd {
  int slosh() { return 1; }
};
struct Terrine {
  int slosh() { return 2; }
};
Terrine* next_terrine();
void block(Gourd g) { { Terrine g; g.slosh(); } g.slosh(); }
//                                   ^ d: src/rebound.cc:5
//                                                ^ d: src/rebound.cc:2
void before(Gourd g) { g.slosh(); { Terrine g; g.slosh(); } }
//                       ^ d: src/rebound.cc:2
//                                               ^ d: src/rebound.cc:5
void range(Gourd g, Terrine* ts) { for (Terrine& g : *ts) g.slosh(); g.slosh(); }
//                                                          ^ d: src/rebound.cc:5
//                                                                     ^ d: src/rebound.cc:2
void lambda(Gourd g) { auto l = [](Terrine g) { return g.slosh(); }; g.slosh(); }
//                                                       ^ d: src/rebound.cc:5
//                                                                     ^ d: src/rebound.cc:2
void later(Gourd g) { g.slosh(); auto l = [](Terrine g) { return g.slosh(); }; }
//                      ^ d: src/rebound.cc:2
void init(Gourd g) { if (auto g = Terrine(); g.slosh()) {} g.slosh(); }
//                                             ^ d: src/rebound.cc:5
//                                                           ^ d: src/rebound.cc:2
void cond(Gourd g) { if (auto* g = next_terrine()) g->slosh(); g.slosh(); }
//                                                    ^ d: picker src/rebound.cc:2, src/rebound.cc:5
//                                                               ^ d: src/rebound.cc:2
void caught(Gourd g) { try {} catch (Terrine& g) { g.slosh(); } g.slosh(); }
//                                                   ^ d: src/rebound.cc:5
//                                                                ^ d: src/rebound.cc:2
void bare(Gourd g) { for (Terrine g; ; ) g.slosh(); }
//                                         ^ d: src/rebound.cc:2; want src/rebound.cc:5 (#1387)
void bare_if(Gourd g) { if (Terrine g; true) g.slosh(); }
//                                             ^ d: src/rebound.cc:2; want src/rebound.cc:5 (#1387)
void reads(Gourd g, int i) { for (i = 0; i < g.slosh(); ++i) g.slosh(); }
//                                             ^ d: src/rebound.cc:2
//                                                             ^ d: src/rebound.cc:2
void closed(Gourd g) { { Terrine g; } g.slosh(); }
//                                      ^ d: src/rebound.cc:2
void noted(Gourd g) { /* { Terrine g; */ g.slosh(); }
//                                         ^ d: src/rebound.cc:2
int quoted(const char*, int);
void quotes(Gourd g) { quoted("{ Terrine g;", g.slosh()); }
//                                              ^ d: src/rebound.cc:2
