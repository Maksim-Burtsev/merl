#pragma once

namespace shelf {
class Pot {
 public:
  int size;
};
struct hash {
  int seed;
};
int count();
class Widget {
  class Impl;
};
class Widget::Impl {
 public:
  int depth;
};
}  // namespace shelf

namespace other {
class Pot {
 public:
  int size;
};
}  // namespace other

namespace std {
template <>
struct hash<shelf::Pot> {
  int operator()(const shelf::Pot& p) const { return p.size; }
};
}  // namespace std
