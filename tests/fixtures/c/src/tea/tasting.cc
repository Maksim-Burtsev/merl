#include "tea/cup.hh"

#include <string>
#include <utility>
#include <vector>

namespace taste {

Mug pour_mug() { return Mug{}; }

Stove* stove_at(int n) { return new Stove; }

}  // namespace taste

/*
int Cup::sip() const {
class Stove {
*/

static const char* menu = "int taste::Mug::sip() const { return 0; }";

#define SIP_TWICE(c) ((c).sip() + (c).sip())

int outside() {
  std::string name = "green";
  std::vector<int> mls;
  mls.push_back(1);
  //  ^ d: none
  name.append("!");
  //   ^ d: none
  return static_cast<int>(name.size() + mls.size());
  //                           ^ d: none
}

int chain() {
  taste::Maker maker;
  auto mug = maker.leaf("green")
                 .water(80)
                 .build();
  //              ^ d: include/tea/cup.hh:37
  return mug.sip() + taste::Maker{}.leaf("black").build().sip() + taste::pour_mug().sip();
  //         ^ d: include/tea/cup.hh:20
  //                                ^ d: include/tea/cup.hh:35
  //                                                      ^ d: include/tea/cup.hh:20
  //                                                                                  ^ d: include/tea/cup.hh:20
}

int arrow() {
  taste::Stove* s = taste::stove_at(1);
  return s->sip() + taste::stove_at(2)->sip();
  //        ^ d: include/tea/cup.hh:29
  //                                    ^ d: include/tea/cup.hh:29
}

int shadows(std::vector<taste::Mug>& mugs, taste::Cup* cup) {
  int total = 0;
  for (const auto& cup : mugs) {
    total += cup.sip();
    //           ^ d: include/tea/cup.hh:20
  }
  for (taste::Stove cup : std::vector<taste::Stove>{}) total += cup.sip();
  //                                                                ^ d: include/tea/cup.hh:29
  cup = cup->refill();
  //         ^ d: include/tea/cup.hh:14
  if (auto* mug = dynamic_cast<taste::Mug*>(cup)) total += mug->sip();
  //                                                             ^ d: include/tea/cup.hh:20
  return total + cup->sip();
  //                  ^ d: include/tea/cup.hh:11
}

int binds() {
  auto [mug, stove] = std::pair<taste::Mug, taste::Stove>{};
  return mug.sip() + stove.sip();
  //         ^ d: include/tea/cup.hh:20
  //                       ^ d: include/tea/cup.hh:29
}

int aliases() {
  taste::Pot pot;
  //     ^ d: include/tea/cup.hh:43
  taste::Range range;
  //     ^ d: include/tea/cup.hh:44
  return pot.sip() + range.sip() + pot.level;
  //         ^ d: include/tea/cup.hh:20
  //                       ^ d: include/tea/cup.hh:29
  //                                   ^ d: include/tea/cup.hh:15
}

int lambdas() {
  taste::Mug cup;
  auto f = [](const taste::Stove& cup) { return cup.sip(); };
  //                                                ^ d: include/tea/cup.hh:29
  auto g = [cup]() { return cup.sip(); };
  //                            ^ d: include/tea/cup.hh:20
  auto h = [cup = taste::Stove{}]() { return cup.sip(); };
  //                                             ^ d: include/tea/cup.hh:29
  //                                         ^ d: src/tea/tasting.cc:95
  return f(taste::Stove{}) + g() + h();
}

int warmth() {
  using namespace taste;
  Stove stove;
  Mug mug;
  //^ d: include/tea/cup.hh:18
  return stove.warm(mug) + mug.handle() + SIP_TWICE(stove) + menu[0];
  //           ^ d: include/tea/cup.hh:30
  //                           ^ d: include/tea/cup.hh:21
  //                                      ^ d: src/tea/tasting.cc:22
}

int corners(std::map<int, taste::Mug>& cups, std::vector<taste::Stove>& stoves, taste::Cup* cup) {
  int total = 0;
  for (auto& kv : cups) total += kv.sip();
  //                                ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  for (auto stove : stoves) total += stove.sip();
  //                                       ^ d: include/tea/cup.hh:29
  {
    std::vector<taste::Mug> stoves;
    for (const auto& s : stoves) total += s.sip();
    //                                      ^ d: include/tea/cup.hh:20
  }
  auto* m = dynamic_cast<taste::Mug*>(cup)->refill();
  total += m->sip();
  //          ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  auto [a, b] = std::make_pair(taste::Mug{}, taste::Stove{});
  total += a.sip();
  //         ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  auto [c, d, e] = std::pair<taste::Mug, taste::Stove>{};
  total += c.sip();
  //         ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  auto k = [one = taste::Mug{}, two = taste::Stove{}.sip()]() { return one.sip() + two; };
  //                                                                       ^ d: include/tea/cup.hh:20
  return total + k();
}

struct Percolator {
  taste::Mug siphon(int ml);
  taste::Stove siphon();
  taste::Maker maker;
};

namespace east { taste::Mug percolate(); }
namespace west { taste::Stove percolate(); }

int calls(Percolator perc, int n) {
  int total = perc.siphon().sip() + Percolator{}.siphon(1).sip();
  //                        ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  //                                                       ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  total += east::percolate().sip();
  //                         ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  total += perc.maker.build().sip();
  //                          ^ d: include/tea/cup.hh:20
  auto made = perc.maker.build();
  auto sum = n + perc.maker.build();
  {
    taste::Stove perc;
    total += made.sip() + sum.sip();
    //            ^ d: include/tea/cup.hh:20
    //                        ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  }
  return total;
}

int poured() {
  using namespace taste;
  Pot pot;
  //^ d: include/tea/cup.hh:43
  std::vector<int> v;
  using Cups = std::vector<int>;
  Cups cups;
  return pot.sip() + cups.size();
  //         ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29
  //                      ^ d: src/rnd.cc:6; want none (#1144)
}
