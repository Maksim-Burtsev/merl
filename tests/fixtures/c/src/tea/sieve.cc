int drain(int cups) { return cups; }

struct Sieve {
  bool operator()(int drain) const;
  bool operator==(int drain) const { return drain == 0; }
//                                          ^ d: src/tea/sieve.cc:5
  int operator[](int cups) const { return drain(cups); }
//                                        ^ d: src/tea/sieve.cc:1
};

bool Sieve::operator()(int drain) const { return drain > 0; }
//                                               ^ d: src/tea/sieve.cc:11

bool operator<(const Sieve &a, int cups) { return drain(cups) < 0; }
//                                                ^ d: src/tea/sieve.cc:1

int holes;

struct Mesh {
  int holes;
  bool operator==(int x) const { return holes == x; }
//                                      ^ d: src/tea/sieve.cc:20
  bool operator()(int x) const;
};

bool Mesh::operator()(int x) const { return holes > x; }
//                                          ^ d: src/tea/sieve.cc:20

namespace sift {
int strain(int n) { return n; }
}

namespace mesh {
int strain(int n) { return n; }
bool operator<(const Mesh &m, int n) { return strain(n) < 0; }
//                                            ^ d: picker src/tea/sieve.cc:30, src/tea/sieve.cc:34
}
