#include "tea/brew.hh"

int pour(int cups);

int pour(int cups) { return cups; }

int serve() { return pour(6); }
//                   ^ d: src/tea/brew_line.cc:5

int twice(int pour) { return pour + pour; }
//                           ^ d: src/tea/brew_line.cc:10

int more() { return infuse_more() + helper(); }
//                  ^ d: picker include/tea/brew.hh:3, include/tea/brew_hidden.hh:4
//                                  ^ d: include/tea/brew_impl.hh:3

struct Cmp {
  bool operator()(int pour) const;
};

bool Cmp::operator()(int pour) const { return pour > 0; }
//                                            ^ d: picker src/tea/brew_line.cc:3, src/tea/brew_line.cc:5, src/tea/shelf.cc:10; want src/tea/brew_line.cc:21 (#1029)
