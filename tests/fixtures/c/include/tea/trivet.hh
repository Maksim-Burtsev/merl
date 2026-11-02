#pragma once

namespace pantry {

class Stand {
 public:
  int legs;
};

class Trivet {
 public:
  void Set();

 protected:
  typedef int Stand;
};

template <typename K>
class Coaster {
 public:
  void Lay();

 private:
  struct Stand {
    K top;
  };
};

class Doily {
 public:
  void Fold();
  int Spread(Stand* s);
};

}
