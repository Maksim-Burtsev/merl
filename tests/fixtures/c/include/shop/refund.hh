#pragma once

namespace shop {

class Refund {
 public:
  Refund();
  explicit Refund(int cents) : cents_(cents) {}
  static Refund Partial(int cents) { return Refund(cents); }
  bool ok() const { return true; }

 private:
  int cents_;
};

struct Aisle {
  virtual void Partial(int cents) {}
};

class Drawer {
 public:
  class Slot;
  struct Scanner;
  int count();
};

namespace depot {
class Aisle {
  int row;
};
}  // namespace depot

}  // namespace shop
