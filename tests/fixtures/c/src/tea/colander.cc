#include "tea/colander.hh"

namespace drainer {

void Colander::Drain() {}

void Spoon::Drain() {}

void Colander::Tilt(int degrees) {}

void Colander::Tilt(const char* how) {}

#ifdef FINE
int Colander::Pour(int cups) { return cups; }
#else
int Colander::Pour(int cups) { return 2 * cups; }
#endif

struct Colander::Funnel {
  int holes;
};

Colander::Funnel* Colander::Fit() {
  Funnel* f = nullptr;
//^ d: src/tea/colander.cc:19
  return f;
}

int Width(Funnel* f) { return f->Width(); }
//        ^ d: include/tea/colander.hh:27

int Brim() {
  using soak::Colander;
  Colander* c = nullptr;
//^ d: !jump
  return 0;
}

}
