struct Random {
  explicit Random(int s) {}
  int Next() { return 1; }
};
struct Slice {
  int size() const { return 0; }
};
int Draw() {
  Random rnd(301);
  return rnd.Next();
  //     ^ d: src/rnd.cc:9
}
int Twice(int seed) {
  auto twice = [](int seed) {
    return seed * 2;
    //     ^ d: src/rnd.cc:14
  };
  Slice result;
  const Slice& view = result;
  //                  ^ d: src/rnd.cc:18
  return twice(seed) + view.size();
  //           ^ d: src/rnd.cc:13
  //                   ^ d: src/rnd.cc:19
}
int Once(int seed) { return [](int seed) { return seed; }(seed); }
//                                                ^ d: src/rnd.cc:25
//                                                        ^ d: src/rnd.cc:25
