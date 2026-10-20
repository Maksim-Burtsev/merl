#pragma once

#include <string>
#include <vector>

namespace taste {

class Cup {
 public:
  virtual ~Cup() = default;
  virtual int sip() const { return 1; }
  int append(int ml) { return ml; }
  int push_back(int ml) { return ml; }
  Cup* refill() { return this; }
  int level;
};

class Mug : public Cup {
 public:
  int sip() const override { return 2; }
  int handle() const { return sip() + Cup::sip() + level; }
  //                          ^ d: include/tea/cup.hh:20
  //                                       ^ d: include/tea/cup.hh:11
  //                                                 ^ d: include/tea/cup.hh:15
};

class Stove {
 public:
  int sip() const { return 3; }
  int warm(const Cup& cup) const { return cup.level; }
};

class Maker {
 public:
  Maker& leaf(const std::string& name) { return *this; }
  Maker& water(int ml) { return *this; }
  Mug build() const { return Mug{}; }
};

Mug pour_mug();
Stove* stove_at(int n);

using Pot = Mug;
typedef Stove Range;

}  // namespace taste
