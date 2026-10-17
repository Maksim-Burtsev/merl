#include "tea/shelf.hh"

int fill() {
  namespace shelf = other;
  shelf::Pot p;
  //     ^ d: include/tea/shelf.hh:22
  return p.size;
}

int pour() {
  shelf::Pot q;
  //     ^ d: include/tea/shelf.hh:4
  return q.size;
}

int shelf::count() {
  struct Pot {
    int size;
  };
  return Pot{}.size + std::hash<int>{}(1);
  //                       ^ d: !jump
}

int stack() {
  shelf::Impl i;
  //     ^ d: !jump
  return 0;
}
