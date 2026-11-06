#pragma once

namespace drainer {

class Colander {
 public:
  void Drain();
//     ^ d: src/tea/colander.cc:5
  void Tilt(int degrees);
//     ^ d: src/tea/colander.cc:9
  void Tilt(const char* how);
//     ^ d: src/tea/colander.cc:11
  int Pour(int cups);
//    ^ d: !jump

 private:
  struct Funnel;
  Funnel* Fit();
};

class Spoon {
 public:
  void Drain();
//     ^ d: src/tea/colander.cc:7
};

class Funnel {
 public:
  int Width();
};

}

namespace soak {

class Colander {
 public:
  void Drain() {}
};

}
