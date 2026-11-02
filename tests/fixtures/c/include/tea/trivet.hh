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

 public:
  void Place(Stand* s);
};

template <typename K>
class Coaster {
 public:
  void Lay();

 private:
  struct Stand {
    K top;
  };

 public:
  void Hold(Stand* s);
};

class Doily {
 public:
  void Fold();
  int Spread(Stand* s);
};

}
