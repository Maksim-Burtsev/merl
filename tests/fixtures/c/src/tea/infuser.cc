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
