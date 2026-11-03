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

struct Dial {
  int Read();
};

struct Gauge {
  int Read();
};

Dial* Reading();

class Kettle {
 public:
  int Boil();

 private:
  Gauge* Reading();
};

template <class T>
struct Brazier {
  Gauge* Reading();
};

template <class T>
struct Hob : Brazier<T> {
  int Simmer() { return Reading()->Read(); }
//                                 ^ d: include/tea/trivet.hh:42
};

class Cauldron {
 public:
  int Stir();

 private:
  Gauge* Reading(int);
};

}
