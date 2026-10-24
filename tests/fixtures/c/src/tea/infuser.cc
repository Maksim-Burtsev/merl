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
  //          ^ d: picker src/tea/infuser.cc:33, src/tea/infuser.cc:34
  //^ d: src/tea/infuser.cc:35
  int ml;
};
