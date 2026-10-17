int pour(int cups) { return cups; }

struct Sieve {
  bool operator()(int pour) const;
  bool operator==(int pour) const { return pour == 0; }
//                                         ^ d: src/tea/sieve.cc:5
  int operator[](int cups) const { return pour(cups); }
//                                        ^ d: picker src/tea/sieve.cc:1, src/tea/shelf.cc:10
};

bool Sieve::operator()(int pour) const { return pour > 0; }
//                                              ^ d: src/tea/sieve.cc:11

bool operator<(const Sieve &a, int cups) { return pour(cups) < 0; }
//                                                ^ d: picker src/tea/sieve.cc:1, src/tea/shelf.cc:10
