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
