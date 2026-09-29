#include "shop/file.hh"

struct Spool {
  Spool(int n);
  int reels_;
  int spare_{0};
};

Spool::Spool(int n)
    : reels_(n),
    //^ d: src/spool.cc:5
      spare_{n} {}
    //^ d: src/spool.cc:6
