namespace matcha {
struct Chawan {
  Chawan(int a);
  Chawan() : Chawan(1) {}
  //         ^ d: src/tea/chawan.cc:22
};
}

namespace sado {
struct Bowl {
  Bowl(int a);
  Bowl() : Bowl(1) {}
  //       ^ d: src/tea/chawan.cc:11
};
}

struct Bowl {
  Bowl(int a);
};

using namespace matcha;
Chawan::Chawan(int a) {}
Bowl::Bowl(int a) {}
