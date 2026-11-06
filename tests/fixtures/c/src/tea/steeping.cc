#include "tea/steep.hh"

#include <deque>
#include <span>

namespace taste {

static Stove Pour(int ml) { return Stove{}; }

struct Tray {
  std::vector<Mug*> cups;
};

int steeped(std::vector<Stove*> stoves, const std::vector<Mug>& mugs, std::array<Stove*, 2> two) {
  int total = brew::Steep(80).sip() + brew::Pour(80).sip();
  //                          ^ d: include/tea/cup.hh:20
  //                                                 ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  total += stoves[0]->sip() + mugs[1].sip() + two[0]->sip();
  //                  ^ d: include/tea/cup.hh:29
  //                                  ^ d: include/tea/cup.hh:20
  //                                                  ^ d: include/tea/cup.hh:29
  std::vector<Pot*> pots;
  total += pots[0]->sip();
  //                ^ d: include/tea/cup.hh:20
  auto* first = stoves[1];
  total += first->sip();
  //              ^ d: include/tea/cup.hh:29
  {
    std::vector<Mug*> stoves;
    total += stoves[0]->sip();
    //                  ^ d: include/tea/cup.hh:20
  }
  return total;
}

int unread(std::vector<Leaf>* many, std::map<int, Mug*> named, Tray* p) {
  std::vector<Leaf> rows[2];
  int total = rows[0].size() + many[0].size();
  //                  ^ d: picker include/tea/steep.hh:12, include/tea/steep.hh:16, src/rnd.cc:6
  //                                    ^ d: picker include/tea/steep.hh:12, include/tea/steep.hh:16, src/rnd.cc:6
  return total + named[0]->sip() + p->cups[0]->sip();
  //                       ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  //                                           ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
}

template <class T>
int dependent(std::vector<T*> pots) {
  return pots[0]->sip();
  //              ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
}

int later(const std::vector<Mug>& mugs, std::deque<Stove*> queue, std::span<Mug> view) {
  int total = queue[0]->sip() + view[0].sip();
  //                    ^ d: include/tea/cup.hh:29
  //                                    ^ d: include/tea/cup.hh:20
  return total + mugs[0].sip() + [](std::vector<Stove> mugs) { return mugs[0].sip(); }({});
  //                     ^ d: include/tea/cup.hh:20
  //                                                                          ^ d: include/tea/cup.hh:29
}

int hidden(std::vector<Mug> mugs, Stove* s) {
  int total = 0;
  {
    decltype(s) mugs = s;
    total += mugs[0].sip();
    //               ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  }
  {
    typeof(s) mugs = s;
    total += mugs[0].sip();
    //               ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  }
  {
    alignas(8) Stove* mugs = s;
    total += mugs[0].sip();
    //               ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  }
  return total;
}

struct Lid {
  int rim;
};

struct Grip {
  Lid* operator->();
  int rim;
};

using Knob = Grip;

int handled(std::vector<Grip> grips, std::vector<Knob> knobs, std::vector<Lid*> lids) {
  return grips[0]->rim + knobs[0]->rim + grips[1].rim + lids[0]->rim;
  //               ^ d: picker src/tea/steeping.cc:82, src/tea/steeping.cc:87
  //                               ^ d: picker src/tea/steeping.cc:82, src/tea/steeping.cc:87
  //                                              ^ d: src/tea/steeping.cc:87
  //                                                             ^ d: src/tea/steeping.cc:82
}

}  // namespace taste
