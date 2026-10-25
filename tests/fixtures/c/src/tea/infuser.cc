struct Canister {
  explicit Canister(int grams) : grams(grams) {}
  //       ^ d: src/tea/infuser.cc:2
  int grams;
};

class Infuser {
 public:
  Infuser() : can_(nullptr) {}
  //^ d: src/tea/infuser.cc:9
  ~Infuser() { delete can_; }
  // ^ d: src/tea/infuser.cc:11
  Infuser* Copy() { return new Infuser(); }
  //                           ^ d: src/tea/infuser.cc:7

 private:
  Canister* can_;
};

struct Alarm {
  Alarm(void (*ring)(void*), void* arg) : ring(ring), arg(arg) {}
  //^ d: src/tea/infuser.cc:21
  void (*const ring)(void*);
  void* const arg;
};

Infuser* fill() {
  Infuser* i = new Infuser();
  //^ d: src/tea/infuser.cc:7
  return i;
}

struct Steeper {
  explicit Steeper(int ml) : ml(ml) {}
  Steeper() : Steeper(500) {}
  //          ^ d: src/tea/infuser.cc:34
  //^ d: src/tea/infuser.cc:35
  int ml;
};

struct Key {
  Key();
  Key(int a, int b);
  Key(const char* name);
  Key(int a, const char* name = "k");
};

Key::Key() {}
//   ^ d: src/tea/infuser.cc:42
Key::Key(int a, int b) {}
//   ^ d: src/tea/infuser.cc:43
Key::Key(const char* name) {}
//   ^ d: src/tea/infuser.cc:44
Key::Key(int a, const char* name) {}
//   ^ d: src/tea/infuser.cc:45

struct Ladle {
  Ladle(int ml, int cups) : ml(ml) {}
  Ladle(int ml) : Ladle(ml, 1) {}
  //              ^ d: src/tea/infuser.cc:58
  Ladle(const char* ml) : Ladle(ml[0]) {}
  //                      ^ d: src/tea/infuser.cc:57
  Ladle() : Ladle(250) {}
  //        ^ d: src/tea/infuser.cc:57
  int ml;
};

struct Tray {
  explicit Tray(int n) : n(n) {}
  Tray(const Tray& t, int extra) : Tray(t) {}
  //                               ^ d: !jump
  int n;
};

typedef int Gills;

struct Taper {
  Taper(Gills n);
  Taper(const char* name);
};

Taper::Taper(int n) {}
//     ^ d: src/tea/infuser.cc:84; want src/tea/infuser.cc:78 (#1248)
Taper::Taper(const char* name) {}

struct Nozzle {
  Nozzle(int x);
  Nozzle() : Nozzle(1) {}
  //         ^ d: src/tea/infuser.cc:92
};

Nozzle::Nozzle(int x) {}

struct Ewer {
  Ewer(int x);
  Ewer() : Ewer(1) {}
  //       ^ d: src/tea/infuser.cc:95
};

namespace pot {
struct Wisp {
  Wisp(int a, int b) {}
};
}

struct Wisp {
  Wisp(int a, int b) : Wisp(1, 2) {}
  //                   ^ d: picker src/tea/infuser.cc:101, src/tea/infuser.cc:106
};

struct Pail {
  explicit Pail(int a) {}
  explicit Pail(const char* s, int n = 0) {}
  Pail() : Pail("x") {}
  //       ^ d: picker src/tea/infuser.cc:111, src/tea/infuser.cc:112, src/tea/infuser.cc:113
};

namespace pot {
struct Gourd {
  Gourd(int a, int b) {}
};
}

struct Gourd {
  Gourd(int a, int b) : n(a) {}
  Gourd() : Gourd(std::max<int>(1, 2), 1) {}
  //        ^ d: picker src/tea/infuser.cc:119, src/tea/infuser.cc:124
  int n;
};

struct Spout {
  explicit Spout(int ml);
  Spout();
};

Spout::Spout(int ml) {}
Spout::Spout() : Spout(500) {}
//               ^ d: src/tea/infuser.cc:136

struct Cruet {
  Cruet(int x);
  Cruet(const Cruet& u, int extra);
  Cruet();
};

Cruet::Cruet() : Cruet(1) {}
//                 ^ d: src/tea/infuser.cc:141
Cruet::Cruet(const Cruet& u, int extra) : Cruet(u) {}
//                                        ^ d: !jump

struct Flask {
  Flask(int a);
  Flask(const char* s);
  Flask();
};

Flask::Flask() : Flask(250) {}
//               ^ d: !jump
Flask::Flask(int a) : Flask("x") {}
//                    ^ d: !jump

struct Beaker {
  Beaker(int a);
  Beaker();
};

namespace pot {
struct Beaker {
  Beaker(const char* s);
  Beaker(int a, int b);
  Beaker();
};
}

pot::Beaker::Beaker() : Beaker("x") {}
//                      ^ d: src/tea/infuser.cc:169
namespace pot {
Beaker::Beaker(int a, int b) : Beaker() {}
//                             ^ d: src/tea/infuser.cc:175
}
