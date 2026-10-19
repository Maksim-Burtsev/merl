#pragma once
// #1050: a pure virtual and its one override, in another class, are no counterparts.
namespace infuse {
class Dripper {
 public:
  virtual int Drip(int drops) = 0;
  //          ^ d: picker src/tea/dripper.cc:6
};
}  // namespace infuse
