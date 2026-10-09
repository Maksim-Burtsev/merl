struct Rim {
  int girth;
};

struct Hoop {
  int girth;
};

struct Stave {
  Rim hoop;
};

struct Cask {
  Stave& operator[](int i);
  Hoop hoop;
};

int girth_of(Cask& c) { return c[0].hoop.girth; }
//                                       ^ d: !jump

class Vat {
 public:
  virtual int brew(int n) = 0;
};

int go() { return make_vat()->brew(1); }
//                            ^ d: src/cask.cc:23

struct Spigot {
  int tap_;
};

class Keg {
  Keg() : tap_(1) {}
//        ^ d: src/cask.cc:30
};

using Barrel = Cask;

int tapped() { return Barrel.hoop.girth; }
//                    ^ d: none
