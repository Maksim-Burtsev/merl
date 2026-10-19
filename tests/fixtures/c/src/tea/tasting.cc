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
  //         ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:20 (#9995)
  //                                ^ d: include/tea/cup.hh:35
  //                                                      ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:20 (#9995)
  //                                                                                  ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:20 (#9995)
}

int arrow() {
  taste::Stove* s = taste::stove_at(1);
  return s->sip() + taste::stove_at(2)->sip();
  //        ^ d: include/tea/cup.hh:29
  //                                    ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:29 (#9995)
}

int shadows(std::vector<taste::Mug>& mugs, taste::Cup* cup) {
  int total = 0;
  for (const auto& cup : mugs) {
    total += cup.sip();
    //           ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:20 (#9996)
  }
  for (taste::Stove cup : std::vector<taste::Stove>{}) total += cup.sip();
  //                                                                ^ d: include/tea/cup.hh:29
  cup = cup->refill();
  //         ^ d: include/tea/cup.hh:14
  if (auto* mug = dynamic_cast<taste::Mug*>(cup)) total += mug->sip();
  //                                                             ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:20 (#9996)
  return total + cup->sip();
  //                  ^ d: include/tea/cup.hh:11
}

int binds() {
  auto [mug, stove] = std::pair<taste::Mug, taste::Stove>{};
  return mug.sip() + stove.sip();
  //         ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:20 (#9996)
  //                       ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:29 (#9996)
}

int aliases() {
  taste::Pot pot;
  //     ^ d: picker include/tea/cup.hh:43, src/tea/shelf.cc:17; want include/tea/cup.hh:43 (#9997)
  taste::Range range;
  //     ^ d: include/tea/cup.hh:44
  return pot.sip() + range.sip() + pot.level;
  //         ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:20 (#9997)
  //                       ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:29 (#9997)
  //                                   ^ d: picker include/tea/cup.hh:15, src/level.cc:7; want include/tea/cup.hh:15 (#9997)
}

int lambdas() {
  taste::Mug cup;
  auto f = [](const taste::Stove& cup) { return cup.sip(); };
  //                                                ^ d: include/tea/cup.hh:29
  auto g = [cup]() { return cup.sip(); };
  //                            ^ d: include/tea/cup.hh:20
  auto h = [cup = taste::Stove{}]() { return cup.sip(); };
  //                                             ^ d: picker include/tea/cup.hh:11, include/tea/cup.hh:20, include/tea/cup.hh:29; want include/tea/cup.hh:29 (#9996)
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
