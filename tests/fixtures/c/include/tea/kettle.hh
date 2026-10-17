#pragma once

namespace infuse {

struct Leaf {
  int dose;
};

int steep(Leaf leaf);

class Kettle {
 public:
  explicit Kettle(int spout);
  int decant(int cups) const;
  int decant(const char* cups) const;
  int topup();
  virtual int rinse() const { return 1; }

 protected:
  int spout_;
};

class Teapot {
 public:
  int decant(int cups) const { return cups; }
};

class Urn : public Kettle {
 public:
  Urn() : Kettle(2) {}
  int serve() const { return decant(1) + spout_; }
  //                         ^ d: src/tea/kettle.cc:23
  //                                     ^ d: include/tea/kettle.hh:20
  int rinse() const override { return Kettle::rinse() + 1; }
  //                                          ^ d: include/tea/kettle.hh:17
};

}  // namespace infuse

namespace stale {
class Kettle {
 public:
  int decant(int cups) const { return cups * 2; }
};
int steep(int dose);
}  // namespace stale

int steep(int dose);

template <typename T>
struct Tin {
  T seal() const { return T{}; }
};

namespace infuse {
class Samovar {
 public:
  explicit Samovar(int spout);
  int spout;
};

class Brewer {
 public:
  explicit Brewer(int spout) : spout(spout) {}
  //                           ^ d: include/tea/kettle.hh:67
  //                                 ^ d: include/tea/kettle.hh:64
  int spout;
};
}  // namespace infuse
