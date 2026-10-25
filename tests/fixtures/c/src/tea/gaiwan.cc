struct Gaiwan : public ext::Gaiwan {
  Gaiwan(int a);
  Gaiwan(int a, int b);
};

Gaiwan::Gaiwan(int a) : ext::Gaiwan(a, 0) {}
//                           ^ d: src/tea/gaiwan.cc:1; want none (#1248)

struct Kyusu : public ext::Kyusu {
  Kyusu(int a) : ext::Kyusu(a, 0) {}
  //                  ^ d: src/tea/gaiwan.cc:9; want none (#1248)
  Kyusu(int a, int b);
};

template <class T>
struct Tureen {
  Tureen(int a);
  Tureen() : Tureen(1) {}
  //         ^ d: src/tea/gaiwan.cc:17
};

template <class T>
Tureen<T>::Tureen(int a) {}
